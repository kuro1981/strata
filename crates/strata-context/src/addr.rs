//! ノードアドレス解決(`--node <alias|ULID>`)と、出力側のアドレスタグ組み立て。

use strata_core::{Graph, Node, NodeId};
use ulid::Ulid;

/// `--node` に渡された文字列(alias または ULID)を `NodeId` に解決する。
/// alias を優先して引き、無ければ ULID としてパースを試みる(sml-spec の
/// アドレス規約と同じ優先順位: alias があれば alias、無ければ ULID)。
pub fn resolve_node_ref(graph: &Graph, s: &str) -> Option<NodeId> {
    if let Some((id, _)) = graph.nodes.iter().find(|(_, n)| n.alias.as_deref() == Some(s)) {
        return Some(*id);
    }
    let ulid = Ulid::from_string(s).ok()?;
    let id = NodeId(ulid);
    graph.nodes.contains_key(&id).then_some(id)
}

/// 出力に埋め込む SML 風のアドレスタグ: `{#ULID alias=... class=c1,c2}`。
/// alias / class が無ければ該当部分を省略する(sml-spec §3 の ID タグ記法に揃える —
/// LLM が同じ記法をソースと出力の両方で読めるようにする裁量)。
pub fn address_tag(id: NodeId, node: &Node) -> String {
    let mut tag = format!("{{#{}", id.0);
    if let Some(alias) = &node.alias {
        tag.push_str(" alias=");
        tag.push_str(alias);
    }
    if !node.classes.is_empty() {
        tag.push_str(" class=");
        tag.push_str(&node.classes.join(","));
    }
    tag.push('}');
    tag
}

/// 短いアドレス(alias があれば alias、無ければ ULID 文字列)。エッジ一覧・近傍要約で
/// 「引用に使う短い名前」として使う。
pub fn short_address(id: NodeId, node: &Node) -> String {
    node.alias.clone().unwrap_or_else(|| id.0.to_string())
}
