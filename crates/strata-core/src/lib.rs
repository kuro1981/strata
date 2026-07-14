//! strata-core — Strata canonical (層2) のスキーマ凍結
//!
//! これは strata-core クレートの `lib.rs` 想定。パーサ(オーサリング→canonical)、
//! ストア(SQLite/グラフDB)、レンダラ(canonical→HTML/Typst/音声)は、
//! このクレートに依存する別クレートにする。
//!
//! 対応する仕様節:
//!   §2  コアデータモデル (Node / Edge / Inline)
//!   §2.4 粒度: レベル1既定 + anchor 昇格
//!   §4  エッジ関係カタログ (Rel)
//!   §5  表 = 次元の木 (Table / DimTree)
//!   §6  数式 = MathML Presentation サブセット (MathNode)
//!   §7  図 (Figure)
//!
//! 設計の要:
//!   - `type` フィールドは enum のバリアントが担う(別建ての type 文字列は持たない)。
//!   - **物理レイアウト語(改ページ/段組/列幅/見栄え改行)はどの payload 型にも存在しない**。
//!     → 不変条件2(軸6排除)を「書ける場所が無い」=型で強制(コンパイル時保証)。
//!   - インライン内容は payload(Vec<Inline>)。子ノードは payload に埋めず Edge(Contains) で持つ。
//!   - 相互参照はインラインに標的IDを直接持たせ、同時に Edge を materialise する(§2.3)。
//!
//! Cargo.toml(目安):
//!   serde = { version = "1", features = ["derive"] }
//!   serde_json = "1"
//!   ulid = { version = "1", features = ["serde"] }

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use ulid::Ulid;

// 識別子

/// 安定・不変のノードID(不変条件1)。内容を編集してもIDは変わらない。
/// ULID を採用。確定時に content-hash leaf を別途発行する案は §14B 参照。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub Ulid);

impl NodeId {
    pub fn new() -> Self {
        NodeId(Ulid::new())
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

// ノード

/// canonical の一級レコード。`{ id, <type固有フィールド…> }` にシリアライズされる。
/// 例: {"id":"01J...","type":"para","inline":[...]}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    /// 出し分けの意味分類タグ(D23、sml-spec §1.4)。「誰に見せるか」はビュー側
    /// (`render --hide <class>`)の仕事で、ここには「何であるか」だけを置く。
    /// build はグラフ構造・成否ともに class に非依存(全ノードを常に格納する)。
    /// 後方互換フィールド(空なら旧形式 JSON と同じ形にシリアライズされる)。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub classes: Vec<String>,
    #[serde(flatten)]
    pub payload: NodePayload,
}

impl Node {
    /// class 無しでノードを作る通常コンストラクタ。
    pub fn new(id: NodeId, payload: NodePayload) -> Self {
        Node { id, classes: Vec::new(), payload }
    }
}

/// ノード型カタログ(§3)。物理レイアウトのバリアントは存在しない。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NodePayload {
    Section(Section),
    Para(Para),
    List(List),
    Table(Table),
    Math(MathBlock),
    Figure(Figure),
    Code(Code),
    Term(Term),
    /// §2.4: 段落より細かいスパンを昇格させたノード。被参照・トランスクルージョン可。
    Anchor(Anchor),
    Value(Value),
    /// フロントマターに対応する文書ルート(D12)。トップレベルブロックを contains する。
    Document(Document),
}

/// フロントマターに対応する文書ルートノード(§9-5)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Section {
    pub heading: Vec<Inline>,
    // 子は contains エッジで持つ(payload には埋めない)。
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Para {
    pub inline: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct List {
    pub ordered: bool,
    // 各項目は contains された para / section。
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Code {
    pub lang: String,
    pub src: String,
}

/// 概念の宣言。定義は `defines` エッジ(定義ブロック → term)で結ぶ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Term {
    pub name: String,
}

/// 昇格したスパン本体(§2.4)。para 側は Inline::Anchor で位置のみ示す。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anchor {
    pub inline: Vec<Inline>,
}

/// 参照可能な単一値。prose と表で同じ値を共有する用途(任意)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Value {
    pub scalar: Scalar,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Scalar {
    Number(f64),
    Text(String),
    Bool(bool),
}

// インライン AST(§3)

/// 「読むための内容」。{"t":"text","s":"…"} の形にシリアライズされる。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum Inline {
    Text {
        s: String,
    },
    Emph {
        kind: EmphKind,
        children: Vec<Inline>,
    },
    /// インライン数式(再帰木)。
    Math {
        tree: MathNode,
    },
    /// 相互参照。標的IDを直接保持し、別途 Edge を materialise する。
    Ref {
        to: NodeId,
        rel: Rel,
        /// セル参照(`cell:`)の座標。他の参照種別では None(§5.3, §9-2)。
        #[serde(default, skip_serializing_if = "Option::is_none")]
        coord: Option<CellCoord>,
        /// 表示テキスト(ロスレス原則。§9-7)。
        #[serde(default, skip_serializing_if = "String::is_empty")]
        text: String,
    },
    /// 用語使用 → term ノードへのリンク。
    Term {
        to: NodeId,
        /// 表示テキスト(ロスレス原則。§9-7)。
        #[serde(default, skip_serializing_if = "String::is_empty")]
        text: String,
    },
    /// 昇格したスパンの位置。中身は Anchor ノード側(§2.4)。
    Anchor {
        to: NodeId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EmphKind {
    Strong,
    Em,
    Code,
}

// エッジ(§2, §4)

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub rel: Rel,
    /// contains の子順序づけ用。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub ord: Option<u32>,
}

/// エッジ関係カタログ(§4)。"term-ref" / "depends-on" 等で出力。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Rel {
    /// 構造の背骨(順序つき)。複数親可・閉路不可の DAG(不変条件6)。
    Contains,
    /// 定義ブロック → term。
    Defines,
    /// 使用ブロック/インライン → term。
    TermRef,
    /// 論拠・根拠(DAG)。
    Supports,
    /// 依存(補題→定義 等, DAG)。
    DependsOn,
    /// 引用 → 出典ノード。
    Cites,
    /// 型付け(任意/将来)。
    InstanceOf,
    /// ナビゲーション弱参照(§5.2, §9-1)。インライン `Ref` が materialise する。
    RefersTo,
}

// 表 = 次元の木(§5)
//
// 格子+colspan を持たない。行/列の「次元の木」とセルだけで持つ。
// 「結合された見出しセル」= children を持つ Member(=内部ノード)。span は render 時に
// 葉の数から計算される派生物であり、ここには現れない(軸6排除)。

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Table {
    pub rows: DimTree,
    pub cols: DimTree,
    pub cells: Vec<Cell>,
    /// 表のキャプション(sml-spec §6.1 の `[caption=...]`。D16、後方互換フィールド)。
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub caption: Option<Vec<Inline>>,
}

pub type DimTree = Vec<Dim>;

/// 束ねる概念(例: "year")。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dim {
    pub name: String,
    pub members: Vec<Member>,
}

/// 次元の要素。children が空なら葉、非空なら入れ子次元(=従来の「結合」)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub label: Option<Vec<Inline>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub children: DimTree,
}

/// セル。座標は (行の葉までの member key 列) × (列の葉までの member key 列)。
/// JSON のマップキーに構造体を使えないため Vec<Cell> で保持し、ルックアップ表は
/// ロード時に構築する(§10 シリアライズ方針)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Cell {
    pub row_path: Vec<String>,
    pub col_path: Vec<String>,
    pub value: CellValue,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "k", rename_all = "snake_case")]
pub enum CellValue {
    Number { v: f64 },
    Text { v: String },
    /// value ノード参照(prose と値を共有する場合)。
    Ref { to: NodeId },
    Empty,
    /// 数量(数値+単位)。SML の型付きパース規則(D4)の canonical 表現(§9-3)。
    Quantity { v: f64, unit: String },
}

/// セル参照(`cell:`)の座標(§5.3, §9-2)。
/// strata-sml の同名型とは別物(両クレートは依存しないため重複定義でよい)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CellCoord {
    pub row_path: Vec<String>,
    pub col_path: Vec<String>,
}

// 数式 = MathML サブセット(§6)
//
// canonical は MathML Presentation のサブセット。人は TeX で書き、パーサが木へ写す。
// バリアント名 ≒ MathML 要素: Num=mn, Ident=mi, Op=mo, Row=mrow, Frac=mfrac,
// Sup=msup, Sub=msub, SubSup=msubsup, UnderOver=munderover, Sqrt=msqrt, Root=mroot,
// Fenced=mrow+mo(括弧), Text=mtext。必要な要素は随時追加する。

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MathBlock {
    pub tree: MathNode,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MathNode {
    Num { v: String },
    Ident { v: String },
    Op { v: String },
    Row { items: Vec<MathNode> },
    Frac { num: Box<MathNode>, den: Box<MathNode> },
    Sup { base: Box<MathNode>, sup: Box<MathNode> },
    Sub { base: Box<MathNode>, sub: Box<MathNode> },
    SubSup { base: Box<MathNode>, sub: Box<MathNode>, sup: Box<MathNode> },
    UnderOver {
        base: Box<MathNode>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        under: Option<Box<MathNode>>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        over: Option<Box<MathNode>>,
    },
    Sqrt { body: Box<MathNode> },
    Root { radicand: Box<MathNode>, index: Box<MathNode> },
    Fenced { open: String, close: String, body: Box<MathNode> },
    Text { s: String },
}

// 図(§7)

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Figure {
    /// 記号図: データを焼かない。既存の table/value ノードを data_ref で再利用。
    Chart(Chart),
    /// 写真: 不透明な外部実体 + 構造化記述。
    Image(ImageFigure),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Chart {
    pub data_ref: NodeId,
    pub mark: Mark,
    pub encode: Encoding,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub caption: Option<Vec<Inline>>,
    /// 構造化記述(sml-spec §6.3 の `[depicts=...]` / `[depicts.<key>=...]`。D16、
    /// `ImageFigure.depicts` と同形・同じキー畳み規則: 裸の `depicts` は
    /// `"description"` キー、`depicts.<key>` はその `<key>` をキーにする)。
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub depicts: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mark {
    Line,
    Bar,
    Point,
    Area,
}

/// x/y/color は table の次元名(Dim.name)または列キーを指す。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Encoding {
    pub x: String,
    pub y: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub color: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageFigure {
    /// 外部アセット URI(asset://… 等)。
    pub src: String,
    pub alt: String,
    /// 構造化記述(単なる alt ではない外付け意味層)。当面は自由なキー値。
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub depicts: BTreeMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub caption: Option<Vec<Inline>>,
}

// グラフ(in-memory)

/// ノードの海とエッジの海。これが真実の源(ファイルは射影)。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Graph {
    pub nodes: BTreeMap<NodeId, Node>,
    pub edges: Vec<Edge>,
}

impl Graph {
    pub fn insert(&mut self, node: Node) {
        self.nodes.insert(node.id, node);
    }

    pub fn link(&mut self, from: NodeId, rel: Rel, to: NodeId, ord: Option<u32>) {
        self.edges.push(Edge { from, to, rel, ord });
    }

    /// contains の子を ord 昇順で返す。
    pub fn children_of(&self, parent: NodeId) -> Vec<NodeId> {
        let mut kids: Vec<(&Edge,)> = self
            .edges
            .iter()
            .filter(|e| e.rel == Rel::Contains && e.from == parent)
            .map(|e| (e,))
            .collect();
        kids.sort_by_key(|(e,)| e.ord.unwrap_or(u32::MAX));
        kids.into_iter().map(|(e,)| e.to).collect()
    }
}

// 不変条件チェック(§1)
//
// 型で守れるもの(不変条件2 物理排除)は型で守った。残りは実行時に検証する。

pub mod invariants {
    use super::*;
    use std::collections::HashSet;

    #[derive(Debug, Clone, PartialEq)]
    pub enum Violation {
        /// エッジの from/to が存在しないノードを指す。
        DanglingEdge { edge: Edge },
        /// contains に閉路がある(不変条件6 違反)。
        ContainsCycle { at: NodeId },
    }

    /// 安価な構造検証。指示語禁止(不変条件3)は型では守れない(`Inline::Text` は
    /// 自由文字列なので「下の表」と書ける)。禁止フレーズ検出は将来の lint 層の仕事(§1.1)。
    /// 複数親(トランスクルージョン)は許す(不変条件5)ので検査しない。
    pub fn validate(g: &Graph) -> Vec<Violation> {
        let mut v = Vec::new();

        // ダングリングエッジ
        for e in &g.edges {
            if !g.nodes.contains_key(&e.from) || !g.nodes.contains_key(&e.to) {
                v.push(Violation::DanglingEdge { edge: e.clone() });
            }
        }

        // contains 閉路(DFS)。複数親は可だが閉路は不可。
        let mut color: BTreeMap<NodeId, u8> = BTreeMap::new(); // 0=未訪問,1=訪問中,2=完了
        let mut stack: Vec<(NodeId, bool)> = Vec::new();
        let roots: Vec<NodeId> = g.nodes.keys().copied().collect();
        for r in roots {
            if color.get(&r).copied().unwrap_or(0) != 0 {
                continue;
            }
            stack.push((r, false));
            let mut on_path: HashSet<NodeId> = HashSet::new();
            while let Some((n, leaving)) = stack.pop() {
                if leaving {
                    color.insert(n, 2);
                    on_path.remove(&n);
                    continue;
                }
                if color.get(&n).copied().unwrap_or(0) == 1 {
                    continue;
                }
                color.insert(n, 1);
                on_path.insert(n);
                stack.push((n, true));
                for child in g.children_of(n) {
                    if on_path.contains(&child) {
                        v.push(Violation::ContainsCycle { at: child });
                    } else if color.get(&child).copied().unwrap_or(0) == 0 {
                        stack.push((child, false));
                    }
                }
            }
        }

        v
    }
}

// 健全性テスト(§5/§6 を実際に表現できるか)

#[cfg(test)]
mod tests {
    use super::*;

    /// §5 の2階建てヘッダ表が次元の木で表せること。
    #[test]
    fn revenue_table_roundtrips() {
        let table = Table {
            rows: vec![Dim {
                name: "metric".into(),
                members: vec![
                    Member { key: "Revenue".into(), label: None, children: vec![] },
                    Member { key: "Cost".into(), label: None, children: vec![] },
                ],
            }],
            cols: vec![Dim {
                name: "year".into(),
                members: vec![
                    Member {
                        key: "2025".into(),
                        label: None,
                        children: vec![Dim {
                            name: "quarter".into(),
                            members: vec![
                                Member { key: "Q1".into(), label: None, children: vec![] },
                                Member { key: "Q2".into(), label: None, children: vec![] },
                            ],
                        }],
                    },
                    Member {
                        key: "2026".into(),
                        label: None,
                        children: vec![Dim {
                            name: "quarter".into(),
                            members: vec![
                                Member { key: "Q1".into(), label: None, children: vec![] },
                                Member { key: "Q2".into(), label: None, children: vec![] },
                            ],
                        }],
                    },
                ],
            }],
            cells: vec![Cell {
                row_path: vec!["Revenue".into()],
                col_path: vec!["2025".into(), "Q1".into()],
                value: CellValue::Number { v: 100.0 },
            }],
            caption: None,
        };
        let node = Node::new(NodeId::new(), NodePayload::Table(table));
        let json = serde_json::to_string(&node).unwrap();
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(node, back);
        // "colspan" という語がどこにも現れないことの確認(軸6排除)。
        assert!(!json.contains("colspan"));
    }

    /// §6 の Σ_{i=1}^{n} x_i が MathML サブセット木で表せること。
    #[test]
    fn sum_math_roundtrips() {
        let sum = MathNode::Row {
            items: vec![
                MathNode::UnderOver {
                    base: Box::new(MathNode::Op { v: "∑".into() }),
                    under: Some(Box::new(MathNode::Row {
                        items: vec![
                            MathNode::Ident { v: "i".into() },
                            MathNode::Op { v: "=".into() },
                            MathNode::Num { v: "1".into() },
                        ],
                    })),
                    over: Some(Box::new(MathNode::Ident { v: "n".into() })),
                },
                MathNode::Sub {
                    base: Box::new(MathNode::Ident { v: "x".into() }),
                    sub: Box::new(MathNode::Ident { v: "i".into() }),
                },
            ],
        };
        let node = Node::new(NodeId::new(), NodePayload::Math(MathBlock { tree: sum }));
        let json = serde_json::to_string(&node).unwrap();
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(node, back);
    }

    /// §2.3/§2.4: 段落の中で term を参照し、対応する term-ref エッジを materialise する。
    #[test]
    fn para_with_term_reference() {
        let term_id = NodeId::new();
        let para_id = NodeId::new();

        let mut g = Graph::default();
        g.insert(Node::new(term_id, NodePayload::Term(Term { name: "次元削減".into() })));
        g.insert(Node::new(
            para_id,
            NodePayload::Para(Para {
                inline: vec![
                    Inline::Text { s: "高次元データを".into() },
                    Inline::Term { to: term_id, text: String::new() },
                    Inline::Text { s: "で低次元へ写す。".into() },
                ],
            }),
        ));
        // インラインの Term に対応する Edge を materialise。
        g.link(para_id, Rel::TermRef, term_id, None);

        assert!(invariants::validate(&g).is_empty());
    }

    /// 不変条件6: contains の閉路を検出できる。
    #[test]
    fn detects_contains_cycle() {
        let a = NodeId::new();
        let b = NodeId::new();
        let mut g = Graph::default();
        g.insert(Node::new(a, NodePayload::Section(Section { heading: vec![] })));
        g.insert(Node::new(b, NodePayload::Section(Section { heading: vec![] })));
        g.link(a, Rel::Contains, b, Some(0));
        g.link(b, Rel::Contains, a, Some(0)); // 閉路
        assert!(matches!(
            invariants::validate(&g).as_slice(),
            [invariants::Violation::ContainsCycle { .. }, ..]
        ));
    }

    // --- WP-B3 (D-B2): 新フィールド/新バリアントの後方互換性・往復テスト ---

    /// 旧形式(coord/text 省略)の Inline::Ref JSON が読め、default 値で補完されること。
    #[test]
    fn inline_ref_deserializes_legacy_json_without_coord_or_text() {
        let to = NodeId::new();
        let legacy = format!(r#"{{"t":"ref","to":"{}","rel":"depends-on"}}"#, to.0);
        let parsed: Inline = serde_json::from_str(&legacy).unwrap();
        assert_eq!(parsed, Inline::Ref { to, rel: Rel::DependsOn, coord: None, text: String::new() });
    }

    /// 旧形式(text 省略)の Inline::Term JSON が読め、default 値で補完されること。
    #[test]
    fn inline_term_deserializes_legacy_json_without_text() {
        let to = NodeId::new();
        let legacy = format!(r#"{{"t":"term","to":"{}"}}"#, to.0);
        let parsed: Inline = serde_json::from_str(&legacy).unwrap();
        assert_eq!(parsed, Inline::Term { to, text: String::new() });
    }

    /// Inline::Ref の coord/text 付き往復。cell: 参照の座標保持(§9-2, §5.3)。
    #[test]
    fn inline_ref_with_coord_and_text_roundtrips() {
        let to = NodeId::new();
        let inline = Inline::Ref {
            to,
            rel: Rel::RefersTo,
            coord: Some(CellCoord {
                row_path: vec!["Opt-v2".into()],
                col_path: vec!["Dataset-A".into(), "Latency".into()],
            }),
            text: "12 ms".into(),
        };
        let json = serde_json::to_string(&inline).unwrap();
        let back: Inline = serde_json::from_str(&json).unwrap();
        assert_eq!(inline, back);
    }

    /// Inline::Term の text 付き往復(表示テキスト保持。§9-7)。
    #[test]
    fn inline_term_with_text_roundtrips() {
        let to = NodeId::new();
        let inline = Inline::Term { to, text: "予測精度".into() };
        let json = serde_json::to_string(&inline).unwrap();
        let back: Inline = serde_json::from_str(&json).unwrap();
        assert_eq!(inline, back);
    }

    /// Rel::RefersTo が "refers-to" にシリアライズされ、往復すること(§9-1)。
    #[test]
    fn rel_refers_to_serializes_as_kebab_case() {
        let json = serde_json::to_string(&Rel::RefersTo).unwrap();
        assert_eq!(json, "\"refers-to\"");
        let back: Rel = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Rel::RefersTo);
    }

    /// CellValue::Quantity(数量セル)の往復(§9-3, D4)。
    #[test]
    fn cell_value_quantity_roundtrips() {
        let cell = Cell {
            row_path: vec!["Opt-v2".into()],
            col_path: vec!["Dataset-A".into(), "Latency".into()],
            value: CellValue::Quantity { v: 12.0, unit: "ms".into() },
        };
        let json = serde_json::to_string(&cell).unwrap();
        let back: Cell = serde_json::from_str(&json).unwrap();
        assert_eq!(cell, back);
    }

    /// NodePayload::Document の往復。title あり/なしの両方(D12, §9-5)。
    #[test]
    fn document_node_roundtrips_with_and_without_title() {
        let with_title =
            Node::new(NodeId::new(), NodePayload::Document(Document { title: Some("設計メモ".into()) }));
        let json = serde_json::to_string(&with_title).unwrap();
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(with_title, back);

        let without_title =
            Node::new(NodeId::new(), NodePayload::Document(Document { title: None }));
        let json = serde_json::to_string(&without_title).unwrap();
        // title: None は skip_serializing_if で落ちること。
        assert!(!json.contains("title"));
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(without_title, back);
    }

    /// 旧形式(id/type のみ)の Document JSON が読め、title が None で補完されること。
    #[test]
    fn document_node_deserializes_legacy_json_without_title() {
        let id = NodeId::new();
        let legacy = format!(r#"{{"id":"{}","type":"document"}}"#, id.0);
        let parsed: Node = serde_json::from_str(&legacy).unwrap();
        assert_eq!(parsed, Node::new(id, NodePayload::Document(Document { title: None })));
    }

    // --- D16(2026-07-14): Table.caption / Chart.depicts の後方互換性 -----------------

    /// 旧形式(caption 省略)の Table JSON が読め、caption が None で補完されること。
    #[test]
    fn table_deserializes_legacy_json_without_caption() {
        let legacy = r#"{"rows":[],"cols":[],"cells":[]}"#;
        let parsed: Table = serde_json::from_str(legacy).unwrap();
        assert_eq!(parsed, Table { rows: vec![], cols: vec![], cells: vec![], caption: None });
    }

    /// Table.caption ありの往復。caption なしなら "caption" が出力に現れないこと。
    #[test]
    fn table_caption_roundtrips_and_is_omitted_when_none() {
        let with_caption = Table {
            rows: vec![],
            cols: vec![],
            cells: vec![],
            caption: Some(vec![Inline::Text { s: "表のキャプション".into() }]),
        };
        let json = serde_json::to_string(&with_caption).unwrap();
        let back: Table = serde_json::from_str(&json).unwrap();
        assert_eq!(with_caption, back);

        let without_caption = Table { rows: vec![], cols: vec![], cells: vec![], caption: None };
        let json = serde_json::to_string(&without_caption).unwrap();
        assert!(!json.contains("caption"));
    }

    /// 旧形式(depicts 省略)の Chart JSON が読め、depicts が空 map で補完されること。
    #[test]
    fn chart_deserializes_legacy_json_without_depicts() {
        let to = NodeId::new();
        let legacy =
            format!(r#"{{"data_ref":"{}","mark":"bar","encode":{{"x":"a","y":"b"}}}}"#, to.0);
        let parsed: Chart = serde_json::from_str(&legacy).unwrap();
        assert_eq!(
            parsed,
            Chart { data_ref: to, mark: Mark::Bar, encode: Encoding { x: "a".into(), y: "b".into(), color: None }, caption: None, depicts: BTreeMap::new() }
        );
    }

    /// Chart.depicts の往復。ImageFigure.depicts と同じ畳み規則を型レベルで共有できること。
    #[test]
    fn chart_depicts_roundtrips_and_is_omitted_when_empty() {
        let to = NodeId::new();
        let mut depicts = BTreeMap::new();
        depicts.insert("description".to_string(), "棒グラフ".to_string());
        let chart = Chart {
            data_ref: to,
            mark: Mark::Bar,
            encode: Encoding { x: "a".into(), y: "b".into(), color: None },
            caption: None,
            depicts,
        };
        let json = serde_json::to_string(&chart).unwrap();
        let back: Chart = serde_json::from_str(&json).unwrap();
        assert_eq!(chart, back);

        let empty = Chart {
            data_ref: to,
            mark: Mark::Bar,
            encode: Encoding { x: "a".into(), y: "b".into(), color: None },
            caption: None,
            depicts: BTreeMap::new(),
        };
        let json = serde_json::to_string(&empty).unwrap();
        assert!(!json.contains("depicts"));
    }

    // --- D23(2026-07-14): Node.classes の後方互換性 ---------------------------------

    /// 旧形式(classes フィールド自体が無い)の Node JSON が読め、classes が空 Vec で
    /// 補完されること(既存ゴールデン JSON を壊さないための後方互換、D23)。
    #[test]
    fn node_deserializes_legacy_json_without_classes() {
        let id = NodeId::new();
        let legacy = format!(r#"{{"id":"{}","type":"document"}}"#, id.0);
        let parsed: Node = serde_json::from_str(&legacy).unwrap();
        assert_eq!(parsed, Node::new(id, NodePayload::Document(Document { title: None })));
        assert!(parsed.classes.is_empty());
    }

    /// classes ありの往復。classes が空なら "classes" が出力に現れないこと
    /// (skip_serializing_if による後方互換の維持)。
    #[test]
    fn node_classes_roundtrip_and_omitted_when_empty() {
        let with_classes = Node {
            id: NodeId::new(),
            classes: vec!["note".to_string(), "actual-name".to_string()],
            payload: NodePayload::Para(Para { inline: vec![] }),
        };
        let json = serde_json::to_string(&with_classes).unwrap();
        assert!(json.contains(r#""classes":["note","actual-name"]"#), "{json}");
        let back: Node = serde_json::from_str(&json).unwrap();
        assert_eq!(with_classes, back);

        let without_classes = Node::new(NodeId::new(), NodePayload::Para(Para { inline: vec![] }));
        let json = serde_json::to_string(&without_classes).unwrap();
        assert!(!json.contains("classes"), "{json}");
    }
}