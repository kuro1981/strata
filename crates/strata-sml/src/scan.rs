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
    /// M6(D40): 段落直後の Setext 下線(`===`/`---`)を検出した場合、`level` 1/2 の
    /// 見出しとして扱う。`line_spans` は下線を除いた見出しテキスト行。
    SetextHeading { level: u8, line_spans: Vec<Span> },
    List { ordered: bool, start: Option<u64>, item_line_spans: Vec<Span> },
    /// `marker_line_span`: `::table {#...}` などマーカー行の中身。
    /// `body_span`: マーカー行の次から閉じ `::` 行の手前まで(不透明)。
    Fence { marker_line_span: Span, body_span: Span },
    CodeFence { marker_line_span: Span, body_span: Span },
    /// M6(D40): 参照スタイルリンクの定義行 `[label]: url "title"`。
    LinkRefDef { line_span: Span },
    /// M6(D40): blockquote(`>` 行群)。`inner_lines` は `>`(+ 1個の空白)を除いた
    /// 本文行(絶対オフセットのまま)。層Bが再帰的にブロック列へ組み立てる。
    Quote { inner_lines: Vec<PhysLine> },
    /// M6(D40): 単独行の水平線(`---`/`***`/`___`)。
    ThematicBreak,
    /// M6(D40 Tier2): GFM パイプ表。`header_span`/`delim_span` はそれぞれの行、
    /// `row_spans` はデータ行。
    GfmTable { header_span: Span, row_spans: Vec<Span> },
}

/// 物理行1本。`content` は改行を含まない中身、`full` は改行を含む全体
/// (最終行に改行が無ければ `content == full`)。
#[derive(Clone, Copy)]
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
///
/// M6(D40、監査「固有記法との衝突」): `[...]` 単独行のうち **`=` を1つも含まない**
/// ものは属性行と誤認しない(属性行の文法 `[key=value, ...]` は必ず `=` を持つ)。
/// これにより GFM チェックボックス単独行(`[ ]`/`[x]`)や参照スタイルリンク単独行
/// (`[Example][ex]`)が段落として素通りする。従来この形は「キーのみの不正な属性行」
/// として BadKeyCharset 等になり得た(裁量、最終報告参照)。
pub(crate) fn looks_like_attr_line(src: &str, content: Span) -> bool {
    let t = content.slice(src).trim();
    t.len() >= 2 && t.starts_with('[') && t.ends_with(']') && t.contains('=')
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
///
/// M6(D40): CommonMark の代替マーカーに対応する — 箇条書きは `-`/`*`/`+`、
/// 順序リストは `N.`/`N)`(どちらも数字+区切り文字+空白)。
pub(crate) fn list_marker_ordered(text: &str) -> Option<bool> {
    if text.starts_with("- ") || text.starts_with("-\t") {
        return Some(false);
    }
    if text.starts_with("* ") || text.starts_with("*\t") || text.starts_with("+ ") || text.starts_with("+\t") {
        return Some(false);
    }
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if !digits.is_empty() {
        let rest = &text[digits.len()..];
        if rest.starts_with(". ") || rest.starts_with(".\t") || rest.starts_with(") ") || rest.starts_with(")\t") {
            return Some(true);
        }
    }
    None
}

/// 順序リストマーカー(`N.`/`N)`)の数値部分を読む(M6 D40、開始値保存)。
/// `text` は行頭からそのまま(インデント無しを前提。呼び出し側で `trim_start_matches(' ')`
/// 済みを渡す)。
pub(crate) fn list_marker_start_value(text: &str) -> Option<u64> {
    let digits: String = text.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() { None } else { digits.parse().ok() }
}

/// M6(D40): 単独行の水平線/Setext 下線候補。トリム後が同一文字(`-`/`*`/`_`/`=`)の
/// 繰り返しのみで構成されていれば `Some(文字)` を返す(文字間の空白混在は非対応、
/// 裁量・最終報告参照)。長さの妥当性(HR は3文字以上、Setext は1文字以上)は
/// 呼び出し側が判断する。
fn rule_char(text: &str) -> Option<(u8, usize)> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let c = t.as_bytes()[0];
    if !matches!(c, b'-' | b'*' | b'_' | b'=') {
        return None;
    }
    if t.bytes().all(|b| b == c) { Some((c, t.len())) } else { None }
}

/// M6(D40): 単独行の水平線(3文字以上の `-`/`*`/`_` の繰り返し)かどうか。
fn is_thematic_break_line(text: &str) -> bool {
    matches!(rule_char(text), Some((b'-' | b'*' | b'_', n)) if n >= 3)
}

/// M6(D40): Setext 見出しの下線(`===...` は H1、`---...` は H2。長さ1文字以上)。
fn setext_level(text: &str) -> Option<u8> {
    match rule_char(text) {
        Some((b'=', _)) => Some(1),
        Some((b'-', _)) => Some(2),
        _ => None,
    }
}

/// M6(D40): blockquote 行(`>` で始まる。先頭の半角スペースは許容)。
fn is_blockquote_line(text: &str) -> bool {
    text.trim_start_matches(' ').starts_with('>')
}

/// `>` 行から `>`(+ 直後の半角スペース1個があれば1個)を取り除いた内側スパンを返す。
fn strip_blockquote_marker(src: &str, content: Span) -> Span {
    let bytes = src.as_bytes();
    let mut i = content.start;
    while i < content.end && bytes[i] == b' ' {
        i += 1;
    }
    debug_assert!(i < content.end && bytes[i] == b'>');
    i += 1;
    if i < content.end && bytes[i] == b' ' {
        i += 1;
    }
    Span::new(i, content.end)
}

/// M6(D40): 参照スタイルリンクの定義行 `[label]: url ["title"]`。
fn looks_like_link_ref_def(src: &str, content: Span) -> bool {
    let t = content.slice(src).trim();
    if !t.starts_with('[') {
        return false;
    }
    let Some(close) = t.find(']') else { return false };
    if close <= 1 {
        return false;
    }
    let rest = &t[close + 1..];
    let Some(rest) = rest.strip_prefix(':') else { return false };
    !rest.trim().is_empty()
}

/// M6(D40 Tier2): GFM パイプ表のヘッダ候補行(パイプを含む)。
fn looks_like_table_row(text: &str) -> bool {
    text.contains('|')
}

/// M6(D40 Tier2): GFM パイプ表の区切り行(`---|:--:|--:` 等、セルは `:?-+:?` のみ)。
fn is_table_delim_row(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() || !t.contains('|') && !t.contains('-') {
        return false;
    }
    let t = t.strip_prefix('|').unwrap_or(t);
    let t = t.strip_suffix('|').unwrap_or(t);
    if t.trim().is_empty() {
        return false;
    }
    t.split('|').all(|cell| {
        let c = cell.trim();
        if c.is_empty() {
            return false;
        }
        let c = c.strip_prefix(':').unwrap_or(c);
        let c = c.strip_suffix(':').unwrap_or(c);
        !c.is_empty() && c.bytes().all(|b| b == b'-')
    })
}

/// HTML ブロックらしき行(M6 D40 Tier3)。`<tag` / `</tag` / `<!--` の粗い判定。
/// 意味グラフには落とさず Warning を出すためだけに使う(block.rs から呼ばれる)。
pub(crate) fn looks_like_html_line(text: &str) -> bool {
    let t = text.trim_start();
    let Some(rest) = t.strip_prefix('<') else { return false };
    if let Some(rest2) = rest.strip_prefix('!') {
        return rest2.starts_with("--") || rest2.chars().next().is_some_and(|c| c.is_ascii_alphabetic());
    }
    let rest = rest.strip_prefix('/').unwrap_or(rest);
    rest.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
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

/// M6(D40): バッククォート(```` ``` ````)に加えチルド(`~~~`)フェンスも同等に扱う
/// (監査②8 の誤爆根絶)。開始行の先頭文字がフェンス種別を決める。
fn is_code_fence_marker(text: &str) -> bool {
    text.starts_with("```") || text.starts_with("~~~")
}

fn code_fence_char(text: &str) -> u8 {
    if text.starts_with("~~~") { b'~' } else { b'`' }
}

fn is_code_fence_close(text: &str, fence_char: u8) -> bool {
    let t = text.trim();
    t.len() >= 3 && t.bytes().all(|b| b == fence_char)
}

enum LineClass {
    Blank,
    AttrLine,
    Heading(u8),
    ListItem(bool),
    FenceOpen,
    CodeFenceOpen,
    /// M6(D40): 単独行の水平線候補(3文字以上の `-`/`*`/`_`)。ブロック開始位置では
    /// `ThematicBreak` ブロックになるが、直前が段落の場合は Setext 下線として
    /// 再解釈されうる(scan_one_block のデフォルト枝が判定する)。
    ThematicBreak,
    /// M6(D40): `>` で始まる blockquote 行。
    BlockquoteLine,
    /// M6(D40): 参照スタイルリンクの定義行。
    LinkRefDef,
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
    if is_thematic_break_line(text) {
        return LineClass::ThematicBreak;
    }
    if is_blockquote_line(text) {
        return LineClass::BlockquoteLine;
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
    if looks_like_link_ref_def(src, line.content) {
        return LineClass::LinkRefDef;
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

/// M6(D40、監査②6): 空行区切りの同種マーカーの連続リストを1つの List に統合する
/// (CommonMark の loose list)。マーカーの種類(`-`/`*`/`+`/`N.`/`N)`)は区別せず
/// `ordered` の真偽だけで同種と判定する(scan.rs の既存の粒度に合わせた簡略化。
/// 裁量、最終報告参照)。統合された2つ目以降のリストが自身の前置属性行を持つ場合
/// (それ自体が明示的に別リストとして ID/属性を与えられている)は統合しない。
/// loose/tight の区別(CommonMark で項目が `<p>` に包まれるか)は canonical
/// `List` に持たせない(既存レンダラは項目を常に Para として描画するため区別が
/// 不要。裁量、最終報告参照)。
fn merge_loose_lists(blocks: Vec<RawBlock>) -> Vec<RawBlock> {
    let mut out: Vec<RawBlock> = Vec::with_capacity(blocks.len());
    for block in blocks {
        let mergeable = block.attr_line_span.is_none()
            && matches!(
                (&block.kind, out.last().map(|b: &RawBlock| &b.kind)),
                (RawKind::List { ordered: a, .. }, Some(RawKind::List { ordered: b, .. })) if a == b
            );
        if mergeable {
            let full_end = block.full_span.end;
            let RawKind::List { item_line_spans, .. } = block.kind else {
                unreachable!("mergeable は List 同士でのみ true になる")
            };
            let prev = out.last_mut().expect("mergeable は out が非空であることを前提とする");
            prev.full_span = Span::new(prev.full_span.start, full_end);
            let RawKind::List { item_line_spans: prev_items, .. } = &mut prev.kind else {
                unreachable!("mergeable の判定で prev も List であることを確認済み")
            };
            prev_items.extend(item_line_spans);
        } else {
            out.push(block);
        }
    }
    out
}

pub(crate) fn scan_lines(src: &str, lines: &[PhysLine], diags: &mut Vec<Diag>) -> Vec<RawBlock> {
    merge_loose_lists(scan_lines_raw(src, lines, diags))
}

fn scan_lines_raw(src: &str, lines: &[PhysLine], diags: &mut Vec<Diag>) -> Vec<RawBlock> {
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
        LineClass::ThematicBreak => (
            1,
            RawBlock { full_span: lines[i].full, attr_line_span: None, kind: RawKind::ThematicBreak },
        ),
        LineClass::LinkRefDef => (
            1,
            RawBlock {
                full_span: lines[i].full,
                attr_line_span: None,
                kind: RawKind::LinkRefDef { line_span: lines[i].content },
            },
        ),
        LineClass::BlockquoteLine => {
            let start = i;
            let mut end = i;
            let mut inner = vec![PhysLine {
                content: strip_blockquote_marker(src, lines[i].content),
                full: lines[i].full,
            }];
            while end + 1 < lines.len() && matches!(classify(src, &lines[end + 1]), LineClass::BlockquoteLine) {
                end += 1;
                inner.push(PhysLine {
                    content: strip_blockquote_marker(src, lines[end].content),
                    full: lines[end].full,
                });
            }
            let full_span = Span::new(lines[start].full.start, lines[end].full.end);
            (end - start + 1, RawBlock { full_span, attr_line_span: None, kind: RawKind::Quote { inner_lines: inner } })
        }
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
            let start_value =
                if ordered { list_marker_start_value(lines[start].content.slice(src).trim_start_matches(' ')) } else { None };
            let full_span = Span::new(lines[start].full.start, lines[end].full.end);
            (
                end - start + 1,
                RawBlock {
                    full_span,
                    attr_line_span: None,
                    kind: RawKind::List { ordered, start: start_value, item_line_spans: items },
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
            let fence_char = code_fence_char(lines[i].content.slice(src));
            let mut j = i + 1;
            let mut closed = false;
            while j < lines.len() {
                if is_code_fence_close(lines[j].content.slice(src), fence_char) {
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
        // Blank / AttrLine はここには来ない(呼び出し側で弾いている)。それ以外は段落
        // (M6 D40: GFM パイプ表・Setext 見出しもここから枝分かれする)。
        _ => {
            // M6(D40 Tier2): ヘッダ行 + 区切り行が揃っていれば GFM パイプ表として読む。
            if looks_like_table_row(lines[i].content.slice(src))
                && i + 1 < lines.len()
                && matches!(classify(src, &lines[i + 1]), LineClass::Paragraph)
                && is_table_delim_row(lines[i + 1].content.slice(src))
            {
                let header_span = lines[i].content;
                let mut end = i + 1;
                let mut row_spans = Vec::new();
                while end + 1 < lines.len()
                    && matches!(classify(src, &lines[end + 1]), LineClass::Paragraph)
                    && looks_like_table_row(lines[end + 1].content.slice(src))
                {
                    end += 1;
                    row_spans.push(lines[end].content);
                }
                let full_span = Span::new(lines[i].full.start, lines[end].full.end);
                return (
                    end - i + 1,
                    RawBlock {
                        full_span,
                        attr_line_span: None,
                        kind: RawKind::GfmTable { header_span, row_spans },
                    },
                );
            }

            let start = i;
            let mut end = i;
            let mut spans = vec![lines[i].content];
            while end + 1 < lines.len() {
                let next_text = lines[end + 1].content.slice(src);
                // M6(D40): 次行が Setext 下線候補ならここで段落収集を止める(下線行自体は
                // 別途 peek して消費する。誤って通常の段落継続行として飲み込まない)。
                if setext_level(next_text).is_some() {
                    break;
                }
                if matches!(classify(src, &lines[end + 1]), LineClass::Paragraph) {
                    end += 1;
                    spans.push(lines[end].content);
                } else {
                    break;
                }
            }

            // M6(D40、監査②9): 段落直後に Setext 下線(`===`/`---`)があれば見出しへ
            // 昇格する(CommonMark 準拠でこちらが優先)。
            if end + 1 < lines.len() {
                let next_text = lines[end + 1].content.slice(src);
                if let Some(level) = setext_level(next_text) {
                    let full_span = Span::new(lines[start].full.start, lines[end + 1].full.end);
                    return (
                        end + 1 - start + 1,
                        RawBlock {
                            full_span,
                            attr_line_span: None,
                            kind: RawKind::SetextHeading { level, line_spans: spans },
                        },
                    );
                }
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
            RawKind::List { ordered, item_line_spans, .. } => {
                assert!(!ordered);
                assert_eq!(item_line_spans.len(), 2);
            }
            _ => panic!("expected list"),
        }
    }

    /// M6(D40): blockquote は専用の `RawKind::Quote` になる(旧: 段落フォールバック)。
    #[test]
    fn blockquote_becomes_quote_block() {
        let src = "> quoted text\n";
        let blocks = no_diags(src);
        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            RawKind::Quote { inner_lines } => {
                assert_eq!(inner_lines.len(), 1);
                assert_eq!(inner_lines[0].content.slice(src), "quoted text");
            }
            _ => panic!("expected quote"),
        }
    }
}
