//! SML-AST — スパン付き構文木(sml-parser-design.md §4、sml-spec.md 準拠)。
//!
//! 設計上の非対称に注意: **ULID か人間ラベルかは AST が区別する**(fmt が注入対象を
//! 列挙するため)が、**ラベル→ULID の解決は AST では行わない**(エイリアス表の構築は
//! build の仕事。sml-spec §3.4)。`RefTarget` はこの非対称をそのまま型にしたもの。
//!
//! このモジュールは M1 の全 WP(WP1〜WP4)が共有する唯一の型定義の場所。
//! `inline.rs` / `table.rs` はここに定義された型を消費するだけで、
//! 型そのものを追加・変更する必要が無いことを意図している。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use ulid::Ulid;

use crate::span::Span;

/// 参照スタイルリンクの定義表(M6 D40)。正規化ラベル(trim + lowercase)→ url のスパン。
/// 定義行は非可視メタ(グラフに段落ノードを作らない)なので、値そのもの(文字列)
/// ではなくソース中の url の位置(Span)を持たせ、他のインライン参照と同じく
/// ゼロコピーで扱えるようにする。
pub(crate) type RefDefs = HashMap<String, Span>;

// ---- ドキュメント / ブロック -------------------------------------------------

/// ドキュメント全体。SML ファイル1つ = `SmlDocument` 1つ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmlDocument {
    pub blocks: Vec<SmlBlock>,
    /// 元テキストの全長(バイト)。スパン被覆不変条件の検証に使う。
    pub src_len: usize,
    /// ファイル先頭(オフセット0)の `---` フロントマター(sml-spec §2.1、D12)。
    /// 無ければ `None`(フォレスト。Document ノードは build が作らない)。
    pub frontmatter: Option<Frontmatter>,
}

/// フロントマター(sml-spec §2.1)。ファイル先頭オフセット0の `---` 単独行から
/// 次の `---` 単独行までを YAML 風の `key: value` 行として読む。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frontmatter {
    /// フロントマター全体のスパン(開き `---` 行の先頭 = 0 から、閉じ `---` 行の末尾
    /// (改行を含む)まで)。閉じが無ければファイル末尾まで(`UnclosedFrontmatter`)。
    pub span: Span,
    /// 開き `---` 単独行の内容スパン(改行を含まない)。fmt が「id 行の挿入位置 =
    /// 開き `---` 行の直後」を計算するために使う(挿入位置は `open_span.end` の次の
    /// 改行の直後)。
    pub open_span: Span,
    /// `id: <値>` の解決前の値と、その値トークンのスパン(診断位置に使う)。
    /// 値が ULID でなければ `BadIdValue`(sml-spec §2.1、フロントマターにラベル/alias
    /// の置換系は持ち込まない)。キー自体が無ければ `None`。
    pub id: Option<(RefTarget, Span)>,
    /// `title: <値>` の生文字列。キーが無ければ `None`。
    pub title: Option<String>,
    /// `alias: <値>` の生文字列とその値トークンのスパン(D41、sml-spec §2.1)。
    /// 文書エイリアス — ワークスペース横断参照 `ref:<文書alias>/<...>` の左辺になる。
    /// キーが無ければ `None`。字句は他の alias と同じ `[A-Za-z0-9_-]+`
    /// (不正なら `BadKeyCharset`)。
    pub alias: Option<(String, Span)>,
    /// 閉じ `---` 単独行の内容スパン(改行を含まない)。ファイル末尾まで閉じが
    /// 見つからなかった場合はファイル末尾の空スパン(`UnclosedFrontmatter` と併発)。
    pub close_span: Span,
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
    List {
        ordered: bool,
        items: Vec<ListItem>,
        /// 順序リストの開始値(M6 D40)。`5. fifth` のように1以外から始まる場合に保持。
        start: Option<u64>,
    },
    /// `::table` / `::math` / `::figure`。
    Fence(FenceBlock),
    /// コードフェンス(```` ```lang ````)。開始行末尾に `{#id}` を書ける(D10、
    /// 2026-07-14 改定。行型ブロックとして扱う — sml-spec §2)。
    CodeFence { lang: String, body: Span, id_tag: Option<IdTag> },
    /// 参照スタイルリンクの定義行(M6 D40)。`[label]: url "title"`。非可視メタ
    /// (build はノードを作らない)。
    LinkRefDef { label: String, url: Span, title: Option<Span> },
    /// blockquote(`>` 行群、M6 D40)。子ブロックは行頭 `> ` を除去した上での
    /// 再帰パース結果(v0 は1段。ネスト引用は裁量、最終報告参照)。
    Quote { blocks: Vec<SmlBlock> },
    /// 水平線(単独行 `---`/`***`/`___`、M6 D40)。
    ThematicBreak,
    /// GFM パイプ表(M6 D40 Tier2)。フラット2次元へブリッジする(header セル =
    /// 列 member label、行 key は自動採番)。
    GfmTable(GfmTableBody),
}

/// GFM パイプ表の本体(M6 D40)。ヘッダ行 + 区切り行 + データ行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GfmTableBody {
    /// ヘッダセル(各列のラベル。生テキストのスパン)。
    pub header: Vec<Span>,
    /// データ行。各行は列数ぶんの型付きセル値(D4 と同じ `CellRaw`。足りない列は
    /// `CellRaw::Empty`)。
    pub rows: Vec<Vec<CellRaw>>,
}

/// リスト項目。行末に自身の `{#id}` を持ちうる(行型ブロック、sml-spec §3.3)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    /// 項目行のスパン(IDタグを含む、行末改行は含まない)。
    pub span: Span,
    pub inline: Vec<SmlInline>,
    pub id_tag: Option<IdTag>,
    /// ネストした子リスト(D24、2026-07-14 裁定)。2スペース/レベルのインデントで
    /// 表現される。「項目=段落1つ」の制約は維持するため、子リストは項目内容とは
    /// 別に高々1つだけ持てる(項目内複数ブロックは引き続き保留 §10)。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child: Option<Box<ListBlock>>,
    /// GFM タスクリストのチェック状態(M6 D40)。`- [ ]`/`- [x]` の項目のみ `Some`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
}

/// ネストしたリスト1つ(D24)。トップレベルの `BlockKind::List` と同形。ネストした
/// リストは前置属性行を書ける場所が無い(親項目の行に埋め込まれるため)ため、ID/alias
/// を持たない — canonical では build が自動生成した ID を持つ `List` ノードになる。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListBlock {
    pub ordered: bool,
    pub items: Vec<ListItem>,
    pub start: Option<u64>,
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
    /// ワークスペース横断参照(D41/D42、sml-spec §1.10): `<文書alias>/<ブロックalias>`。
    /// `doc` は参照先文書の frontmatter alias、`alias` はその文書内のブロック alias
    /// (無修飾の alias と同じ字句・同じ解決規則で、文書をまたぐだけ)。
    /// **id タグ(宣言側)・フロントマターの `id:` では決して生成されない** — この
    /// variant を作るのは参照側のパーサ(inline.rs の `resolve_target` /
    /// value.rs 経由の `block::parse_scoped_ref_target`)だけ。単一ファイル build
    /// (`--workspace` 無し)でこの variant に遭遇したら `BuildError::CrossDocRef`
    /// (専用の案内メッセージ)を返す(WP-W1.3)。
    DocLabel { doc: String, alias: String },
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
    /// バックスラッシュエスケープされた1文字(M6 D40)。`span` は `\` を含む2バイト
    /// (`\` + ASCII 記号1文字)。build はこの span の2バイト目だけを Text にする。
    Escaped(Span),
    /// 外部リンク(`[text](https://…)` / autolink `<https://…>`、M6 D40)。
    Link { url: Span, text: Span },
    /// インライン画像(`![alt](url)`、M6 D40)。
    Image { url: Span, alt: Span },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmphKind {
    Strong,
    Em,
    Code,
    /// `~~取消線~~`(M6 D40 Tier2)。
    Strike,
}

/// インライン参照のスキーム(sml-spec §5.2)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefScheme {
    Ref,
    Table,
    Fig,
    Math,
    Cell,
    /// `doc:<文書alias>`(または ULID)— Document ノードを直接指す(D53、
    /// sml-spec.md §1.14)。target は他スキームと違い `<doc>/<alias>` のスラッシュ修飾を
    /// 取らない(文書そのものを指すので「文書内のブロック」という第2階層が無い)。
    Doc,
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
    /// key-value ブロック(D28、sml-spec §1.5・§6)。
    Record,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FenceBody {
    Table(TableBody),
    /// TeX ソースのまま(遅延パース)。
    MathTex(Span),
    /// 属性行のみで完結(本体なし)。
    Figure,
    /// `::record` 本体(D28)。
    Record(RecordBody),
}

// ---- record(sml-spec §1.5、D28)------------------------------------------------

/// `::record` 本体。「キー: 値」の行の順序保存列(D28)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordBody {
    pub entries: Vec<RecordEntry>,
}

/// record の1エントリ。キーは自由テキスト(日本語可、表の座標キーとは別物で
/// パス構文に入らない)。値は表セルと同じ型付きパース(`CellRaw`)を共有する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordEntry {
    pub key: String,
    pub value: CellRaw,
    pub span: Span,
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

/// セル値 / record 値の型付きパース結果(D4 + D29)。表セルと record 値で共通。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CellRaw {
    Number(f64),
    Quantity { v: f64, unit: String },
    Text(String),
    Ref(RefTarget),
    Empty,
    /// 日付(D29)。既定は ISO(`YYYY-MM-DD` / `YYYY-MM`)のみ。フェンス属性
    /// `date-format=` が宣言されていれば、その追加書式も受理する。
    Date(DateRaw),
    /// 期間(D29)。「A 〜 B」「A 〜 現在」(`〜`/`~` 両可)。`to` 無しは「現在」。
    Period { from: DateRaw, to: Option<DateRaw> },
}

/// 年月(日)。`d` 無しは「月までの精度」(D29)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DateRaw {
    pub y: i32,
    pub m: u32,
    pub d: Option<u32>,
}
