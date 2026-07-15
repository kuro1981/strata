//! 3スコープ(D36)の Markdown 組み立て本体。

use std::collections::HashSet;

use strata_core::{Figure, Graph, NodeId, NodePayload};

use crate::addr::{address_tag, short_address};
use crate::inline::{render_inlines_md, render_math_text};
use crate::label::{cell_value_text, node_short_label};
use crate::{ContextError, ancestor_path, is_semantic, nodes_with_class, parent_index, rel_str, semantic_neighbors, subtree_ids};

/// ブロックの描画モード。`Full` は `contains` を最後まで辿る(全文書 / node チャンク)。
/// `Leaf` はそのブロック自身の内容だけを描く(class 横断列挙 — D23 の「class はブロック
/// 単位」に合わせ、子孫ブロックを勝手に道連れにしない裁量)。`List` の直接の項目だけは
/// どちらのモードでも描く(項目列挙自体がリストというブロックの中身のため)。
#[derive(Clone, Copy, PartialEq, Eq)]
enum RenderMode {
    Full,
    Leaf,
}

// --- スコープ1: 全文書 -----------------------------------------------------------

pub(crate) fn render_document_scope(graph: &Graph, root: NodeId) -> Result<String, ContextError> {
    let mut out = String::new();
    let root_node = graph.nodes.get(&root).ok_or(ContextError::NoRoot)?;
    // D21 と同じ3段フォールバック(Document.title → 最初の H1 → 無題)を踏襲する(裁量:
    // strata-typst の `first_h1_title` と同じ運用に揃え、タイトルの見え方を一致させる)。
    let title = match &root_node.payload {
        NodePayload::Document(d) => d.title.clone().unwrap_or_else(|| first_h1_title(graph, root).unwrap_or_else(|| "(無題)".to_string())),
        _ => node_short_label(graph, root),
    };
    out.push_str(&format!("# {} {}\n\n", title, address_tag(root, root_node)));

    for child in graph.children_of(root) {
        render_block(graph, child, 1, &mut out, RenderMode::Full);
    }

    // D36 形式要件: 全ブロックノードが ULID でアドレス可能。`contains` で document から
    // 到達できないノード(Term は用語使用側からしか辿れず、contains を持たない)を
    // 「その他のノード」として末尾に列挙し、抜け漏れを無くす。
    let visited: HashSet<NodeId> = subtree_ids(graph, root).into_iter().collect();
    let orphans: Vec<NodeId> = graph.nodes.keys().copied().filter(|id| !visited.contains(id)).collect();
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
    let semantic_edges: Vec<_> = graph.edges.iter().filter(|e| is_semantic(e.rel)).collect();
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

    Ok(out)
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
        render_block(graph, id, 1, &mut out, RenderMode::Full);
    }

    let neighbors = semantic_neighbors(graph, &chunk_ids, hops);
    out.push_str(&format!("## 近傍({hops} ホップ)\n\n"));
    if neighbors.is_empty() {
        out.push_str("(近傍ノードなし)\n");
    } else {
        let parents = parent_index(graph);
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

    let doc_order = root.map(|r| subtree_ids(graph, r));
    let matches = nodes_with_class(graph, tag, doc_order.as_deref());
    render_class_matches(graph, &matches, &mut out);
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
    let matches = nodes_with_class(graph, tag, Some(&scope_ids));
    render_class_matches(graph, &matches, &mut out);
    Ok(out)
}

fn render_class_matches(graph: &Graph, matches: &[(&NodeId, &strata_core::Node)], out: &mut String) {
    if matches.is_empty() {
        out.push_str("(該当ブロックなし)\n");
        return;
    }
    let parents = parent_index(graph);
    for (id, _node) in matches {
        let path = ancestor_path(graph, &parents, **id);
        let loc = if path.is_empty() { "(位置不明)".to_string() } else { path.join(" > ") };
        out.push_str(&format!("位置: {loc}\n\n"));
        render_block(graph, **id, 1, out, RenderMode::Leaf);
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

fn render_block(graph: &Graph, id: NodeId, depth: usize, out: &mut String, mode: RenderMode) {
    let Some(node) = graph.nodes.get(&id) else { return };
    let tag = address_tag(id, node);

    match &node.payload {
        NodePayload::Section(s) => {
            let level = "#".repeat((depth + 1).min(6));
            out.push_str(&format!("{level} {} {tag}\n\n", render_inlines_md(graph, &s.heading)));
            if mode == RenderMode::Full {
                for child in graph.children_of(id) {
                    render_block(graph, child, depth + 1, out, mode);
                }
            }
        }
        NodePayload::Para(p) => {
            out.push_str(&format!("{} {tag}\n\n", render_inlines_md(graph, &p.inline)));
        }
        NodePayload::List(l) => {
            let _ = l;
            out.push_str(&format!("リスト {tag}\n\n"));
            render_list_items(graph, id, 0, out, mode);
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
    }
}

fn render_list_items(graph: &Graph, list_id: NodeId, indent: usize, out: &mut String, mode: RenderMode) {
    let ordered = matches!(graph.nodes.get(&list_id).map(|n| &n.payload), Some(NodePayload::List(l)) if l.ordered);
    let pad = "  ".repeat(indent);

    for (i, child_id) in graph.children_of(list_id).into_iter().enumerate() {
        let Some(child_node) = graph.nodes.get(&child_id) else { continue };
        let tag = address_tag(child_id, child_node);
        let marker = if ordered { format!("{}.", i + 1) } else { "-".to_string() };

        match &child_node.payload {
            NodePayload::Para(p) => {
                out.push_str(&format!("{pad}{marker} {} {tag}\n", render_inlines_md(graph, &p.inline)));
                if mode == RenderMode::Full {
                    for sub_id in graph.children_of(child_id) {
                        if let Some(NodePayload::List(_)) = graph.nodes.get(&sub_id).map(|n| &n.payload) {
                            render_list_items(graph, sub_id, indent + 1, out, mode);
                        }
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
