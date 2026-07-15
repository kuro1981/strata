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

use strata_build::{BuildOutput, WorkspaceBuildOutput};
use strata_core::{Graph, NodeId, NodePayload, Rel};

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
    /// D44: `context --workspace --doc <alias>` に指定された文書 alias が、
    /// ワークスペースのどのメンバーの frontmatter alias にも一致しない。
    UnknownDoc(String),
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
            ContextError::UnknownDoc(alias) => {
                write!(f, "文書 alias '{alias}' を持つメンバーがワークスペースにありません。")
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

/// 公開 API: `WorkspaceBuildOutput` から AI 向けコンテキスト Markdown を生成する
/// (D44、sml-spec.md §1.11)。`--node`/`--class`/`--node`+`--class` の3スコープは
/// 統合グラフ上でそのまま動く(文書境界を跨いで辿れる。位置文脈のパス
/// (`ancestor_path`)は各ノードの所属文書の `Document.title` を自然に含む)ため、
/// 単一文書版の `render::render_node_scope`/`render_class_scope`/
/// `render_node_and_class_scope` をそのまま再利用する。無指定(全文書スコープ)だけ
/// ワークスペース専用の描画(`render::render_workspace_document_scope`)を使う —
/// `doc` で1文書に絞り込める(裁量、D44「--doc での絞り込みも可」)。
pub fn render_context_workspace(
    ws: &WorkspaceBuildOutput,
    opts: &ContextOptions,
    doc: Option<&str>,
) -> Result<String, ContextError> {
    let graph = &ws.graph;
    let scope = resolve_scope(graph, opts)?;

    match scope {
        Scope::Document => {
            let roots = resolve_doc_roots(ws, doc)?;
            let all_doc_owned: HashSet<NodeId> = strata_build::doc_ownership(graph, &ws.roots).into_keys().collect();
            render::render_workspace_document_scope(graph, &roots, &all_doc_owned)
        }
        Scope::Node { ids } => render::render_node_scope(graph, &ids, opts.hops),
        // 裁量(D44): class/node+class スコープは元々ワークスペース全体(統合グラフ)を
        // 対象にした横断検索であり、`--doc` による絞り込みは実装しない(単一文書版と
        // 完全に同じ経路を再利用する。最終報告参照)。
        Scope::Class { tag } => render::render_class_scope(graph, &tag, None),
        Scope::NodeAndClass { ids, tag } => render::render_node_and_class_scope(graph, &ids, &tag),
    }
}

/// `--doc` の解決: `Some(alias)` ならその frontmatter alias を持つメンバー1件に絞る、
/// `None` なら Document ルートを持つ全メンバーを `DocRoot::path` 昇順(`ws.roots` は
/// build 時点で既にソート済み)で返す。
fn resolve_doc_roots(ws: &WorkspaceBuildOutput, doc: Option<&str>) -> Result<Vec<(String, NodeId)>, ContextError> {
    match doc {
        Some(alias) => {
            let r = ws
                .roots
                .iter()
                .find(|r| r.alias.as_deref() == Some(alias))
                .ok_or_else(|| ContextError::UnknownDoc(alias.to_string()))?;
            let root = r.root.ok_or(ContextError::NoRoot)?;
            Ok(vec![(r.path.clone(), root)])
        }
        None => {
            let out: Vec<(String, NodeId)> =
                ws.roots.iter().filter_map(|r| r.root.map(|root| (r.path.clone(), root))).collect();
            if out.is_empty() {
                return Err(ContextError::NoRoot);
            }
            Ok(out)
        }
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

/// D46(sml-spec.md §1.11): `--class <tag>` の対象ノードを、**実効 class**
/// (自身+祖先(contains 上流)の和集合、`strata_core::has_effective_class`)で判定し、
/// かつコンテナ(見出し・リスト・引用等)が該当する場合は子を重複列挙しないよう
/// 「chunk の根」だけを返す。
///
/// アルゴリズム: (1) `scope`(`--node` と併用時はその contains サブツリー、
/// 無指定なら全ノード)の中から実効 class が `tag` を含むノードを集める(`matched`)、
/// (2) `matched` のうち「`contains` 上流の親が `matched` に含まれない」ものだけを
/// 「chunk の根」として残す(親が matched に含まれるなら、その親を chunk として
/// 展開した時点で自分もサブツリーとして出力されるため、二重に列挙しない)。
/// 親の判定は `scope` に絞った `matched` 集合で行う(裁量: `--node` 併用時、
/// scope の境界そのものを暗黙の「根」として扱う — 祖先が scope 外にいて matched で
/// あっても、そこは展開できないので scope 内の子が chunk の根になるのが自然)。
pub(crate) fn class_chunk_roots(
    graph: &Graph,
    tag: &str,
    parents: &HashMap<NodeId, NodeId>,
    scope: Option<&HashSet<NodeId>>,
) -> HashSet<NodeId> {
    let tags: HashSet<&str> = std::iter::once(tag).collect();
    let candidates: Vec<NodeId> = match scope {
        Some(s) => s.iter().copied().collect(),
        None => graph.nodes.keys().copied().collect(),
    };
    let matched: HashSet<NodeId> = candidates
        .into_iter()
        .filter(|&id| strata_core::has_effective_class(graph, parents, id, &tags))
        .collect();
    matched
        .iter()
        .copied()
        .filter(|id| match parents.get(id) {
            Some(p) => !matched.contains(p),
            None => true,
        })
        .collect()
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
