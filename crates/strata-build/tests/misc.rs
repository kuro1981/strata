//! WP-B4 完了チェックリストの残り: term 集約と安定 ID(小さい合成入力での確認。
//! 導出式のハードコード固定は `term.rs` の単体テストで別途行う)、見出しネスト
//! (レベル飛び含む)、invariants 通過(dangling を意図的に作って検出させる)、
//! コードフェンスの変換。

use strata_build::{build, BuildError};
use strata_core::{NodePayload, Rel};
use ulid::Ulid;

#[test]
fn same_named_term_referenced_twice_aggregates_to_one_node() {
    let p1 = Ulid::new();
    let p2 = Ulid::new();
    let src = format!(
        "[id={p1}]\n最初の参照 [用語](term:用語) です。\n\n[id={p2}]\n2回目の参照 [用語](term:用語) です。\n"
    );
    let out = build(&src).expect("well-formed doc with a repeated term reference");

    let term_nodes: Vec<_> =
        out.graph.nodes.values().filter(|n| matches!(n.payload, NodePayload::Term(_))).collect();
    assert_eq!(term_nodes.len(), 1, "same name must aggregate to a single Term node");

    let term_id = term_nodes[0].id;
    let refs: Vec<_> = out.graph.edges.iter().filter(|e| e.to == term_id && e.rel == Rel::TermRef).collect();
    assert_eq!(refs.len(), 2, "both paragraphs must link to the same term node");
}

#[test]
fn heading_nesting_handles_level_skip() {
    // # A (level1) -> ### C (level3, レベル飛び) -> ## B (level2)
    // C は直近の浅い方(A)の子、B も同様に C を pop して A の子になる。
    let a = Ulid::new();
    let b = Ulid::new();
    let c = Ulid::new();
    let src = format!("# A {{#{a}}}\n\n### C {{#{c}}}\n\n## B {{#{b}}}\n");
    let out = build(&src).expect("well-formed heading-only doc");

    let a_id = strata_core::NodeId(a);
    let b_id = strata_core::NodeId(b);
    let c_id = strata_core::NodeId(c);

    assert_eq!(out.graph.children_of(a_id), vec![c_id, b_id], "both C and B nest directly under A");
}

#[test]
fn dangling_ulid_reference_is_caught_by_invariants_after_build() {
    // 参照先が ULID(alias 表を経由しない)なので Pass 2 では解決済み扱いになるが、
    // そのノードは実在しない → build 後の invariants::validate が DanglingEdge として
    // 検出し、BuildError::Invariant に変換されて返ること(sml-build-m3-handoff.md D-B5)。
    let para = Ulid::new();
    let nowhere = Ulid::new();
    let src = format!("[id={para}]\nSee [it](ref:{nowhere}).\n");
    let errors = build(&src).expect_err("dangling ULID reference must be caught");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::Invariant(strata_core::invariants::Violation::DanglingEdge { .. }))),
        "got: {errors:#?}"
    );
}

#[test]
fn code_fence_converts_to_code_node_with_lang_and_source() {
    let id = Ulid::new();
    let src = format!("```rust {{#{id}}}\nfn main() {{}}\n```\n");
    let out = build(&src).expect("well-formed code fence");
    let node = &out.graph.nodes[&strata_core::NodeId(id)];
    match &node.payload {
        NodePayload::Code(c) => {
            assert_eq!(c.lang, "rust");
            assert_eq!(c.src, "fn main() {}\n");
        }
        other => panic!("expected Code, got {other:?}"),
    }
}
