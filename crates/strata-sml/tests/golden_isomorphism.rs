//! ゴールデン同型テスト(WP5, sml-parser-design.md §7-1・§8)。
//!
//! 検証すること:
//! 1. `docs/sml_example_draft.sml` と `sml_example_formatted.sml` が両方とも
//!    diags **ゼロ**でパースできること
//! 2. 両ファイルの AST が「ID情報を無視すれば同型」であること(fmt 契約
//!    「意味保存」— sml-spec §8.1 — をパーサ側から挟む検証)
//! 3. 非対応 Markdown(blockquote / GFM表 / setext見出し)がエラーにならず
//!    段落として解釈されること(design.md §5・§7-4 のフォールバック方針)
//!
//! 同型比較は「ID/Span を消した正規化構造」への変換(`norm_doc` 以下)を経由して
//! 行う。無視するのは: 各ブロックの id_tag 全体(alias 含む)、属性行中の `id`
//! エントリ、すべての Span の数値。それ以外(ブロック種別・見出しレベル・リスト
//! 項目数・属性行の id 以外のエントリ・インラインの構造とテキスト内容・参照の
//! scheme/target/coord・表の次元木とセル)は厳密に比較する。

use strata_sml::{
    AttrLine, AttrValue, BlockKind, CellEntry, CellRaw, DimNode, EmphKind, FenceBody, FenceKind, MemberNode,
    RefScheme, RefTarget, SmlBlock, SmlDocument, SmlInline, TableBody,
};

fn read_doc(rel: &str) -> String {
    let path = format!("{}/../../docs/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

// ---- 正規化構造(ID/Span を消したもの) ------------------------------------------

#[derive(Debug, PartialEq)]
struct NAttrEntry {
    key: String,
    value: AttrValue,
}

/// 属性行1本を正規化する。`id` エントリは無視する(タスク仕様)。
fn norm_attr_line(al: &AttrLine) -> Vec<NAttrEntry> {
    al.entries
        .iter()
        .filter(|(k, _, _)| k != "id")
        .map(|(k, v, _)| NAttrEntry { key: k.clone(), value: v.clone() })
        .collect()
}

fn norm_attrs(attrs: &Option<AttrLine>) -> Vec<NAttrEntry> {
    attrs.as_ref().map(norm_attr_line).unwrap_or_default()
}

#[derive(Debug, PartialEq)]
enum NInline {
    Text(String),
    Emph { kind: EmphKind, children: Vec<NInline> },
    MathTex(String),
    Ref { scheme: RefScheme, target: RefTarget, coord: Option<strata_sml::CellCoord>, text: String },
    TermRef { name_or_id: RefTarget, text: String },
}

fn norm_inline(src: &str, node: &SmlInline) -> NInline {
    match node {
        SmlInline::Text(sp) => NInline::Text(sp.slice(src).to_string()),
        SmlInline::Emph { kind, children } => NInline::Emph { kind: *kind, children: norm_inlines(src, children) },
        SmlInline::MathTex(sp) => NInline::MathTex(sp.slice(src).to_string()),
        SmlInline::Ref { scheme, target, coord, text } => {
            NInline::Ref { scheme: *scheme, target: target.clone(), coord: coord.clone(), text: text.slice(src).to_string() }
        }
        SmlInline::TermRef { name_or_id, text } => {
            NInline::TermRef { name_or_id: name_or_id.clone(), text: text.slice(src).to_string() }
        }
    }
}

fn norm_inlines(src: &str, list: &[SmlInline]) -> Vec<NInline> {
    list.iter().map(|n| norm_inline(src, n)).collect()
}

#[derive(Debug, PartialEq)]
struct NDim {
    name: String,
    members: Vec<NMember>,
}

#[derive(Debug, PartialEq)]
struct NMember {
    key: String,
    label: Option<String>,
    children: Vec<NDim>,
}

fn norm_dim(d: &DimNode) -> NDim {
    NDim { name: d.name.clone(), members: d.members.iter().map(norm_member).collect() }
}

fn norm_member(m: &MemberNode) -> NMember {
    NMember { key: m.key.clone(), label: m.label.clone(), children: m.children.iter().map(norm_dim).collect() }
}

#[derive(Debug, PartialEq)]
struct NCell {
    row_path: Vec<String>,
    col_path: Vec<String>,
    value: CellRaw,
}

fn norm_cell(c: &CellEntry) -> NCell {
    NCell { row_path: c.row_path.clone(), col_path: c.col_path.clone(), value: c.value.clone() }
}

#[derive(Debug, PartialEq)]
struct NTableBody {
    rows: Vec<NDim>,
    cols: Vec<NDim>,
    cells: Vec<NCell>,
}

fn norm_table(tb: &TableBody) -> NTableBody {
    NTableBody {
        rows: tb.rows.iter().map(norm_dim).collect(),
        cols: tb.cols.iter().map(norm_dim).collect(),
        cells: tb.cells.iter().map(norm_cell).collect(),
    }
}

#[derive(Debug, PartialEq)]
enum NFenceBody {
    Table(NTableBody),
    MathTex(String),
    Figure,
}

fn norm_fence_body(src: &str, body: &FenceBody) -> NFenceBody {
    match body {
        FenceBody::Table(tb) => NFenceBody::Table(norm_table(tb)),
        FenceBody::MathTex(sp) => NFenceBody::MathTex(sp.slice(src).to_string()),
        FenceBody::Figure => NFenceBody::Figure,
    }
}

#[derive(Debug, PartialEq)]
enum NBlockKind {
    Heading { level: u8, inline: Vec<NInline> },
    Paragraph { inline: Vec<NInline> },
    /// 各要素は1項目のインライン列(id_tag は無視)。
    List { ordered: bool, items: Vec<Vec<NInline>> },
    Fence { fence_kind: FenceKind, fence_attrs: Vec<Vec<NAttrEntry>>, body: NFenceBody },
    CodeFence { lang: String, body: String },
}

#[derive(Debug, PartialEq)]
struct NBlock {
    attrs: Vec<NAttrEntry>,
    kind: NBlockKind,
}

fn norm_block(src: &str, b: &SmlBlock) -> NBlock {
    let kind = match &b.kind {
        BlockKind::Heading { level, inline, id_tag: _ } => {
            NBlockKind::Heading { level: *level, inline: norm_inlines(src, inline) }
        }
        BlockKind::Paragraph { inline } => NBlockKind::Paragraph { inline: norm_inlines(src, inline) },
        BlockKind::List { ordered, items } => NBlockKind::List {
            ordered: *ordered,
            items: items.iter().map(|it| norm_inlines(src, &it.inline)).collect(),
        },
        BlockKind::Fence(fb) => NBlockKind::Fence {
            fence_kind: fb.fence_kind,
            fence_attrs: fb.fence_attrs.iter().map(norm_attr_line).collect(),
            body: norm_fence_body(src, &fb.body),
        },
        BlockKind::CodeFence { lang, body } => {
            NBlockKind::CodeFence { lang: lang.clone(), body: body.slice(src).to_string() }
        }
    };
    NBlock { attrs: norm_attrs(&b.attrs), kind }
}

fn norm_doc(src: &str, doc: &SmlDocument) -> Vec<NBlock> {
    doc.blocks.iter().map(|b| norm_block(src, b)).collect()
}

// ---- 1. ゴールデンペアが diags ゼロでパースできること ---------------------------

#[test]
fn golden_draft_parses_with_zero_diags() {
    let src = read_doc("sml_example_draft.sml");
    let out = strata_sml::parse(&src);
    assert!(out.diags.is_empty(), "draft: expected zero diags, got {:?}", out.diags);
}

#[test]
fn golden_formatted_parses_with_zero_diags() {
    let src = read_doc("sml_example_formatted.sml");
    let out = strata_sml::parse(&src);
    assert!(out.diags.is_empty(), "formatted: expected zero diags, got {:?}", out.diags);
}

// ---- 2. draft と formatted の AST が ID 無視で同型であること -------------------

#[test]
fn draft_and_formatted_are_isomorphic_ignoring_id_information() {
    let draft_src = read_doc("sml_example_draft.sml");
    let formatted_src = read_doc("sml_example_formatted.sml");

    let draft_out = strata_sml::parse(&draft_src);
    let formatted_out = strata_sml::parse(&formatted_src);

    assert!(draft_out.diags.is_empty(), "draft: expected zero diags, got {:?}", draft_out.diags);
    assert!(formatted_out.diags.is_empty(), "formatted: expected zero diags, got {:?}", formatted_out.diags);

    let draft_norm = norm_doc(&draft_src, &draft_out.doc);
    let formatted_norm = norm_doc(&formatted_src, &formatted_out.doc);

    assert_eq!(
        draft_norm.len(),
        formatted_norm.len(),
        "block count differs: draft has {}, formatted has {}",
        draft_norm.len(),
        formatted_norm.len()
    );

    for (i, (d, f)) in draft_norm.iter().zip(formatted_norm.iter()).enumerate() {
        assert_eq!(d, f, "block #{i} differs between draft and formatted (ID-agnostic comparison)");
    }
}

// ---- 3. 非対応 Markdown のフォールバック ---------------------------------------
//
// design.md §5: blockquote / GFM表 / setext見出しは v0 が解釈しないサブセット外の
// 構文であり、エラーにはせず段落(プレーンテキスト)として読む。

#[test]
fn blockquote_falls_back_to_paragraph_without_diags() {
    let src = "> quoted text\n";
    let out = strata_sml::parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 1);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Paragraph { .. }));
}

#[test]
fn gfm_table_falls_back_to_paragraph_without_diags() {
    let src = "| a | b |\n| - | - |\n| 1 | 2 |\n";
    let out = strata_sml::parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 1);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Paragraph { .. }));
}

#[test]
fn setext_heading_falls_back_to_paragraph_without_diags() {
    // 下線式見出し。v0 は ATX(`#`)のみ対応(design.md §5)。
    let src = "Title\n---\n";
    let out = strata_sml::parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 1);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Paragraph { .. }));
}
