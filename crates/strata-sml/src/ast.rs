//! SML-AST — スパン付き構文木(sml-parser-design.md §4、sml-spec.md 準拠)。
//!
//! 設計上の非対称に注意: **ULID か人間ラベルかは AST が区別する**(fmt が注入対象を
//! 列挙するため)が、**ラベル→ULID の解決は AST では行わない**(エイリアス表の構築は
//! build の仕事。sml-spec §3.4)。`RefTarget` はこの非対称をそのまま型にしたもの。
//!
//! このモジュールは M1 の全 WP(WP1〜WP4)が共有する唯一の型定義の場所。
//! `inline.rs` / `table.rs` はここに定義された型を消費するだけで、
//! 型そのものを追加・変更する必要が無いことを意図している。

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::span::Span;

// ---- ドキュメント / ブロック -------------------------------------------------

/// ドキュメント全体。SML ファイル1つ = `SmlDocument` 1つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmlDocument {
    pub blocks: Vec<SmlBlock>,
    /// 元テキストの全長(バイト)。スパン被覆不変条件の検証に使う。
    pub src_len: usize,
}

/// 層Aが確定させるブロック単位。前置属性行を含む全体スパンを持つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmlBlock {
    /// 属性行を含むブロック全体のスパン。
    pub span: Span,
    /// 前置属性行(あれば)。
    pub attrs: Option<AttrLine>,
    pub kind: BlockKind,
}

/// ブロック種別カタログ(sml-spec §2、v0 サブセットは design.md §5)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BlockKind {
    Heading { level: u8, inline: Vec<SmlInline>, id_tag: Option<IdTag> },
    Paragraph { inline: Vec<SmlInline> },
    /// フラットなリスト(`- ` / `N. `)。項目 = 1行 = 1段落(ネストは保留、sml-spec §10)。
    List { ordered: bool, items: Vec<ListItem> },
    /// `::table` / `::math` / `::figure`。
    Fence(FenceBlock),
    CodeFence { lang: String, body: Span },
}

/// リスト項目。行末に自身の `{#id}` を持ちうる(行型ブロック、sml-spec §3.3)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    /// 項目行のスパン(IDタグを含む、行末改行は含まない)。
    pub span: Span,
    pub inline: Vec<SmlInline>,
    pub id_tag: Option<IdTag>,
}

// ---- ID / エイリアス --------------------------------------------------------

/// 行末 `{#id}` / `{#ULID alias=x}` タグ(sml-spec §3.1、D2/D3)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdTag {
    /// ULID か、fmt 未実行のドラフト段階の人間ラベルか。
    pub id: RefTarget,
    pub alias: Option<String>,
    /// `{#` と `}` の内側(両者を含まない)のスパン。fmt はここだけを書き換える。
    pub inner_span: Span,
}

/// 属性行 `[key=value, ...]`(sml-spec §4)。ブロック前置・フェンス内属性行の両方に使う。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttrLine {
    /// `[` から `]` までを含む行全体のスパン。
    pub span: Span,
    /// `(key, value, "key=value" エントリ全体のスパン)`。
    pub entries: Vec<(String, AttrValue, Span)>,
}

/// 属性値。単一値・引用符付き値・リスト値の3種(sml-spec §4 の例、§7 の字句制約)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AttrValue {
    /// 裸の単一トークン(例: `supports=eval-table`、`id=01J2...`)。
    Single(String),
    /// 引用符付き文字列値(例: `caption="..."`)。引用符を除いた中身を持つ。
    Quoted(String),
    /// リスト値(例: `supports=[claim-1, claim-2]`)。各要素は引用符を剥がした文字列。
    List(Vec<String>),
}

/// ID/参照ターゲットの未解決表現。ULID か人間ラベルかを AST が区別するだけで、
/// ラベル→ULID の解決は行わない(build の仕事、sml-spec §3.4)。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefTarget {
    Ulid(Ulid),
    Label(String),
}

// ---- インライン --------------------------------------------------------------

/// インライン AST(sml-spec §5)。
///
/// WP1/WP2 時点では `inline::parse_inlines` がプレースホルダのため、実際に生成される
/// バリアントは常に `Text` 一つ。他のバリアントは WP4 が実装する再帰下降パーサの
/// 出力先として型だけ確定させてある。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SmlInline {
    Text(Span),
    Emph { kind: EmphKind, children: Vec<SmlInline> },
    /// TeX ソースのまま(遅延パース)。tex2math は build 時に呼ぶ。
    MathTex(Span),
    Ref { scheme: RefScheme, target: RefTarget, coord: Option<CellCoord>, text: Span },
    TermRef { name_or_id: RefTarget, text: Span },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmphKind {
    Strong,
    Em,
    Code,
}

/// インライン参照のスキーム(sml-spec §5.2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefScheme {
    Ref,
    Table,
    Fig,
    Math,
    Cell,
}

/// `cell:` 参照の座標(sml-spec §5.3): `<行path>|<列path>`。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellCoord {
    pub row_path: Vec<String>,
    pub col_path: Vec<String>,
}

// ---- フェンス -----------------------------------------------------------------

/// フェンスブロック(`::table` / `::math` / `::figure`、sml-spec §6)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FenceBlock {
    pub fence_kind: FenceKind,
    pub id_tag: Option<IdTag>,
    /// フェンス内属性行(`[caption=...]` 等)。`::figure` は本体がこれのみで完結する。
    pub fence_attrs: Vec<AttrLine>,
    pub body: FenceBody,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FenceKind {
    Table,
    Math,
    Figure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FenceBody {
    Table(TableBody),
    /// TeX ソースのまま(遅延パース)。
    MathTex(Span),
    /// 属性行のみで完結(本体なし)。
    Figure,
}

// ---- 表(sml-spec §6.1、D4)----------------------------------------------------

/// `::table` 本体。次元木(行/列)とセルの列。
///
/// WP1/WP2 時点では `table::parse_table_body` がプレースホルダのため、常に
/// 空(rows/cols/cells が全て空)の値が入る。WP3 が実装を差し替える。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableBody {
    pub rows: Vec<DimNode>,
    pub cols: Vec<DimNode>,
    pub cells: Vec<CellEntry>,
}

/// 次元(例: `model`)。直下の member 列を持つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimNode {
    pub name: String,
    pub span: Span,
    pub members: Vec<MemberNode>,
}

/// 次元の要素。`children` が空なら葉、非空なら入れ子次元。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberNode {
    pub key: String,
    /// 表示名(`- key "表示名"`、sml-spec §6.1 の初版案)。
    pub label: Option<String>,
    pub span: Span,
    pub children: Vec<DimNode>,
}

/// セル行 `path | path : value`(sml-spec §7 の座標文法)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellEntry {
    pub row_path: Vec<String>,
    pub col_path: Vec<String>,
    pub value: CellRaw,
    pub span: Span,
}

/// セル値の型付きパース結果(D4)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CellRaw {
    Number(f64),
    Quantity { v: f64, unit: String },
    Text(String),
    Ref(RefTarget),
    Empty,
}
