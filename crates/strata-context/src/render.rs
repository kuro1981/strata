//! 3スコープ(D36)の Markdown 組み立て本体。

use std::collections::{HashMap, HashSet};

use strata_core::{Figure, Graph, NodeId, NodePayload};

use crate::addr::{address_tag, short_address};
use crate::inline::{render_inlines_md, render_math_text};
use crate::label::{cell_value_text, node_short_label};
use crate::{ContextError, ancestor_path, class_chunk_roots, is_semantic, rel_str, semantic_neighbors, subtree_ids};

// D46(sml-spec.md §1.11): `render_block` は常に `contains` を最後まで辿ってブロックの
// サブツリー全体を描く(旧 `RenderMode::Leaf`=子孫を描かない縮退モードは廃止)。
// `--class` スコープは `class_chunk_roots` が「実効 class が一致する chunk の根」だけを
// 選んだ上でこの関数に渡すため、根のサブツリーを丸ごと描画すれば子の重複列挙は
// 起きない(コンテナに1回 class を書けばよい、D46 の核心)。

// --- スコープ1: 全文書 -----------------------------------------------------------

pub(crate) fn render_document_scope(graph: &Graph, root: NodeId) -> Result<String, ContextError> {
    let mut out = String::new();
    render_document_body(graph, root, &mut out)?;

    // D36 形式要件: 全ブロックノードが ULID でアドレス可能。`contains` で document から
    // 到達できないノード(Term は用語使用側からしか辿れず、contains を持たない)を
    // 「その他のノード」として末尾に列挙し、抜け漏れを無くす。
    let visited: HashSet<NodeId> = subtree_ids(graph, root).into_iter().collect();
    render_orphans_and_edges(graph, &visited, &HashSet::new(), &mut out);
    Ok(out)
}

/// D44(sml-spec.md §1.11): `context --workspace` の無指定(全メンバー)/`--doc` 絞り込み
/// スコープ。各文書の本文を `---` 区切りで連結し、「その他のノード」「エッジ」節は
/// ワークスペース全体で1回だけ出す(`render_document_scope` を文書ごとに単純に
/// 繰り返すと、共有グラフの意味エッジ一覧が文書数ぶん重複してしまうため)。
/// `all_doc_owned`: ワークスペース全体(`--doc` で絞り込む前の全メンバー)の
/// `contains` サブツリーに属する全ノードの集合(`strata_build::doc_ownership` の
/// キー集合)。`--doc` で1文書に絞り込んだ場合、選ばれなかった他文書のノードは
/// 「その他のノード」(真の孤立ノード向けの節)に紛れ込ませない・意味エッジ一覧も
/// 選択文書に無関係な行を出さないための除外集合として使う(裁量: 単なる「全ノード
/// 走査」だと `--doc` の絞り込み効果がその2節だけ効かなくなり、混乱を招くため)。
pub(crate) fn render_workspace_document_scope(
    graph: &Graph,
    roots: &[(String, NodeId)],
    all_doc_owned: &HashSet<NodeId>,
) -> Result<String, ContextError> {
    let mut out = String::new();
    let mut visited: HashSet<NodeId> = HashSet::new();
    for (i, (path, root)) in roots.iter().enumerate() {
        if i > 0 {
            out.push_str("---\n\n");
        }
        // D44 裁量: 複数文書を1つの出力に連結する際、どの文書の本文かを見出しの前に
        // 明示する(単一文書スコープには無い要素だが、workspace スコープでは「chunk の
        // 位置文脈に文書名を含める」という WP-Z1 の要求に沿う)。
        out.push_str(&format!("_(文書ファイル: {path})_\n\n"));
        render_document_body(graph, *root, &mut out)?;
        visited.extend(subtree_ids(graph, *root));
    }
    let excluded: HashSet<NodeId> = all_doc_owned.difference(&visited).copied().collect();
    render_orphans_and_edges(graph, &visited, &excluded, &mut out);
    Ok(out)
}

/// 1文書の本文(タイトル見出し+`contains` サブツリー全体)を描画する。全文書
/// スコープ(`render_document_scope`)とワークスペース版
/// (`render_workspace_document_scope`)が共有する(D44 でのリファクタ抽出)。
fn render_document_body(graph: &Graph, root: NodeId, out: &mut String) -> Result<(), ContextError> {
    let root_node = graph.nodes.get(&root).ok_or(ContextError::NoRoot)?;
    // D21 と同じ3段フォールバック(Document.title → 最初の H1 → 無題)を踏襲する(裁量:
    // strata-typst の `first_h1_title` と同じ運用に揃え、タイトルの見え方を一致させる)。
    let title = match &root_node.payload {
        NodePayload::Document(d) => d.title.clone().unwrap_or_else(|| first_h1_title(graph, root).unwrap_or_else(|| "(無題)".to_string())),
        _ => node_short_label(graph, root),
    };
    out.push_str(&format!("# {} {}\n\n", title, address_tag(root, root_node)));

    for child in graph.children_of(root) {
        render_block(graph, child, 1, out);
    }
    Ok(())
}

/// 「その他のノード」節(D36 形式要件: 全ブロックが ULID アドレス可能)+「エッジ」節
/// (意味エッジ一覧)。`visited` は「本文にすでに現れたノード」の集合(単一文書なら
/// その文書の contains サブツリー、ワークスペースなら全文書分の和集合)。`excluded`
/// は「選択スコープ外の他文書のノード」(D44 `--doc` 絞り込み専用。単一文書
/// スコープでは常に空集合)— 「その他のノード」に他文書の内容を紛れ込ませず、
/// 「エッジ」も選択スコープに無関係な行(両端が excluded 側)を出さない。
fn render_orphans_and_edges(graph: &Graph, visited: &HashSet<NodeId>, excluded: &HashSet<NodeId>, out: &mut String) {
    let orphans: Vec<NodeId> =
        graph.nodes.keys().copied().filter(|id| !visited.contains(id) && !excluded.contains(id)).collect();
    if !orphans.is_empty() {
        out.push_str("## その他のノード\n\n");
        out.push_str("(本文の contains 構造には現れないが、意味エッジ経由で参照されうるノード。例: term)\n\n");
        for id in orphans {
            let n = &graph.nodes[&id];
            out.push_str(&format!("- {} {}\n", node_short_label(graph, id), address_tag(id, n)));
        }
        out.push('\n');
    }

    out.push_str("## エッジ\n\n");
    let semantic_edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| is_semantic(e.rel) && (excluded.is_empty() || visited.contains(&e.from) || visited.contains(&e.to)))
        .collect();
    if semantic_edges.is_empty() {
        out.push_str("(意味エッジなし)\n");
    } else {
        for e in semantic_edges {
            let (Some(from_n), Some(to_n)) = (graph.nodes.get(&e.from), graph.nodes.get(&e.to)) else {
                continue;
            };
            out.push_str(&format!(
                "- {}: {} \"{}\" → {} \"{}\"\n",
                rel_str(e.rel),
                short_address(e.from, from_n),
                node_short_label(graph, e.from),
                short_address(e.to, to_n),
                node_short_label(graph, e.to),
            ));
        }
    }
}

// --- スコープ2: --node + --hops --------------------------------------------------

pub(crate) fn render_node_scope(graph: &Graph, ids: &[NodeId], hops: u32) -> Result<String, ContextError> {
    let mut out = String::new();
    out.push_str("# Context: --node scope\n\n");

    let mut chunk_ids: HashSet<NodeId> = HashSet::new();
    for &id in ids {
        chunk_ids.extend(subtree_ids(graph, id));
    }

    // 「Chunk:」見出しは付けない: render_block がノード自身の行(見出し/表/段落等、
    // いずれも address_tag 込み)を必ず出すため、別行で同じラベルを繰り返すと冒頭が
    // 二重表示になる(実装時に実データで確認、裁量で削除)。複数 --node の場合だけ
    // `---` で区切って境界を分かるようにする。
    for (i, &id) in ids.iter().enumerate() {
        if i > 0 {
            out.push_str("---\n\n");
        }
        render_block(graph, id, 1, &mut out);
    }

    let neighbors = semantic_neighbors(graph, &chunk_ids, hops);
    out.push_str(&format!("## 近傍({hops} ホップ)\n\n"));
    if neighbors.is_empty() {
        out.push_str("(近傍ノードなし)\n");
    } else {
        let parents = strata_core::parent_index(graph);
        for (nid, rels) in &neighbors {
            let Some(n) = graph.nodes.get(nid) else { continue };
            let rel_desc = rels
                .iter()
                .map(|(rel, from_seed)| {
                    let arrow = if *from_seed { "→" } else { "←" };
                    format!("{}{arrow}", rel_str(*rel))
                })
                .collect::<Vec<_>>()
                .join(", ");
            let path = ancestor_path(graph, &parents, *nid);
            let loc = if path.is_empty() { String::new() } else { format!(" — 位置: {}", path.join(" > ")) };
            out.push_str(&format!("- [{rel_desc}] {} {}{loc}\n", node_short_label(graph, *nid), address_tag(*nid, n)));
        }
    }
    out.push('\n');

    Ok(out)
}

// --- スコープ3: --class -----------------------------------------------------------

pub(crate) fn render_class_scope(graph: &Graph, tag: &str, root: Option<NodeId>) -> Result<String, ContextError> {
    let mut out = String::new();
    out.push_str(&format!("# Context: --class {tag}\n\n"));

    let parents = strata_core::parent_index(graph);
    let doc_order = root.map(|r| subtree_ids(graph, r));
    let roots = class_chunk_roots(graph, tag, &parents, None);
    let ordered = order_ids(&roots, doc_order.as_deref());
    render_class_matches(graph, &parents, &ordered, &mut out);
    Ok(out)
}

/// `--node` と `--class` の併用(裁量、AND 判定): 指定ノード群の contains サブツリーに
/// 収まる class 一致ブロックだけを列挙する。`--hops`(近傍)は使わない — 近傍という
/// 概念と class 横断列挙は目的が異なる(前者は1ノードの周辺文脈、後者は文書横断の
/// タグ検索)ため素直に両立しない、という判断(最終報告に明記)。
pub(crate) fn render_node_and_class_scope(graph: &Graph, ids: &[NodeId], tag: &str) -> Result<String, ContextError> {
    let mut out = String::new();
    let node_list = ids.iter().map(|id| short_address(*id, &graph.nodes[id])).collect::<Vec<_>>().join(", ");
    out.push_str(&format!("# Context: --node {node_list} --class {tag}\n\n"));

    let mut scope_ids: Vec<NodeId> = Vec::new();
    let mut seen = HashSet::new();
    for &id in ids {
        for sid in subtree_ids(graph, id) {
            if seen.insert(sid) {
                scope_ids.push(sid);
            }
        }
    }
    let scope_set: HashSet<NodeId> = scope_ids.iter().copied().collect();
    let parents = strata_core::parent_index(graph);
    let roots = class_chunk_roots(graph, tag, &parents, Some(&scope_set));
    let ordered = order_ids(&roots, Some(&scope_ids));
    render_class_matches(graph, &parents, &ordered, &mut out);
    Ok(out)
}

/// `ids` を `doc_order`(文書順)があればその順で、無ければ `NodeId` 昇順で並べる。
fn order_ids(ids: &HashSet<NodeId>, doc_order: Option<&[NodeId]>) -> Vec<NodeId> {
    match doc_order {
        Some(order) => order.iter().copied().filter(|id| ids.contains(id)).collect(),
        None => {
            let mut v: Vec<NodeId> = ids.iter().copied().collect();
            v.sort();
            v
        }
    }
}

/// D46: `class_chunk_roots` が選んだ chunk の根を、各々サブツリーごと(`render_block` は
/// 常に `contains` を最後まで辿る)描画する。コンテナ(見出し・リスト・引用)に class を
/// 1回書けば、配下の子は根の描画に含まれて自動的に出る(子を重複列挙しない)。
fn render_class_matches(graph: &Graph, parents: &HashMap<NodeId, NodeId>, ids: &[NodeId], out: &mut String) {
    if ids.is_empty() {
        out.push_str("(該当ブロックなし)\n");
        return;
    }
    for &id in ids {
        let path = ancestor_path(graph, parents, id);
        let loc = if path.is_empty() { "(位置不明)".to_string() } else { path.join(" > ") };
        out.push_str(&format!("位置: {loc}\n\n"));
        render_block(graph, id, 1, out);
    }
}

/// D21 の「最初の H1」と同じ定義: Document 直下(トップレベル)に ord 順で最初に現れる
/// Section の見出しプレーンテキスト。
fn first_h1_title(graph: &Graph, root: NodeId) -> Option<String> {
    for child_id in graph.children_of(root) {
        if let Some(NodePayload::Section(s)) = graph.nodes.get(&child_id).map(|n| &n.payload) {
            return Some(crate::inline::plain_text(&s.heading));
        }
    }
    None
}

// --- ブロック描画本体 --------------------------------------------------------------

fn render_block(graph: &Graph, id: NodeId, depth: usize, out: &mut String) {
    let Some(node) = graph.nodes.get(&id) else { return };
    let tag = address_tag(id, node);

    match &node.payload {
        NodePayload::Section(s) => {
            let level = "#".repeat((depth + 1).min(6));
            out.push_str(&format!("{level} {} {tag}\n\n", render_inlines_md(graph, &s.heading)));
            for child in graph.children_of(id) {
                render_block(graph, child, depth + 1, out);
            }
        }
        NodePayload::Para(p) => {
            out.push_str(&format!("{} {tag}\n\n", render_inlines_md(graph, &p.inline)));
        }
        NodePayload::List(l) => {
            let _ = l;
            out.push_str(&format!("リスト {tag}\n\n"));
            render_list_items(graph, id, 0, out);
            out.push('\n');
        }
        NodePayload::Table(t) => {
            let caption = t.caption.as_ref().map(|c| render_inlines_md(graph, c)).unwrap_or_default();
            out.push_str(&format!("表: {caption} {tag}\n\n"));
            out.push_str("行パス | 列パス: 値\n\n");
            for cell in &t.cells {
                let v = cell_value_text(&cell.value);
                if v.is_empty() {
                    continue;
                }
                out.push_str(&format!("{} | {}: {v}\n", cell.row_path.join("."), cell.col_path.join(".")));
            }
            out.push('\n');
        }
        NodePayload::Record(r) => {
            out.push_str(&format!("record {tag}\n\n"));
            for e in &r.entries {
                out.push_str(&format!("{}: {}\n", e.key, cell_value_text(&e.value)));
            }
            out.push('\n');
        }
        NodePayload::Math(m) => {
            out.push_str(&format!("数式 {tag}\n\n$$ {} $$\n\n", render_math_text(&m.tree)));
        }
        NodePayload::Code(c) => {
            out.push_str(&format!("```{}\n{}\n``` {tag}\n\n", c.lang, c.src));
        }
        NodePayload::Figure(f) => render_figure(graph, f, &tag, out),
        NodePayload::Value(_) => {
            out.push_str(&format!("値: {} {tag}\n\n", node_short_label(graph, id)));
        }
        NodePayload::Anchor(a) => {
            out.push_str(&format!("[{}] {tag}\n\n", render_inlines_md(graph, &a.inline)));
        }
        NodePayload::Term(t) => {
            out.push_str(&format!("用語: {} {tag}\n\n", t.name));
        }
        NodePayload::Document(_) => {
            // 通常 Document はルートとしてのみ現れ、render_document_scope が別経路で処理する。
        }
        // M6(D40): blockquote — Markdown の `>` 記法を模した見出し行 + 子ブロック展開。
        NodePayload::Quote(_) => {
            out.push_str(&format!("引用 {tag}\n\n"));
            for child in graph.children_of(id) {
                render_block(graph, child, depth, out);
            }
        }
        // M6(D40): 水平線。
        NodePayload::ThematicBreak(_) => {
            out.push_str(&format!("--- {tag}\n\n"));
        }
    }
}

fn render_list_items(graph: &Graph, list_id: NodeId, indent: usize, out: &mut String) {
    let (ordered, start) = match graph.nodes.get(&list_id).map(|n| &n.payload) {
        Some(NodePayload::List(l)) => (l.ordered, l.start),
        _ => (false, None),
    };
    let pad = "  ".repeat(indent);
    // M6(D40、監査②5): 順序リストの開始値を反映した番号を振る。
    let base = start.unwrap_or(1);

    for (i, child_id) in graph.children_of(list_id).into_iter().enumerate() {
        let Some(child_node) = graph.nodes.get(&child_id) else { continue };
        let tag = address_tag(child_id, child_node);
        let marker = if ordered { format!("{}.", base + i as u64) } else { "-".to_string() };

        match &child_node.payload {
            NodePayload::Para(p) => {
                // M6(D40 Tier2): タスクリストのチェック状態(GFM 記法)。
                let check = match p.checked {
                    Some(true) => "[x] ",
                    Some(false) => "[ ] ",
                    None => "",
                };
                out.push_str(&format!("{pad}{marker} {check}{} {tag}\n", render_inlines_md(graph, &p.inline)));
                for sub_id in graph.children_of(child_id) {
                    if let Some(NodePayload::List(_)) = graph.nodes.get(&sub_id).map(|n| &n.payload) {
                        render_list_items(graph, sub_id, indent + 1, out);
                    }
                }
            }
            _ => {
                out.push_str(&format!("{pad}{marker} {} {tag}\n", node_short_label(graph, child_id)));
            }
        }
    }
}

fn render_figure(graph: &Graph, f: &Figure, tag: &str, out: &mut String) {
    match f {
        Figure::Chart(c) => {
            let desc = c.depicts.get("description").cloned().unwrap_or_default();
            let caption = c.caption.as_ref().map(|cap| render_inlines_md(graph, cap)).unwrap_or_default();
            out.push_str(&format!(
                "図(チャート mark={:?} x={} y={}): {caption} {desc} {tag}\n\n",
                c.mark, c.encode.x, c.encode.y
            ));
        }
        Figure::Image(img) => {
            let desc = img.depicts.get("description").cloned().unwrap_or_default();
            let caption = img.caption.as_ref().map(|cap| render_inlines_md(graph, cap)).unwrap_or_default();
            out.push_str(&format!("図(画像 alt={}): {caption} {desc} {tag}\n\n", img.alt));
        }
    }
}
