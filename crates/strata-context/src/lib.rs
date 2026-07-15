//! strata-context — canonical グラフ(層2)→ AI 向けコンテキストビュー(M5-A、D36)の直列化器。
//!
//! docs/context-m5a-handoff.md WP-A1/WP-A2 の実装。`strata_build::BuildOutput` を
//! 入力に取り、LLM が読みやすい「ULID 付き Markdown+意味エッジ一覧」を出力する。
//! 描画(strata-typst)とは違い、体裁ではなく**引用可能性**(alias/ULID でノードを
//! 名指しできること)が目的。
//!
//! ## スコープ3形態(D36)
//! 1. [`Scope::Document`] — 無指定。文書全体を見出し構造のまま直列化し、末尾に
//!    「エッジ」節で全意味エッジ(`contains` 以外)を一覧する。
//! 2. [`Scope::Node`] — `--node <alias|ULID>`(複数可)+ `--hops N`。指定ノードの
//!    `contains` サブツリーを chunk 本体として直列化し、意味エッジを N ホップ辿った先を
//!    「近傍」節に要約(サブツリー全展開はしない)。
//! 3. [`Scope::Class`] — `--class <tag>`。該当 class を持つブロックを文書順に横断
//!    列挙し、各項目に位置文脈(祖先見出しのパス)を1行付ける。
//!
//! `--node` と `--class` の併用(裁量、AND): 指定ノードの contains サブツリー内に
//! 収まる class 一致ブロックだけを Class 形式で列挙する(`--hops` は無視 —
//! 近傍という概念が class 横断列挙とは噛み合わないため)。

mod addr;
mod inline;
mod label;
mod render;

use std::collections::{BTreeMap, HashMap, HashSet};

use strata_build::BuildOutput;
use strata_core::{Graph, Node, NodeId, NodePayload, Rel};

pub use addr::resolve_node_ref;

/// `strata context` のスコープ指定。CLI 引数から組み立てる。
#[derive(Debug, Clone, Default)]
pub struct ContextOptions {
    /// `--node`(複数可)。alias または ULID 文字列。空なら Node スコープ無効。
    pub nodes: Vec<String>,
    /// `--hops`(既定 1、D36)。Node スコープでのみ使う。
    pub hops: u32,
    /// `--class`。Some なら Class スコープ(または Node との併用)。
    pub class: Option<String>,
}

/// スコープ解決後の内部表現。
enum Scope {
    Document,
    Node { ids: Vec<NodeId> },
    Class { tag: String },
    NodeAndClass { ids: Vec<NodeId>, tag: String },
}

/// `strata context` のエラー。CLI は `UnknownNodeRef` を exit 2 の明確なエラーとして扱う。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextError {
    /// `--node` に指定された alias/ULID がグラフに存在しない。
    UnknownNodeRef(String),
    /// フルドキュメントスコープだが `BuildOutput::root` が無い(フロントマター無し)。
    NoRoot,
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextError::UnknownNodeRef(s) => {
                write!(f, "'{s}' は alias にも ULID にも解決できません(グラフに存在しません)")
            }
            ContextError::NoRoot => {
                write!(f, "フロントマターがありません(root が None)。全文書スコープには Document ルートが必要です。")
            }
        }
    }
}

/// 公開 API: `BuildOutput` から AI 向けコンテキスト Markdown を生成する(D36)。
///
/// 決定的(同一入力→バイト同一): ノード列挙は常に `NodeId`(ULID の辞書順 = 生成順、
/// fixture では時系列)または明示された `--node` 引数順に従う。
pub fn render_context(build: &BuildOutput, opts: &ContextOptions) -> Result<String, ContextError> {
    let graph = &build.graph;
    let scope = resolve_scope(graph, opts)?;

    match scope {
        Scope::Document => {
            let root = build.root.ok_or(ContextError::NoRoot)?;
            render::render_document_scope(graph, root)
        }
        Scope::Node { ids } => render::render_node_scope(graph, &ids, opts.hops),
        Scope::Class { tag } => render::render_class_scope(graph, &tag, build.root),
        Scope::NodeAndClass { ids, tag } => render::render_node_and_class_scope(graph, &ids, &tag),
    }
}

fn resolve_scope(graph: &Graph, opts: &ContextOptions) -> Result<Scope, ContextError> {
    let node_ids: Vec<NodeId> = opts
        .nodes
        .iter()
        .map(|s| resolve_node_ref(graph, s).ok_or_else(|| ContextError::UnknownNodeRef(s.clone())))
        .collect::<Result<_, _>>()?;

    match (node_ids.is_empty(), &opts.class) {
        (true, None) => Ok(Scope::Document),
        (false, None) => Ok(Scope::Node { ids: node_ids }),
        (true, Some(tag)) => Ok(Scope::Class { tag: tag.clone() }),
        (false, Some(tag)) => Ok(Scope::NodeAndClass { ids: node_ids, tag: tag.clone() }),
    }
}

// --- 共有ユーティリティ(render.rs / addr.rs から使う内部型) -----------------------

/// `contains` エッジから「子 → 親」の逆引き表を1回だけ構築する(位置文脈の算出用)。
/// 複数親(トランスクルージョン)を持つノードは最初に現れたエッジを採用する
/// (`Graph::edges` の格納順は build が決定的に積むため、これも決定的)。
pub(crate) fn parent_index(graph: &Graph) -> HashMap<NodeId, NodeId> {
    let mut parents = HashMap::new();
    for e in &graph.edges {
        if e.rel == Rel::Contains {
            parents.entry(e.to).or_insert(e.from);
        }
    }
    parents
}

/// `id` の祖先のうち `Section`/`Document` である見出しテキストを根 → 葉の順に集めた
/// 「位置文脈パス」(例: `["職務経歴 詳細", "電通総研", "AI人材育成..."]`)。
/// `id` 自身は含まない。
pub(crate) fn ancestor_path(graph: &Graph, parents: &HashMap<NodeId, NodeId>, id: NodeId) -> Vec<String> {
    let mut chain = Vec::new();
    let mut cur = id;
    while let Some(&p) = parents.get(&cur) {
        if let Some(node) = graph.nodes.get(&p) {
            match &node.payload {
                NodePayload::Section(s) => chain.push(inline::plain_text(&s.heading)),
                NodePayload::Document(d) => {
                    if let Some(t) = &d.title {
                        chain.push(t.clone());
                    }
                }
                _ => {}
            }
        }
        cur = p;
    }
    chain.reverse();
    // D21 と同じフォールバックにより Document.title と最初の H1 が同文になりがち
    // (fixture・work_history.sml 双方で実際に発生)。位置文脈としては雑音なので、
    // 隣接する重複だけ畳む(裁量)。
    chain.dedup();
    chain
}

/// `contains` を辿って `root` のサブツリー(自身含む)の `NodeId` を文書順(ord 昇順、
/// 深さ優先)で集める。
pub(crate) fn subtree_ids(graph: &Graph, root: NodeId) -> Vec<NodeId> {
    fn walk(graph: &Graph, id: NodeId, out: &mut Vec<NodeId>) {
        out.push(id);
        for child in graph.children_of(id) {
            walk(graph, child, out);
        }
    }
    let mut out = Vec::new();
    walk(graph, root, &mut out);
    out
}

/// class(D23)を1つでも持つノードを文書順(`doc_order`、無ければ `NodeId` 昇順)で返す。
pub(crate) fn nodes_with_class<'g>(
    graph: &'g Graph,
    tag: &str,
    doc_order: Option<&'g [NodeId]>,
) -> Vec<(&'g NodeId, &'g Node)> {
    let matches: HashSet<NodeId> = graph
        .nodes
        .iter()
        .filter(|(_, n)| n.classes.iter().any(|c| c == tag))
        .map(|(id, _)| *id)
        .collect();

    match doc_order {
        Some(order) => order
            .iter()
            .filter(|id| matches.contains(id))
            .map(|id| (id, &graph.nodes[id]))
            .collect(),
        None => graph.nodes.iter().filter(|(id, _)| matches.contains(id)).collect(),
    }
}

/// 意味エッジ(`Rel::Contains` 以外の全種別)。エッジ種の選別パラメータは保留(D36)
/// なので、この判定はここ1箇所に集約する。
pub(crate) fn is_semantic(rel: Rel) -> bool {
    rel != Rel::Contains
}

/// 意味エッジを N ホップ辿った近傍ノード集合を計算する(D36 スコープ2)。
/// `seeds` はホップ0(chunk 本体側)のノード集合。戻り値は `(neighbor_id, [(rel, from_seed_side, hop)])`
/// — 同じ近傍に複数の関係が付きうるため、見つかった関係を全て集める。
/// `from_seed_side` は true なら「seed 側 → 近傍」(`Edge.from` が seed 側)、false なら逆向き。
pub(crate) fn semantic_neighbors(
    graph: &Graph,
    seeds: &HashSet<NodeId>,
    hops: u32,
) -> BTreeMap<NodeId, Vec<(Rel, bool)>> {
    let mut found: BTreeMap<NodeId, Vec<(Rel, bool)>> = BTreeMap::new();
    if hops == 0 {
        return found;
    }
    let mut frontier: HashSet<NodeId> = seeds.clone();
    let mut visited: HashSet<NodeId> = seeds.clone();

    for _ in 0..hops {
        let mut next_frontier: HashSet<NodeId> = HashSet::new();
        for e in &graph.edges {
            if !is_semantic(e.rel) {
                continue;
            }
            if frontier.contains(&e.from) && !seeds.contains(&e.to) {
                found.entry(e.to).or_default().push((e.rel, true));
                if visited.insert(e.to) {
                    next_frontier.insert(e.to);
                }
            }
            if frontier.contains(&e.to) && !seeds.contains(&e.from) {
                found.entry(e.from).or_default().push((e.rel, false));
                if visited.insert(e.from) {
                    next_frontier.insert(e.from);
                }
            }
        }
        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }

    // 同じ (rel, side) の重複を除く(複数ホップ/複数エッジで同じ関係が再発見されうる)。
    // `Rel` は `Hash` を derive していない(strata-core 側の凍結スキーマ、D1)ため
    // `HashSet` は使えず、件数が小さい(1ノードあたり数件)ことを前提に線形探索で十分。
    for rels in found.values_mut() {
        let mut unique: Vec<(Rel, bool)> = Vec::new();
        for r in rels.drain(..) {
            if !unique.contains(&r) {
                unique.push(r);
            }
        }
        *rels = unique;
    }
    found
}

/// `Rel` の kebab-case 表示名。strata-core の `#[serde(rename_all = "kebab-case")]` と
/// 同じ字句にする(serde_json への依存を避けるため手書きで揃える)。
pub(crate) fn rel_str(rel: Rel) -> &'static str {
    match rel {
        Rel::Contains => "contains",
        Rel::Defines => "defines",
        Rel::TermRef => "term-ref",
        Rel::Supports => "supports",
        Rel::DependsOn => "depends-on",
        Rel::Cites => "cites",
        Rel::InstanceOf => "instance-of",
        Rel::RefersTo => "refers-to",
    }
}

#[cfg(test)]
mod tests;
