//! D24(2026-07-14 裁定、WP-X2): ネストリストの build 側テスト。
//!
//! - 子リストが `List` ノードとして親項目(Para)に contains で接続されること
//! - ord 順序の維持
//! - 既存の平坦リストのグラフ表現が変わらないこと
//! - 不正インデント(旧・無警告誤パース入力)が build エラーになる回帰テスト

use strata_build::{build, BuildError};
use strata_core::{NodeId, NodePayload};
use ulid::Ulid;

#[test]
fn nested_list_builds_child_list_node_under_parent_item() {
    let list = Ulid::new();
    let top = Ulid::new();
    let sub1 = Ulid::new();
    let sub2 = Ulid::new();
    let src = format!(
        "[id={list}]\n- top {{#{top}}}\n  - sub1 {{#{sub1}}}\n  - sub2 {{#{sub2}}}\n"
    );
    let out = build(&src).expect("nested list must build");

    let top_id = NodeId(top);
    // 親項目(Para)の子は自動生成 ID の List ノード1つ。
    let top_children = out.graph.children_of(top_id);
    assert_eq!(top_children.len(), 1, "parent item must contain exactly one child list");
    let child_list_id = top_children[0];
    assert!(
        matches!(out.graph.nodes[&child_list_id].payload, NodePayload::List(_)),
        "child of parent item must be a List node"
    );

    // 子リストの子は sub1, sub2 が ord 順。
    assert_eq!(out.graph.children_of(child_list_id), vec![NodeId(sub1), NodeId(sub2)]);

    // リスト全体の子は top のみ(ネスト項目はトップレベル List の直下に来ない)。
    assert_eq!(out.graph.children_of(NodeId(list)), vec![top_id]);
}

#[test]
fn flat_list_graph_shape_is_unchanged() {
    let list = Ulid::new();
    let a = Ulid::new();
    let b = Ulid::new();
    let src = format!("[id={list}]\n- a {{#{a}}}\n- b {{#{b}}}\n");
    let out = build(&src).expect("flat list must build");
    // 平坦リスト: List が項目 Para を直接 contains(従来どおり)。
    assert_eq!(out.graph.children_of(NodeId(list)), vec![NodeId(a), NodeId(b)]);
    // 中間ノードが増えていないこと(Document 無し: List + Para×2 = 3ノード)。
    assert_eq!(out.graph.nodes.len(), 3);
}

#[test]
fn three_level_nest_builds_recursively() {
    let list = Ulid::new();
    let a = Ulid::new();
    let b = Ulid::new();
    let c = Ulid::new();
    let src = format!("[id={list}]\n- a {{#{a}}}\n  - b {{#{b}}}\n    - c {{#{c}}}\n");
    let out = build(&src).expect("3-level nest must build");

    let list_under_a = out.graph.children_of(NodeId(a))[0];
    assert_eq!(out.graph.children_of(list_under_a), vec![NodeId(b)]);
    let list_under_b = out.graph.children_of(NodeId(b))[0];
    assert_eq!(out.graph.children_of(list_under_b), vec![NodeId(c)]);
}

/// ネスト項目に ULID が無い(fmt 未実行)場合も MissingId になる(平坦項目と同じ扱い)。
#[test]
fn nested_item_without_ulid_is_missing_id() {
    let list = Ulid::new();
    let top = Ulid::new();
    let src = format!("[id={list}]\n- top {{#{top}}}\n  - sub-without-id\n");
    let errors = build(&src).expect_err("nested item without ULID must fail");
    assert!(errors.iter().any(|e| matches!(e, BuildError::MissingId { .. })), "{errors:#?}");
}

/// 回帰(D24): 旧実装で無警告のまま別段落に化けていた不正インデント入力は、
/// いまや `InconsistentIndent`(Error)のパース診断として build を失敗させる。
#[test]
fn bad_indent_input_fails_build_with_parse_error() {
    let para = Ulid::new();
    let src = format!("[id={para}]\n本文の段落です。\n   - 奇数インデントの行\n");
    let errors = build(&src).expect_err("bad indent must fail the build");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            BuildError::Parse(d) if d.kind == strata_sml::DiagKind::InconsistentIndent
        )),
        "{errors:#?}"
    );
}
