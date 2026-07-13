//! テスト共通ヘルパ(WP-F2, docs/sml-fmt-m2-handoff.md)。
//!
//! `tests/golden_isomorphism.rs`(WP5)が実装した「ID/Span を消した正規化構造」への
//! 変換一式(`norm_doc` 以下)と、ゴールデンfixtureの読み込み(`read_doc`)をここに
//! 共通化する。fmt の契約4(意味保存、D-F5)が draft/formatted の同型比較に加えて、
//! fmt をかけた任意入力の同型比較にも同じロジックを再利用するため。
//!
//! 無視するのは: 各ブロックの id_tag 全体(alias 含む)、属性行中の `id` エントリ、
//! すべての Span の数値。それ以外(ブロック種別・見出しレベル・リスト項目数・属性行の
//! id 以外のエントリ・インラインの構造とテキスト内容・参照の scheme/target/coord・
//! 表の次元木とセル)は厳密に比較する。
//!
//! `tests/common/mod.rs` は Rust の慣習どおり複数のテストバイナリ(golden_isomorphism.rs /
//! fmt_contract.rs)からそれぞれ `mod common;` で取り込まれる。どちらか一方でしか
//! 使わない項目があっても未使用警告が出ないよう、モジュール全体に `dead_code` を許容する。

#![allow(dead_code)]

use strata_sml::{
    AttrLine, AttrValue, BlockKind, CellEntry, CellRaw, DimNode, EmphKind, FenceBody, FenceKind, MemberNode,
    RefScheme, RefTarget, SmlBlock, SmlDocument, SmlInline, TableBody,
};

/// `docs/` 配下のファイルをリポジトリルート相対で読み込む(ゴールデンfixture用)。
pub fn read_doc(rel: &str) -> String {
    let path = format!("{}/../../docs/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

// ---- 正規化構造(ID/Span を消したもの) ------------------------------------------

#[derive(Debug, PartialEq)]
pub struct NAttrEntry {
    pub key: String,
    pub value: AttrValue,
}

/// 属性行1本を正規化する。`id` エントリは無視する(タスク仕様)。
pub fn norm_attr_line(al: &AttrLine) -> Vec<NAttrEntry> {
    al.entries
        .iter()
        .filter(|(k, _, _)| k != "id")
        .map(|(k, v, _)| NAttrEntry { key: k.clone(), value: v.clone() })
        .collect()
}

pub fn norm_attrs(attrs: &Option<AttrLine>) -> Vec<NAttrEntry> {
    attrs.as_ref().map(norm_attr_line).unwrap_or_default()
}

#[derive(Debug, PartialEq)]
pub enum NInline {
    Text(String),
    Emph { kind: EmphKind, children: Vec<NInline> },
    MathTex(String),
    Ref { scheme: RefScheme, target: RefTarget, coord: Option<strata_sml::CellCoord>, text: String },
    TermRef { name_or_id: RefTarget, text: String },
}

pub fn norm_inline(src: &str, node: &SmlInline) -> NInline {
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

pub fn norm_inlines(src: &str, list: &[SmlInline]) -> Vec<NInline> {
    list.iter().map(|n| norm_inline(src, n)).collect()
}

#[derive(Debug, PartialEq)]
pub struct NDim {
    pub name: String,
    pub members: Vec<NMember>,
}

#[derive(Debug, PartialEq)]
pub struct NMember {
    pub key: String,
    pub label: Option<String>,
    pub children: Vec<NDim>,
}

pub fn norm_dim(d: &DimNode) -> NDim {
    NDim { name: d.name.clone(), members: d.members.iter().map(norm_member).collect() }
}

pub fn norm_member(m: &MemberNode) -> NMember {
    NMember { key: m.key.clone(), label: m.label.clone(), children: m.children.iter().map(norm_dim).collect() }
}

#[derive(Debug, PartialEq)]
pub struct NCell {
    pub row_path: Vec<String>,
    pub col_path: Vec<String>,
    pub value: CellRaw,
}

pub fn norm_cell(c: &CellEntry) -> NCell {
    NCell { row_path: c.row_path.clone(), col_path: c.col_path.clone(), value: c.value.clone() }
}

#[derive(Debug, PartialEq)]
pub struct NTableBody {
    pub rows: Vec<NDim>,
    pub cols: Vec<NDim>,
    pub cells: Vec<NCell>,
}

pub fn norm_table(tb: &TableBody) -> NTableBody {
    NTableBody {
        rows: tb.rows.iter().map(norm_dim).collect(),
        cols: tb.cols.iter().map(norm_dim).collect(),
        cells: tb.cells.iter().map(norm_cell).collect(),
    }
}

#[derive(Debug, PartialEq)]
pub enum NFenceBody {
    Table(NTableBody),
    MathTex(String),
    Figure,
}

pub fn norm_fence_body(src: &str, body: &FenceBody) -> NFenceBody {
    match body {
        FenceBody::Table(tb) => NFenceBody::Table(norm_table(tb)),
        FenceBody::MathTex(sp) => NFenceBody::MathTex(sp.slice(src).to_string()),
        FenceBody::Figure => NFenceBody::Figure,
    }
}

#[derive(Debug, PartialEq)]
pub enum NBlockKind {
    Heading { level: u8, inline: Vec<NInline> },
    Paragraph { inline: Vec<NInline> },
    /// 各要素は1項目のインライン列(id_tag は無視)。
    List { ordered: bool, items: Vec<Vec<NInline>> },
    Fence { fence_kind: FenceKind, fence_attrs: Vec<Vec<NAttrEntry>>, body: NFenceBody },
    CodeFence { lang: String, body: String },
}

#[derive(Debug, PartialEq)]
pub struct NBlock {
    pub attrs: Vec<NAttrEntry>,
    pub kind: NBlockKind,
}

pub fn norm_block(src: &str, b: &SmlBlock) -> NBlock {
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

pub fn norm_doc(src: &str, doc: &SmlDocument) -> Vec<NBlock> {
    doc.blocks.iter().map(|b| norm_block(src, b)).collect()
}
