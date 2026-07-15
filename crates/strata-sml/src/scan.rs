//! 層A: ブロックスキャナ(sml-parser-design.md §3)。
//!
//! 行単位の1パスで、ファイルを**ブロックスパンの列**に分割する。インラインの中身・
//! 属性行の key=value・IDタグの中身は見ない(すべて層B = `block.rs` の仕事)。
//!
//! - 空行 = ブロック区切り
//! - 行頭パターンでブロック種別を判定: `#`+SP → 見出し / `- `・`N. ` → リスト項目 /
//!   `::<table|math|figure>` → フェンス開始 / ` ``` ` → コードフェンス開始 /
//!   `[...]`(単独行で `]` 終わり)→ 属性行 / その他 → 段落
//! - フェンスは対応する閉じ(`::` 単独行 / ` ``` ` 行)まで本体を**不透明スパン**として飲む。
//!   閉じ忘れは `UnclosedFence` を積みつつファイル末尾まで飲んで処理を続ける
//! - 属性行は**直後に空行を挟まず**ブロックが続く場合のみ、そのブロックに束縛される。
//!   続かなければ `OrphanAttrLine`(この属性行自体は段落ブロックとして扱う — スパン
//!   被覆不変条件を「隙間は空行のみ」に保つため、宙に浮かせるわけにはいかない)
//!
//! **スパン被覆不変条件**: 出力ブロックの列はオフセット昇順・非重複で、隙間は空行のみ。
//! `Σ(ブロック+隙間) = ファイル全体`(§7 のテストで固定)。

use crate::error::{Diag, DiagKind};
use crate::span::Span;

/// 層Aと層Bの橋渡しに使う内部表現。公開APIには出さない(ast.rs の型のみが公開契約)。
pub(crate) struct RawBlock {
    pub full_span: Span,
    /// 束縛された前置属性行の生スパン(`[` から `]` まで、改行を含まない)。
    pub attr_line_span: Option<Span>,
    pub kind: RawKind,
}

pub(crate) enum RawKind {
    Heading { level: u8, line_span: Span },
    Paragraph { line_spans: Vec<Span> },
    List { ordered: bool, item_line_spans: Vec<Span> },
    /// `marker_line_span`: `::table {#...}` などマーカー行の中身。
    /// `body_span`: マーカー行の次から閉じ `::` 行の手前まで(不透明)。
    Fence { marker_line_span: Span, body_span: Span },
    CodeFence { marker_line_span: Span, body_span: Span },
}

/// 物理行1本。`content` は改行を含まない中身、`full` は改行を含む全体
/// (最終行に改行が無ければ `content == full`)。
pub(crate) struct PhysLine {
    pub(crate) content: Span,
    pub(crate) full: Span,
}

/// `range` に限定して物理行に分割する(フェンス本体・フロントマター後の本体など
/// 部分範囲の再走査に使う)。
pub(crate) fn split_lines_range(src: &str, range: Span) -> Vec<PhysLine> {
    let bytes = src.as_bytes();
    let mut lines = Vec::new();
    let mut start = range.start;
    while start < range.end {
        let mut end = start;
        while end < range.end && bytes[end] != b'\n' {
            end += 1;
        }
        let full_end = if end < range.end { end + 1 } else { end };
        let mut content_end = end;
        if content_end > start && bytes[content_end - 1] == b'\r' {
            content_end -= 1;
        }
        lines.push(PhysLine {
            content: Span::new(start, content_end),
            full: Span::new(start, full_end),
        });
        start = full_end;
    }
    lines
}

fn is_blank(text: &str) -> bool {
    text.trim().is_empty()
}

/// 行頭が属性行(`[...]` 単独)かどうか。中身の key=value 検証は層Bの仕事。
pub(crate) fn looks_like_attr_line(src: &str, content: Span) -> bool {
    let t = content.slice(src).trim();
    t.len() >= 2 && t.starts_with('[') && t.ends_with(']')
}

fn heading_level(text: &str) -> Option<u8> {
    let bytes = text.as_bytes();
    let hashes = bytes.iter().take_while(|&&b| b == b'#').count();
    if hashes == 0 || hashes > 6 {
        return None;
    }
    if bytes.len() > hashes && matches!(bytes[hashes], b' ' | b'\t') {
        Some(hashes as u8)
    } else {
        None
    }
}

/// `Some(true)` = 番号付き、`Some(false)` = 番号なし、`None` = リスト項目でない。
/// `text` は行頭からそのまま(インデント無しを前提)。
fn list_marker_ordered(text: &str) -> Option<bool> {
    if text.starts_with("- ") || text.starts_with("-\t") {
        return Some(false);
    }
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let rest = &text[digits.len()..];
        if rest.starts_with(". ") || rest.starts_with(".\t") {
            return Some(true);
        }
    }
    None
}

/// D24(2026-07-14 裁定): 行頭の半角スペースを何個でも読み飛ばしたうえでリスト項目
/// マーカーを判定する。ネストしたリスト項目(2スペース/レベルでインデントされる)を
/// 層Aのブロック境界検出で「リスト項目行」として扱い続けさせるために使う
/// (2スペース単位かどうか等の妥当性検証は層B `block.rs` の仕事。ここでは「マーカーの
/// 形をしているか」だけを見る)。
fn indented_list_marker_ordered(text: &str) -> Option<bool> {
    list_marker_ordered(text.trim_start_matches(' '))
}

/// `::table` / `::math` / `::figure` / `::record`(D28)のいずれかで開くフェンス
/// マーカーか。grammar (sml-spec 付録A) が `kind` をこの4語に閉じているため、
/// それ以外の `::foo` は非対応構文としてフォールバック(段落扱い)にする。
fn is_fence_open(text: &str) -> bool {
    if text.trim() == "::" {
        // これは閉じ行であって開始行ではない。
        return false;
    }
    match fence_kind_word(text) {
        Some(w) => matches!(w.as_str(), "table" | "math" | "figure" | "record"),
        None => false,
    }
}

pub(crate) fn fence_kind_word(text: &str) -> Option<String> {
    let rest = text.strip_prefix("::")?;
    let word: String = rest.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
    if word.is_empty() { None } else { Some(word) }
}

fn is_fence_close(text: &str) -> bool {
    text.trim() == "::"
}

fn is_code_fence_marker(text: &str) -> bool {
    text.starts_with("```")
}

fn is_code_fence_close(text: &str) -> bool {
    let t = text.trim();
    t.len() >= 3 && t.bytes().all(|b| b == b'`')
}

enum LineClass {
    Blank,
    AttrLine,
    Heading(u8),
    ListItem(bool),
    FenceOpen,
    CodeFenceOpen,
    Paragraph,
}

fn classify(src: &str, line: &PhysLine) -> LineClass {
    let text = line.content.slice(src);
    if is_blank(text) {
        return LineClass::Blank;
    }
    if is_code_fence_marker(text) {
        return LineClass::CodeFenceOpen;
    }
    if is_fence_open(text) {
        return LineClass::FenceOpen;
    }
    if let Some(level) = heading_level(text) {
        return LineClass::Heading(level);
    }
    // D24: インデントされたマーカー行(ネストしたリスト項目の候補)も ListItem として
    // 扱い、リストブロックのグルーピング(scan_one_block)を継続させる。新規ブロックの
    // 開始判定(scan_lines のトップループ)もこの同じ classify を使うため、インデント
    // 付きの孤立したマーカー行がブロック先頭に来た場合も ListItem になる —
    // その場合は層B(block.rs)がインデント0を期待する箇所で `InconsistentIndent` を
    // 診断する(従来の「無警告で別段落に化ける」誤パースを診断に置き換える、D24)。
    if let Some(ordered) = indented_list_marker_ordered(text) {
        return LineClass::ListItem(ordered);
    }
    if looks_like_attr_line(src, line.content) {
        return LineClass::AttrLine;
    }
    LineClass::Paragraph
}

/// 層Aのエントリポイント。`src` の `start` バイトオフセット以降をブロック列に分割する。
/// フロントマター(D12)を読み飛ばした残りを走査するために `parse` から呼ばれる
/// (`start == 0` ならフロントマター無しの通常のファイル全体走査になる)。`start` は
/// 行頭(またはファイル末尾)であることを前提とする。
pub(crate) fn scan_from(src: &str, start: usize, diags: &mut Vec<Diag>) -> Vec<RawBlock> {
    let lines = split_lines_range(src, Span::new(start, src.len()));
    scan_lines(src, &lines, diags)
}

fn scan_lines(src: &str, lines: &[PhysLine], diags: &mut Vec<Diag>) -> Vec<RawBlock> {
    let n = lines.len();
    let mut i = 0usize;
    let mut blocks = Vec::new();

    while i < n {
        match classify(src, &lines[i]) {
            LineClass::Blank => {
                i += 1;
            }
            LineClass::AttrLine => {
                // 連続する属性行ランを集める(grammar 上は1ブロックにつき attr-line は
                // 高々1個だが、実際にスタックされた場合は寛容に扱う: 最後の1個だけを
                // 束縛対象とし、それ以前は孤立として報告する)。
                let run_start = i;
                let mut run_end = i;
                while run_end + 1 < n && matches!(classify(src, &lines[run_end + 1]), LineClass::AttrLine) {
                    run_end += 1;
                }
                let next_idx = run_end + 1;
                let next_is_content = next_idx < n && !matches!(classify(src, &lines[next_idx]), LineClass::Blank);

                if !next_is_content {
                    for line in &lines[run_start..=run_end] {
                        diags.push(Diag::new(
                            DiagKind::OrphanAttrLine,
                            line.content,
                            "属性行が孤立しています(直後に空行、またはファイル終端があります)",
                        ));
                        blocks.push(RawBlock {
                            full_span: line.full,
                            attr_line_span: None,
                            kind: RawKind::Paragraph { line_spans: vec![line.content] },
                        });
                    }
                    i = run_end + 1;
                } else {
                    for line in &lines[run_start..run_end] {
                        diags.push(Diag::new(
                            DiagKind::OrphanAttrLine,
                            line.content,
                            "属性行が孤立しています(直後にさらに属性行が続くため束縛先がありません)",
                        ));
                        blocks.push(RawBlock {
                            full_span: line.full,
                            attr_line_span: None,
                            kind: RawKind::Paragraph { line_spans: vec![line.content] },
                        });
                    }
                    let bound_attr_line = &lines[run_end];
                    let bound_attr_span = bound_attr_line.content;
                    let attr_full_start = bound_attr_line.full.start;

                    let (consumed, mut block) = scan_one_block(src, lines, next_idx, diags);
                    block.full_span = Span::new(attr_full_start, block.full_span.end);
                    block.attr_line_span = Some(bound_attr_span);
                    blocks.push(block);
                    i = next_idx + consumed;
                }
            }
            _ => {
                let (consumed, block) = scan_one_block(src, lines, i, diags);
                blocks.push(block);
                i += consumed;
            }
        }
    }

    blocks
}

/// `lines[i]` から始まる1ブロックを読み取る。`(消費した行数, ブロック)` を返す。
fn scan_one_block(src: &str, lines: &[PhysLine], i: usize, diags: &mut Vec<Diag>) -> (usize, RawBlock) {
    match classify(src, &lines[i]) {
        LineClass::Heading(level) => (
            1,
            RawBlock {
                full_span: lines[i].full,
                attr_line_span: None,
                kind: RawKind::Heading { level, line_span: lines[i].content },
            },
        ),
        LineClass::ListItem(ordered) => {
            let start = i;
            let mut end = i;
            let mut items = vec![lines[i].content];
            while end + 1 < lines.len() {
                if let LineClass::ListItem(_) = classify(src, &lines[end + 1]) {
                    end += 1;
                    items.push(lines[end].content);
                } else {
                    break;
                }
            }
            let full_span = Span::new(lines[start].full.start, lines[end].full.end);
            (
                end - start + 1,
                RawBlock {
                    full_span,
                    attr_line_span: None,
                    kind: RawKind::List { ordered, item_line_spans: items },
                },
            )
        }
        LineClass::FenceOpen => {
            let marker_line_span = lines[i].content;
            let mut j = i + 1;
            let mut closed = false;
            while j < lines.len() {
                if is_fence_close(lines[j].content.slice(src)) {
                    closed = true;
                    break;
                }
                j += 1;
            }
            let (body_span, end_line) = if closed {
                (Span::new(lines[i].full.end, lines[j].full.start), j)
            } else {
                diags.push(Diag::new(
                    DiagKind::UnclosedFence,
                    lines[i].content,
                    "フェンス(::)が閉じられていません",
                ));
                let last = lines.len() - 1;
                (Span::new(lines[i].full.end, lines[last].full.end), last)
            };
            let full_span = Span::new(lines[i].full.start, lines[end_line].full.end);
            (
                end_line - i + 1,
                RawBlock {
                    full_span,
                    attr_line_span: None,
                    kind: RawKind::Fence { marker_line_span, body_span },
                },
            )
        }
        LineClass::CodeFenceOpen => {
            let marker_line_span = lines[i].content;
            let mut j = i + 1;
            let mut closed = false;
            while j < lines.len() {
                if is_code_fence_close(lines[j].content.slice(src)) {
                    closed = true;
                    break;
                }
                j += 1;
            }
            let (body_span, end_line) = if closed {
                (Span::new(lines[i].full.end, lines[j].full.start), j)
            } else {
                diags.push(Diag::new(
                    DiagKind::UnclosedFence,
                    lines[i].content,
                    "コードフェンス(```)が閉じられていません",
                ));
                let last = lines.len() - 1;
                (Span::new(lines[i].full.end, lines[last].full.end), last)
            };
            let full_span = Span::new(lines[i].full.start, lines[end_line].full.end);
            (
                end_line - i + 1,
                RawBlock {
                    full_span,
                    attr_line_span: None,
                    kind: RawKind::CodeFence { marker_line_span, body_span },
                },
            )
        }
        // Blank / AttrLine はここには来ない(呼び出し側で弾いている)。それ以外は段落。
        _ => {
            let start = i;
            let mut end = i;
            let mut spans = vec![lines[i].content];
            while end + 1 < lines.len() && matches!(classify(src, &lines[end + 1]), LineClass::Paragraph) {
                end += 1;
                spans.push(lines[end].content);
            }
            let full_span = Span::new(lines[start].full.start, lines[end].full.end);
            (
                end - start + 1,
                RawBlock {
                    full_span,
                    attr_line_span: None,
                    kind: RawKind::Paragraph { line_spans: spans },
                },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn no_diags(src: &str) -> Vec<RawBlock> {
        let mut diags = Vec::new();
        let blocks = scan_from(src, 0, &mut diags);
        assert!(diags.is_empty(), "unexpected diags: {diags:?}");
        blocks
    }

    /// 任意の入力に対し、ブロックの列(隙間を含む)が全バイトを昇順・非重複・全被覆で
    /// カバーし、隙間が空行のみであることを検証する。
    fn assert_coverage(src: &str, blocks: &[RawBlock]) {
        let mut cursor = 0usize;
        for b in blocks {
            assert!(b.full_span.start >= cursor, "block starts before cursor");
            if b.full_span.start > cursor {
                let gap = &src[cursor..b.full_span.start];
                for line in gap.split_inclusive('\n') {
                    assert!(line.trim().is_empty(), "gap is not blank: {line:?}");
                }
            }
            assert!(b.full_span.end >= b.full_span.start);
            cursor = b.full_span.end;
        }
        if cursor < src.len() {
            let gap = &src[cursor..];
            for line in gap.split_inclusive('\n') {
                assert!(line.trim().is_empty(), "trailing gap is not blank: {line:?}");
            }
        }
    }

    #[test]
    fn empty_file_has_no_blocks() {
        let blocks = no_diags("");
        assert!(blocks.is_empty());
        assert_coverage("", &blocks);
    }

    #[test]
    fn blank_only_file_has_no_blocks() {
        let src = "\n\n\n";
        let blocks = no_diags(src);
        assert!(blocks.is_empty());
        assert_coverage(src, &blocks);
    }

    #[test]
    fn heading_paragraph_and_gap_are_covered() {
        let src = "# Title\n\nHello world.\n";
        let mut diags = Vec::new();
        let blocks = scan_from(src, 0, &mut diags);
        assert!(diags.is_empty());
        assert_eq!(blocks.len(), 2);
        assert_coverage(src, &blocks);
    }

    #[test]
    fn unclosed_fence_reports_diag_and_consumes_to_eof() {
        let src = "::table {#x}\n@rows:\n  - a: [b]\n";
        let mut diags = Vec::new();
        let blocks = scan_from(src, 0, &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagKind::UnclosedFence);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].full_span, Span::new(0, src.len()));
        assert_coverage(src, &blocks);
    }

    #[test]
    fn unclosed_code_fence_reports_diag() {
        let src = "```rust\nfn main() {}\n";
        let mut diags = Vec::new();
        let blocks = scan_from(src, 0, &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagKind::UnclosedFence);
        assert_coverage(src, &blocks);
    }

    #[test]
    fn orphan_attr_line_at_eof() {
        let src = "[id=foo]\n";
        let mut diags = Vec::new();
        let blocks = scan_from(src, 0, &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagKind::OrphanAttrLine);
        assert_eq!(blocks.len(), 1);
        assert_coverage(src, &blocks);
    }

    #[test]
    fn orphan_attr_line_before_blank() {
        let src = "[id=foo]\n\nParagraph.\n";
        let mut diags = Vec::new();
        let blocks = scan_from(src, 0, &mut diags);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagKind::OrphanAttrLine);
        assert_eq!(blocks.len(), 2);
        assert_coverage(src, &blocks);
    }

    #[test]
    fn attr_line_binds_to_following_paragraph() {
        let src = "[id=foo]\nParagraph text.\n";
        let mut diags = Vec::new();
        let blocks = scan_from(src, 0, &mut diags);
        assert!(diags.is_empty());
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].attr_line_span.is_some());
        assert_coverage(src, &blocks);
    }

    #[test]
    fn ordered_and_unordered_list_items_group_together() {
        let src = "- one\n- two\n";
        let blocks = no_diags(src);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            RawKind::List { ordered, item_line_spans } => {
                assert!(!ordered);
                assert_eq!(item_line_spans.len(), 2);
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn non_conforming_markdown_falls_back_to_paragraph() {
        // blockquote は非対応 → 段落フォールバック
        let src = "> quoted text\n";
        let blocks = no_diags(src);
        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0].kind, RawKind::Paragraph { .. }));
    }
}
