//! フロントマター(sml-spec.md §2.1、D12)のパース。
//!
//! ファイル先頭(オフセット0)が `---` 単独行で始まる場合のみ有効。次の `---` 単独行
//! までを YAML 風の `key: value` 行として読む(ネスト・リスト・引用符エスケープは
//! 持たない自前の最小実装)。ファイル途中に現れる `---` 単独行は通常の行として
//! `scan.rs` 側で処理される(段落フォールバック)— ここではオフセット0だけを見る。
//!
//! 層Aの一部として `parse`(lib.rs)から最初に呼ばれ、フロントマターの直後の
//! オフセットを `scan::scan_from` に渡すことで、残りのブロック列を通常どおり
//! 走査させる。

use crate::ast::{Frontmatter, RefTarget};
use crate::block::parse_ref_target;
use crate::error::{Diag, DiagKind};
use crate::scan::{split_lines_range, PhysLine};
use crate::span::Span;

/// `src` の先頭を調べる。フロントマターがあれば `(Some(frontmatter), 本文開始オフセット)`
/// を、無ければ `(None, 0)` を返す。
pub(crate) fn parse_frontmatter(src: &str, diags: &mut Vec<Diag>) -> (Option<Frontmatter>, usize) {
    let lines = split_lines_range(src, Span::new(0, src.len()));
    let Some(open) = lines.first() else { return (None, 0) };
    if open.content.slice(src).trim() != "---" {
        return (None, 0);
    }
    let open_span = open.content;

    let close_idx = lines.iter().enumerate().skip(1).find_map(|(i, line)| {
        (line.content.slice(src).trim() == "---").then_some(i)
    });

    let Some(close_idx) = close_idx else {
        // 閉じが無い: ファイル末尾まで飲み込み、best-effort で id/title/alias を拾いつつ
        // UnclosedFrontmatter を報告する(scan.rs の UnclosedFence と同じ方針)。
        let last = lines.len() - 1;
        let (id, title, alias) = parse_body(src, &lines[1..], diags);
        diags.push(Diag::new(
            DiagKind::UnclosedFrontmatter,
            open_span,
            "フロントマターが閉じられていません(`---` 単独行が見つかりません)",
        ));
        let end = lines[last].full.end;
        let fm = Frontmatter {
            span: Span::new(0, end),
            open_span,
            id,
            title,
            alias,
            close_span: Span::new(end, end),
        };
        return (Some(fm), end);
    };

    let (id, title, alias) = parse_body(src, &lines[1..close_idx], diags);
    let close_span = lines[close_idx].content;
    let body_start = lines[close_idx].full.end;
    let fm = Frontmatter { span: Span::new(0, body_start), open_span, id, title, alias, close_span };
    (Some(fm), body_start)
}

/// 開き `---` と閉じ `---` の間の行を `key: value` として解釈する(コロン後の空白は
/// 任意)。空行は無視する。キーが `id` / `title` / `alias` 以外なら
/// `UnknownFrontmatterKey`(D41、v0 の許可キーは3つ)。`id` の値が ULID でなければ
/// `BadIdValue`(sml-spec §2.1)。`alias` の値が key 字句(`[A-Za-z0-9_-]+`)に
/// 違反すれば `BadKeyCharset`。同一キーが複数回現れたら `DuplicateFrontmatterKey`
/// (D17、`Warning`)。挙動は従来どおり後勝ち(最後の出現が採用される)のまま変えない。
/// `parse_body` の戻り値: `(id, title, alias)`。
type FrontmatterBody = (Option<(RefTarget, Span)>, Option<String>, Option<(String, Span)>);

fn parse_body(src: &str, lines: &[PhysLine], diags: &mut Vec<Diag>) -> FrontmatterBody {
    let mut id = None;
    let mut title = None;
    let mut alias = None;
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for line in lines {
        let text = line.content.slice(src);
        if text.trim().is_empty() {
            continue;
        }
        let trim_start_len = text.len() - text.trim_start().len();
        let content = text.trim_start();
        let content_start = line.content.start + trim_start_len;

        let (key, value, value_span) = match content.find(':') {
            Some(idx) => {
                let key = content[..idx].trim();
                let value_part = &content[idx + 1..];
                let value_ws = value_part.len() - value_part.trim_start().len();
                let value = value_part.trim();
                let value_start = content_start + idx + 1 + value_ws;
                (key, value, Span::new(value_start, value_start + value.len()))
            }
            None => {
                let key = content.trim();
                (key, "", Span::new(content_start, content_start + key.len()))
            }
        };

        if !seen.insert(key.to_string()) {
            diags.push(Diag::new(
                DiagKind::DuplicateFrontmatterKey,
                line.content,
                format!("フロントマターキー '{key}' が複数回宣言されています(最後の宣言を採用します)"),
            ));
        }

        match key {
            "id" => {
                let target = parse_ref_target(value);
                if !matches!(target, RefTarget::Ulid(_)) {
                    diags.push(Diag::new(
                        DiagKind::BadIdValue,
                        value_span,
                        format!("フロントマターの id は ULID のみです(got: '{value}')"),
                    ));
                }
                id = Some((target, value_span));
            }
            "title" => {
                title = Some(value.to_string());
            }
            "alias" => {
                if !is_valid_key_charset(value) {
                    diags.push(Diag::new(
                        DiagKind::BadKeyCharset,
                        value_span,
                        format!("フロントマターの alias '{value}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
                    ));
                }
                alias = Some((value.to_string(), value_span));
            }
            _ => {
                diags.push(Diag::new(
                    DiagKind::UnknownFrontmatterKey,
                    line.content,
                    format!("未知のフロントマターキー '{key}' です(v0 は id / title / alias のみ)"),
                ));
            }
        }
    }

    (id, title, alias)
}

/// key 字句(`[A-Za-z0-9_-]+`)の検証(D5)。block.rs / inline.rs にも同名の
/// private ヘルパがある(既存の意図的な小重複の流儀 — このモジュールだけが
/// 依存する共有 crate を新設するほどではないため)。
fn is_valid_key_charset(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}
