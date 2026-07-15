//! 層B: インラインパース(sml-parser-design.md §3、sml-spec.md §5、M6 D40)。
//!
//! 手書きの再帰下降 + 素朴なバイトスキャン。ブロック(層A/層Bのブロック側)は厳格だが、
//! インラインは寛容(design.md §3): 未対応・不正な構文は診断を出さずに(あるいは
//! 診断は出しつつ)プレーンテキストへフォールバックし、パース全体を止めない。
//!
//! 実装する構文(sml-spec §5、M6 D40 で拡張):
//!   - `\X`(X は ASCII 記号)→ `SmlInline::Escaped`。バイト保存契約と両立させるため、
//!     ソース上のバックスラッシュは消さず、build 側でのみ unescape する
//!   - `**strong**`/`__strong__`、`*em*`/`_em_`、`***bold italic***`、`~~取消線~~`、
//!     `` `code` `` → `SmlInline::Emph`(`code` の中はネスト不可)
//!   - `$...$` → `SmlInline::MathTex(Span)`。**内側 TeX のスパンのみ**を記録し、
//!     中身はパースしない(tex2math は build の仕事)
//!   - `[text](scheme:target)` の参照6スキーム(`ref` `table` `fig` `math` `term` `cell`)
//!     → `SmlInline::Ref` / `SmlInline::TermRef`。`cell:target#path|path` は `CellCoord` へ
//!   - `[text](http(s)://…)` / `[text](mailto:…)` / autolink `<http(s)://…>` →
//!     `SmlInline::Link`。`![alt](url)` → `SmlInline::Image`
//!   - `[text][label]` + 文書中の定義行 `[label]: url` → `SmlInline::Link`
//!     (未解決ラベルはリテラル維持、CommonMark 準拠)
//!
//! スキーム別の target 字句規則:
//!   - `ref` / `table` / `fig` / `math` / `cell` の target: ULID(26字 Crockford)なら
//!     `RefTarget::Ulid`、そうでなければ `RefTarget::Label` だが `[A-Za-z0-9_-]+` の
//!     字句検証を行い、違反すれば `BadKeyCharset` を積む(ノード自体は構築する。
//!     block.rs の alias 検証と同じ「診断は積むが止めない」方針)
//!   - `term` の target のみ字句制限の対象外(日本語等の任意の非空文字列を許す)
//!   - `cell` の座標 path の各 key が字句違反なら `BadCellCoord` を積む(ノードは構築する)
//!   - スキーム語がこの6種のいずれでもなければ `UnknownScheme` を積み、
//!     **ノードは構築せずテキストへフォールバック**(sml-spec に無いスキームなので
//!     未解決のまま残すより「読めるテキスト」に倒す)
//!   - dest が `http://`/`https://`/`mailto:` で始まる場合は外部リンク(M6 D40、
//!     監査②2の解消)。それ以外の `://` を含むスキームは引き続き `UnknownScheme`
//!
//! シグネチャは確定済み: `src` は文書全体、`span` はこのインライン領域の絶対バイト
//! スパン。段落は複数行を含みうるため、改行はプレーンテキストの一部としてそのまま
//! 保持する(特別扱いしない)。

use ulid::Ulid;

use crate::ast::{CellCoord, EmphKind, RefDefs, RefScheme, RefTarget, SmlInline};
use crate::error::{Diag, DiagKind};
use crate::span::Span;

/// `span` の範囲のインライン内容をパースする。`refdefs` は文書内の参照スタイル
/// リンク定義表(M6 D40、正規化ラベル→url スパン)。
pub fn parse_inlines(src: &str, span: Span, diags: &mut Vec<Diag>, refdefs: &RefDefs) -> Vec<SmlInline> {
    parse_span(src, span, diags, refdefs)
}

/// CommonMark の ASCII punctuation(バックスラッシュエスケープの対象、M6 D40)。
fn is_ascii_punct(b: u8) -> bool {
    matches!(
        b,
        b'!' | b'"'
            | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b'-'
            | b'.'
            | b'/'
            | b':'
            | b';'
            | b'<'
            | b'='
            | b'>'
            | b'?'
            | b'@'
            | b'['
            | b'\\'
            | b']'
            | b'^'
            | b'_'
            | b'`'
            | b'{'
            | b'|'
            | b'}'
            | b'~'
    )
}

/// 再帰下降の本体。`span` の範囲を左から右へ1パスで走査し、各種マーカーの開始に
/// 出会うたびに閉じを探す。見つからなければマーカーを普通の文字として扱い1バイト
/// 進める(寛容フォールバック)。
fn parse_span(src: &str, span: Span, diags: &mut Vec<Diag>, refdefs: &RefDefs) -> Vec<SmlInline> {
    let bytes = src.as_bytes();
    let end = span.end;
    let mut out = Vec::new();
    let mut i = span.start;
    let mut text_start = i;

    while i < end {
        // M6(D40、監査②1): バックスラッシュエスケープを最優先で判定する。
        // `\*` はここで消費され、後続の `*` マッチには回らない。
        if bytes[i] == b'\\' && i + 1 < end && is_ascii_punct(bytes[i + 1]) {
            flush_text(&mut out, text_start, i);
            out.push(SmlInline::Escaped(Span::new(i, i + 2)));
            i += 2;
            text_start = i;
            continue;
        }

        match bytes[i] {
            b'*' | b'_' => {
                let c = bytes[i];
                if let Some((node, next_i)) = try_delim(src, i, end, c, 3, diags, refdefs) {
                    flush_text(&mut out, text_start, i);
                    out.push(node);
                    i = next_i;
                    text_start = i;
                } else {
                    i += 1;
                }
            }
            b'~' if i + 1 < end && bytes[i + 1] == b'~' => {
                if let Some((node, next_i)) = try_delim(src, i, end, b'~', 2, diags, refdefs) {
                    flush_text(&mut out, text_start, i);
                    out.push(node);
                    i = next_i;
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
            b'!' if i + 1 < end && bytes[i + 1] == b'[' => {
                if let Some((node, next_i)) = try_parse_image(src, i, end, diags) {
                    flush_text(&mut out, text_start, i);
                    out.push(node);
                    i = next_i;
                    text_start = i;
                } else {
                    i += 1;
                }
            }
            b'<' => {
                if let Some((node, next_i)) = try_parse_autolink(src, i, end) {
                    flush_text(&mut out, text_start, i);
                    out.push(node);
                    i = next_i;
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
                } else if let Some((node, next_i)) = try_parse_ref_link(src, i, end, refdefs) {
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

/// `[start, limit)` 内で連続する `c` の個数を数える。
fn run_length(bytes: &[u8], i: usize, limit: usize, c: u8) -> usize {
    let mut j = i;
    while j < limit && bytes[j] == c {
        j += 1;
    }
    j - i
}

/// `*`/`_`/`~` の強調系デリミタ(M6 D40、監査②7: `***bold italic***` 誤ネスト修正・
/// `_em_`/`__strong__` 対応・`~~取消線~~`)。`c` の連続長 `n`(`max_n` で頭打ち)を
/// 求め、同じ長さ以上で閉じる最初の連続を探す。CommonMark の左/右フランキング規則は
/// 実装しない(簡略化、裁量・最終報告参照)。
fn try_delim(
    src: &str,
    i: usize,
    end: usize,
    c: u8,
    max_n: usize,
    diags: &mut Vec<Diag>,
    refdefs: &RefDefs,
) -> Option<(SmlInline, usize)> {
    let bytes = src.as_bytes();
    let n = run_length(bytes, i, end, c).min(max_n);
    if n == 0 {
        return None;
    }
    let mut j = i + n;
    while j < end {
        if bytes[j] == c {
            let close_n = run_length(bytes, j, end, c);
            if close_n >= n {
                let inner = Span::new(i + n, j);
                let children = parse_span(src, inner, diags, refdefs);
                return Some((build_emph_node(c, n, children), j + n));
            }
            j += close_n;
        } else {
            j += 1;
        }
    }
    None
}

fn build_emph_node(c: u8, n: usize, children: Vec<SmlInline>) -> SmlInline {
    if c == b'~' {
        return SmlInline::Emph { kind: EmphKind::Strike, children };
    }
    match n {
        1 => SmlInline::Emph { kind: EmphKind::Em, children },
        2 => SmlInline::Emph { kind: EmphKind::Strong, children },
        // n==3: `***bold italic***` → CommonMark の <strong><em>…</em></strong> に
        // 対応する Strong(Em(children)) の入れ子(監査②7)。
        _ => SmlInline::Emph {
            kind: EmphKind::Strong,
            children: vec![SmlInline::Emph { kind: EmphKind::Em, children }],
        },
    }
}

/// `<http(s)://…>` / `<mailto:…>` autolink(M6 D40)。空白を含む、または対応する
/// 外部スキームで始まらない場合は `None`(呼び出し側は `<` を普通の1文字として扱う)。
fn try_parse_autolink(src: &str, open: usize, limit: usize) -> Option<(SmlInline, usize)> {
    let bytes = src.as_bytes();
    let close = find_byte(bytes, open + 1, limit, b'>')?;
    let inner = Span::new(open + 1, close);
    let text = inner.slice(src);
    if text.is_empty() || text.chars().any(char::is_whitespace) {
        return None;
    }
    if is_external_scheme(text) {
        Some((SmlInline::Link { url: inner, text: inner }, close + 1))
    } else {
        None
    }
}

/// M6(D40、監査②2): 外部リンクとして扱うスキーム(`http`/`https`/`mailto`)。
fn is_external_scheme(dest: &str) -> bool {
    dest.starts_with("http://") || dest.starts_with("https://") || dest.starts_with("mailto:")
}

/// `![alt](url)` を試しにパースする(M6 D40、監査②2/②3)。
/// - url が外部(http/https/mailto、またはそれ以外の無スキーム URL)→ `Inline::Image`
/// - url が内部参照スキーム(`ref`/`table`/`fig`/`math`/`cell`/`term`)を指す →
///   `ImageRefUnsupported`(Error)を積み、`![alt](url)` 全体をリテラル Text へ
///   フォールバックする(裁量。監査②3の `!` 孤立バグの根絶を兼ねる: 誤って `[alt]`
///   だけが独立した Ref として再パースされることが無いよう、ここで一括消費する)
fn try_parse_image(src: &str, bang: usize, limit: usize, diags: &mut Vec<Diag>) -> Option<(SmlInline, usize)> {
    let bytes = src.as_bytes();
    let open = bang + 1; // '[' の位置
    let text_start = open + 1;
    let close_bracket = find_byte(bytes, text_start, limit, b']')?;
    if close_bracket + 1 >= limit || bytes[close_bracket + 1] != b'(' {
        return None;
    }
    let dest_start = close_bracket + 2;
    let close_paren = find_byte(bytes, dest_start, limit, b')')?;

    let alt_span = Span::new(text_start, close_bracket);
    let dest_span = Span::new(dest_start, close_paren);
    let dest_text = dest_span.slice(src);
    let next_i = close_paren + 1;
    let whole_span = Span::new(bang, next_i);

    if let Some(colon_idx) = dest_text.find(':') {
        let scheme_word = &dest_text[..colon_idx];
        if matches!(scheme_word, "ref" | "table" | "fig" | "math" | "cell" | "term") {
            diags.push(Diag::new(
                DiagKind::ImageRefUnsupported,
                dest_span,
                format!("画像が内部参照スキーム '{scheme_word}:' を指しています(非対応。外部URLのみ許可)"),
            ));
            return Some((SmlInline::Text(whole_span), next_i));
        }
    }

    Some((SmlInline::Image { url: dest_span, alt: alt_span }, next_i))
}

/// `[text](scheme:target...)` を `i`(`[` の位置)から試しにパースする。
/// 成功すれば `(ノード, 続きの絶対オフセット)` を返す。構文が壊れていれば `None`
/// (呼び出し側は参照スタイル `[text][label]` を試し、それも失敗すれば `[` を普通の
/// 1文字として扱い寛容にフォールバックする)。
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

    // M6(D40、監査②2): http(s)/mailto は外部リンクとして Inline::Link へ。
    if is_external_scheme(dest_text) {
        return Some((SmlInline::Link { url: dest_span, text: text_span }, next_i));
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

/// `[text][label]` 参照スタイルリンク(M6 D40、監査②4)。定義行が無い/ラベルが
/// 未解決の場合は `None`(呼び出し側が寛容にリテラルへフォールバックする — 各文字を
/// 1バイトずつ消費し、結果的に `[text][label]` がそのまま Text になる。CommonMark
/// 準拠の「未解決ラベルはリテラル維持」)。
fn try_parse_ref_link(src: &str, open: usize, limit: usize, refdefs: &RefDefs) -> Option<(SmlInline, usize)> {
    let bytes = src.as_bytes();
    let text_start = open + 1;
    let close_bracket = find_byte(bytes, text_start, limit, b']')?;
    if close_bracket + 1 >= limit || bytes[close_bracket + 1] != b'[' {
        return None;
    }
    let label_start = close_bracket + 2;
    let label_close = find_byte(bytes, label_start, limit, b']')?;
    if label_close == label_start {
        // `[text][]` shorthand(暗黙参照)は v0 スコープ外(裁量、最終報告参照)。
        return None;
    }
    let text_span = Span::new(text_start, close_bracket);
    let label_text = Span::new(label_start, label_close).slice(src);
    let key = normalize_label(label_text);
    let url_span = *refdefs.get(&key)?;
    Some((SmlInline::Link { url: url_span, text: text_span }, label_close + 1))
}

/// 参照スタイルリンクのラベル正規化(M6 D40): 前後の空白を除去し小文字化する。
/// `block.rs` の定義行収集(`collect_link_ref_defs`)と同じ規則を使う必要がある。
pub(crate) fn normalize_label(s: &str) -> String {
    s.trim().to_lowercase()
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
    use std::collections::HashMap;

    fn parse(src: &str) -> (Vec<SmlInline>, Vec<Diag>) {
        let mut diags = Vec::new();
        let span = Span::new(0, src.len());
        let refdefs = HashMap::new();
        let out = parse_inlines(src, span, &mut diags, &refdefs);
        (out, diags)
    }

    fn parse_with_refdefs(src: &str, refdefs: &RefDefs) -> (Vec<SmlInline>, Vec<Diag>) {
        let mut diags = Vec::new();
        let span = Span::new(0, src.len());
        let out = parse_inlines(src, span, &mut diags, refdefs);
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

    // ---- M6 D40: WP-C1 --------------------------------------------------------------

    #[test]
    fn escaped_asterisk_is_not_emphasis() {
        let src = r"\*not emphasis\*";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        // Escaped(\*) Text(not emphasis) Escaped(\*)
        assert_eq!(out.len(), 3);
        assert!(matches!(out[0], SmlInline::Escaped(_)));
        assert!(matches!(out[2], SmlInline::Escaped(_)));
        match &out[0] {
            SmlInline::Escaped(sp) => assert_eq!(sp.slice(src), "\\*"),
            _ => unreachable!(),
        }
    }

    #[test]
    fn bold_italic_triple_star_nests_strong_then_em() {
        let src = "***bold italic***";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(out.len(), 1);
        match &out[0] {
            SmlInline::Emph { kind: EmphKind::Strong, children } => match &children[0] {
                SmlInline::Emph { kind: EmphKind::Em, children } => match &children[0] {
                    SmlInline::Text(sp) => assert_eq!(sp.slice(src), "bold italic"),
                    other => panic!("{other:?}"),
                },
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn underscore_em_and_strong() {
        let (out, _) = parse("_em_");
        assert!(matches!(&out[0], SmlInline::Emph { kind: EmphKind::Em, .. }));
        let (out, _) = parse("__strong__");
        assert!(matches!(&out[0], SmlInline::Emph { kind: EmphKind::Strong, .. }));
    }

    #[test]
    fn strikethrough_is_recognized() {
        let src = "~~struck~~";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(matches!(&out[0], SmlInline::Emph { kind: EmphKind::Strike, .. }));
    }

    #[test]
    fn external_http_link_becomes_link_node() {
        let src = "[Site](https://example.com)";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &out[0] {
            SmlInline::Link { url, text } => {
                assert_eq!(url.slice(src), "https://example.com");
                assert_eq!(text.slice(src), "Site");
            }
            other => panic!("expected link, got {other:?}"),
        }
    }

    #[test]
    fn mailto_link_becomes_link_node() {
        let src = "[Mail](mailto:a@example.com)";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        assert!(matches!(&out[0], SmlInline::Link { .. }));
    }

    #[test]
    fn autolink_is_recognized() {
        let src = "<https://example.com>";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &out[0] {
            SmlInline::Link { url, text } => {
                assert_eq!(url.slice(src), "https://example.com");
                assert_eq!(text.slice(src), "https://example.com");
            }
            other => panic!("expected link, got {other:?}"),
        }
    }

    #[test]
    fn external_image_becomes_image_node() {
        let src = "![alt text](https://example.com/x.png)";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &out[0] {
            SmlInline::Image { url, alt } => {
                assert_eq!(url.slice(src), "https://example.com/x.png");
                assert_eq!(alt.slice(src), "alt text");
            }
            other => panic!("expected image, got {other:?}"),
        }
    }

    /// 監査②3: `![alt](ref:target)` の `!` 孤立バグの解消。全体がリテラルへ
    /// フォールバックし、`[alt](ref:target)` が独立した Ref として誤解決されないこと。
    #[test]
    fn image_with_internal_ref_scheme_is_rejected_as_whole() {
        let src = "![alt](ref:target)";
        let (out, diags) = parse(src);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagKind::ImageRefUnsupported);
        assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
    }

    #[test]
    fn reference_style_link_resolves_against_refdefs() {
        let mut refdefs = HashMap::new();
        let url_src = "https://example.com/ex";
        refdefs.insert("ex".to_string(), Span::new(0, url_src.len()));
        let src = "[Example][ex]";
        let (out, diags) = parse_with_refdefs(src, &refdefs);
        assert!(diags.is_empty(), "{diags:?}");
        match &out[0] {
            SmlInline::Link { text, .. } => assert_eq!(text.slice(src), "Example"),
            other => panic!("expected link, got {other:?}"),
        }
    }

    #[test]
    fn unresolved_reference_style_link_stays_literal() {
        let src = "[Example][missing]";
        let (out, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
    }
}
