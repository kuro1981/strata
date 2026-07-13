//! 層B: インラインパース(sml-parser-design.md §3、sml-spec.md §5)。
//!
//! 手書きの再帰下降 + 素朴なバイトスキャン。ブロック(層A/層Bのブロック側)は厳格だが、
//! インラインは寛容(design.md §3): 未対応・不正な構文は診断を出さずに(あるいは
//! 診断は出しつつ)プレーンテキストへフォールバックし、パース全体を止めない。
//!
//! 実装する構文(sml-spec §5):
//!   - `**strong**` / `*em*` / `` `code` `` → `SmlInline::Emph`(`code` の中はネスト不可)
//!   - `$...$` → `SmlInline::MathTex(Span)`。**内側 TeX のスパンのみ**を記録し、
//!     中身はパースしない(tex2math は build の仕事)
//!   - `[text](scheme:target)` の参照5スキーム(`ref` `table` `fig` `math` `term` `cell`)
//!     → `SmlInline::Ref` / `SmlInline::TermRef`。`cell:target#path|path` は `CellCoord` へ
//!
//! スキーム別の target 字句規則:
//!   - `ref` / `table` / `fig` / `math` / `cell` の target: ULID(26字 Crockford)なら
//!     `RefTarget::Ulid`、そうでなければ `RefTarget::Label` だが `[A-Za-z0-9_-]+` の
//!     字句検証を行い、違反すれば `BadKeyCharset` を積む(ノード自体は構築する。
//!     block.rs の alias 検証と同じ「診断は積むが止めない」方針)
//!   - `term` の target のみ字句制限の対象外(日本語等の任意の非空文字列を許す)
//!   - `cell` の座標 path の各 key が字句違反なら `BadCellCoord` を積む(ノードは構築する)
//!   - スキーム語がこの5+1種のいずれでもなければ `UnknownScheme` を積み、
//!     **ノードは構築せずテキストへフォールバック**(sml-spec に無いスキームなので
//!     未解決のまま残すより「読めるテキスト」に倒す)
//!   - dest が `://` を含む(例 `https://...`)場合は外部リンクの記法が v0 仕様に
//!     存在しないため、診断を出さずにテキストへフォールバックする(曖昧点。最終報告参照)
//!
//! シグネチャは確定済み: `src` は文書全体、`span` はこのインライン領域の絶対バイト
//! スパン。段落は複数行を含みうるため、改行はプレーンテキストの一部としてそのまま
//! 保持する(特別扱いしない)。

use ulid::Ulid;

use crate::ast::{CellCoord, EmphKind, RefScheme, RefTarget, SmlInline};
use crate::error::{Diag, DiagKind};
use crate::span::Span;

/// `span` の範囲のインライン内容をパースする。
pub fn parse_inlines(src: &str, span: Span, diags: &mut Vec<Diag>) -> Vec<SmlInline> {
    parse_span(src, span, diags)
}

/// 再帰下降の本体。`span` の範囲を左から右へ1パスで走査し、`**`/`*`/`` ` ``/`$`/`[...]( )`
/// のいずれかの開始マーカーに出会うたびに閉じを探す。見つからなければマーカーを普通の
/// 文字として扱い1バイト進める(寛容フォールバック)。
fn parse_span(src: &str, span: Span, diags: &mut Vec<Diag>) -> Vec<SmlInline> {
    let bytes = src.as_bytes();
    let end = span.end;
    let mut out = Vec::new();
    let mut i = span.start;
    let mut text_start = i;

    while i < end {
        match bytes[i] {
            b'*' if i + 1 < end && bytes[i + 1] == b'*' => {
                if let Some(close) = find_bytes(bytes, i + 2, end, b"**") {
                    flush_text(&mut out, text_start, i);
                    let inner = Span::new(i + 2, close);
                    let children = parse_span(src, inner, diags);
                    out.push(SmlInline::Emph { kind: EmphKind::Strong, children });
                    i = close + 2;
                    text_start = i;
                } else {
                    i += 1;
                }
            }
            b'*' => {
                if let Some(close) = find_byte(bytes, i + 1, end, b'*') {
                    flush_text(&mut out, text_start, i);
                    let inner = Span::new(i + 1, close);
                    let children = parse_span(src, inner, diags);
                    out.push(SmlInline::Emph { kind: EmphKind::Em, children });
                    i = close + 1;
                    text_start = i;
                } else {
                    i += 1;
                }
            }
            b'`' => {
                if let Some(close) = find_byte(bytes, i + 1, end, b'`') {
                    flush_text(&mut out, text_start, i);
                    let inner = Span::new(i + 1, close);
                    // code の中はネスト不可: 中身を再パースせず単一の Text にする。
                    out.push(SmlInline::Emph {
                        kind: EmphKind::Code,
                        children: vec![SmlInline::Text(inner)],
                    });
                    i = close + 1;
                    text_start = i;
                } else {
                    i += 1;
                }
            }
            b'$' => {
                if let Some(close) = find_byte(bytes, i + 1, end, b'$') {
                    flush_text(&mut out, text_start, i);
                    // 内側 TeX のスパンのみを記録する(中身はパースしない)。
                    out.push(SmlInline::MathTex(Span::new(i + 1, close)));
                    i = close + 1;
                    text_start = i;
                } else {
                    i += 1;
                }
            }
            b'[' => {
                if let Some((node, next_i)) = try_parse_link(src, i, end, diags) {
                    flush_text(&mut out, text_start, i);
                    out.push(node);
                    i = next_i;
                    text_start = i;
                } else {
                    i += 1;
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    flush_text(&mut out, text_start, end);
    out
}

fn flush_text(out: &mut Vec<SmlInline>, start: usize, end: usize) {
    if start < end {
        out.push(SmlInline::Text(Span::new(start, end)));
    }
}

/// 次に現れる単一バイト `needle` の絶対オフセットを `[start, limit)` の範囲で探す。
fn find_byte(bytes: &[u8], start: usize, limit: usize, needle: u8) -> Option<usize> {
    bytes[start..limit].iter().position(|&b| b == needle).map(|p| p + start)
}

/// 次に現れるバイト列 `needle` の絶対開始オフセットを `[start, limit)` の範囲で探す。
fn find_bytes(bytes: &[u8], start: usize, limit: usize, needle: &[u8]) -> Option<usize> {
    let n = needle.len();
    if n == 0 || start + n > limit {
        return None;
    }
    (start..=limit - n).find(|&i| &bytes[i..i + n] == needle)
}

/// `[text](scheme:target...)` を `i`(`[` の位置)から試しにパースする。
/// 成功すれば `(ノード, 続きの絶対オフセット)` を返す。構文が壊れていれば `None`
/// (呼び出し側は `[` を普通の1文字として扱い、寛容にフォールバックする)。
fn try_parse_link(src: &str, open: usize, limit: usize, diags: &mut Vec<Diag>) -> Option<(SmlInline, usize)> {
    let bytes = src.as_bytes();
    let text_start = open + 1;
    let close_bracket = find_byte(bytes, text_start, limit, b']')?;
    if close_bracket + 1 >= limit || bytes[close_bracket + 1] != b'(' {
        return None;
    }
    let dest_start = close_bracket + 2;
    let close_paren = find_byte(bytes, dest_start, limit, b')')?;

    let text_span = Span::new(text_start, close_bracket);
    let dest_span = Span::new(dest_start, close_paren);
    let dest_text = dest_span.slice(src);
    let next_i = close_paren + 1;

    // 外部リンク(`https://...` 等)は v0 仕様に存在しない。診断を出さずフォールバックする
    // (曖昧点。最終報告参照)。
    if dest_text.contains("://") {
        return None;
    }

    let colon_idx = dest_text.find(':')?;
    let scheme_word = &dest_text[..colon_idx];
    let rest = &dest_text[colon_idx + 1..];
    let rest_abs_start = dest_start + colon_idx + 1;

    match scheme_word {
        "ref" | "table" | "fig" | "math" => {
            if rest.is_empty() {
                return None;
            }
            let scheme = match scheme_word {
                "ref" => RefScheme::Ref,
                "table" => RefScheme::Table,
                "fig" => RefScheme::Fig,
                "math" => RefScheme::Math,
                _ => unreachable!("scheme_word はこの4語のいずれかで match 済み"),
            };
            let target_span = Span::new(rest_abs_start, dest_span.end);
            let target = resolve_target(rest, target_span, diags);
            Some((SmlInline::Ref { scheme, target, coord: None, text: text_span }, next_i))
        }
        "term" => {
            // term: の target のみ字句制限の対象外(D5 の対象外、sml-spec §5.2)。
            if rest.is_empty() {
                return None;
            }
            let name_or_id = match rest.parse::<Ulid>() {
                Ok(u) => RefTarget::Ulid(u),
                Err(_) => RefTarget::Label(rest.to_string()),
            };
            Some((SmlInline::TermRef { name_or_id, text: text_span }, next_i))
        }
        "cell" => {
            let hash_idx = rest.find('#')?;
            let target_str = &rest[..hash_idx];
            if target_str.is_empty() {
                return None;
            }
            let target_span = Span::new(rest_abs_start, rest_abs_start + hash_idx);
            let target = resolve_target(target_str, target_span, diags);

            let coord_str = &rest[hash_idx + 1..];
            let coord_abs_start = rest_abs_start + hash_idx + 1;
            let coord_span = Span::new(coord_abs_start, dest_span.end);
            let (row_path, col_path, coord_ok) = parse_cell_coord(coord_str);
            if !coord_ok {
                diags.push(Diag::new(
                    DiagKind::BadCellCoord,
                    coord_span,
                    format!("cell 参照の座標 '{coord_str}' の字句が不正です(path は key(\".\" key)* のみ許可)"),
                ));
            }
            Some((
                SmlInline::Ref {
                    scheme: RefScheme::Cell,
                    target,
                    coord: Some(CellCoord { row_path, col_path }),
                    text: text_span,
                },
                next_i,
            ))
        }
        _ => {
            // 未知のスキーム: 診断は積むが、ノードは構築せずテキストへフォールバックする。
            diags.push(Diag::new(
                DiagKind::UnknownScheme,
                dest_span,
                format!("未知の参照スキーム '{scheme_word}' です(ref/term/table/fig/math/cell のいずれでもありません)"),
            ));
            None
        }
    }
}

/// `ref` / `table` / `fig` / `math` / `cell` の target を解決する。ULID ならそのまま、
/// そうでなければ `[A-Za-z0-9_-]+` の字句検証を行い、違反すれば `BadKeyCharset` を積む
/// (ノード自体は構築を続ける — block.rs の alias 検証と同じ方針)。
fn resolve_target(text: &str, span: Span, diags: &mut Vec<Diag>) -> RefTarget {
    if let Ok(u) = text.parse::<Ulid>() {
        return RefTarget::Ulid(u);
    }
    if !is_valid_key_charset(text) {
        diags.push(Diag::new(
            DiagKind::BadKeyCharset,
            span,
            format!("参照ターゲット '{text}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
        ));
    }
    RefTarget::Label(text.to_string())
}

fn is_valid_key_charset(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// `<行path>|<列path>` をパースする。`path = key ("." key)*`(sml-spec §7)。
/// 戻り値は `(row_path, col_path, 全キーが字句的に妥当か)`。
/// `|` が無い、または `.` で割ったキーが `[A-Za-z0-9_-]+` に違反する場合は
/// 3番目の値が `false`(呼び出し側が `BadCellCoord` を積む)。ノード自体は
/// パースできた範囲でベストエフォート構築する。
fn parse_cell_coord(coord_str: &str) -> (Vec<String>, Vec<String>, bool) {
    let mut ok = true;
    let (row_str, col_str) = match coord_str.find('|') {
        Some(idx) => (&coord_str[..idx], &coord_str[idx + 1..]),
        None => {
            ok = false;
            (coord_str, "")
        }
    };
    let row_path = split_path(row_str, &mut ok);
    let col_path = split_path(col_str, &mut ok);
    if row_path.is_empty() || col_path.is_empty() {
        ok = false;
    }
    (row_path, col_path, ok)
}

/// `key ("." key)*` を `.` で分割する。`|` `.` の前後の空白は無視する(sml-spec §7)。
fn split_path(s: &str, ok: &mut bool) -> Vec<String> {
    s.split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            if !is_valid_key_charset(part) {
                *ok = false;
            }
            part.to_string()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> (Vec<SmlInline>, Vec<Diag>) {
        let mut diags = Vec::new();
        let span = Span::new(0, src.len());
        let out = parse_inlines(src, span, &mut diags);
        (out, diags)
    }

    #[test]
    fn placeholder_style_plain_text_still_works() {
        let (out, diags) = parse("hello world");
        assert_eq!(out, vec![SmlInline::Text(Span::new(0, 11))]);
        assert!(diags.is_empty());
    }

    #[test]
    fn strong_produces_nested_text() {
        let src = "**12 ms**";
        let (out, diags) = parse(src);
        assert!(diags.is_empty());
        assert_eq!(out.len(), 1);
        match &out[0] {
            SmlInline::Emph { kind: EmphKind::Strong, children } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    SmlInline::Text(sp) => assert_eq!(sp.slice(src), "12 ms"),
                    other => panic!("expected text, got {other:?}"),
                }
            }
            other => panic!("expected strong, got {other:?}"),
        }
    }

    #[test]
    fn code_does_not_nest_emphasis() {
        let src = "`a *b* c`";
        let (out, diags) = parse(src);
        assert!(diags.is_empty());
        assert_eq!(out.len(), 1);
        match &out[0] {
            SmlInline::Emph { kind: EmphKind::Code, children } => {
                assert_eq!(children.len(), 1);
                match &children[0] {
                    SmlInline::Text(sp) => assert_eq!(sp.slice(src), "a *b* c"),
                    other => panic!("expected single text child, got {other:?}"),
                }
            }
            other => panic!("expected code, got {other:?}"),
        }
    }

    #[test]
    fn math_span_covers_inner_tex_only() {
        let src = "$x^2$";
        let (out, diags) = parse(src);
        assert!(diags.is_empty());
        assert_eq!(out.len(), 1);
        match &out[0] {
            SmlInline::MathTex(sp) => assert_eq!(sp.slice(src), "x^2"),
            other => panic!("expected math tex, got {other:?}"),
        }
    }

    #[test]
    fn unclosed_markers_fall_back_to_plain_text() {
        for src in ["**unclosed", "*unclosed", "`unclosed", "$unclosed", "[no paren] after"] {
            let (out, diags) = parse(src);
            assert!(diags.is_empty(), "{src}: unexpected diags {diags:?}");
            assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))], "{src}");
        }
    }

    #[test]
    fn unknown_scheme_falls_back_with_diag() {
        let src = "[x](foo:bar)";
        let (out, diags) = parse(src);
        assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagKind::UnknownScheme);
    }

    #[test]
    fn external_link_falls_back_without_diag() {
        let src = "[x](https://example.com)";
        let (out, diags) = parse(src);
        assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
        assert!(diags.is_empty());
    }
}
