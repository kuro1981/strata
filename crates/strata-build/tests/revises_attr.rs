//! rel `revises`(sml-spec §1.13 D48)の build 側テスト(WS-A)。
//!
//! - 属性行キー `revises=` は既存の supports/depends-on/cites と同経路で
//!   `Rel::Revises` エッジを materialise する(方向は新→旧: from=改定する側)
//! - 複数ターゲット(`revises=[a, b]`)も張れる
//! - 未知の rel ではなく既知キーとして扱われるため `UnknownAttrKey` warning は出ない

use strata_build::build;
use strata_core::{NodeId, Rel};
use ulid::Ulid;

#[test]
fn revises_attr_materialises_revises_edge() {
    let old_id = Ulid::new();
    let new_id = Ulid::new();
    let src = format!(
        "# 旧裁定 {{#{old_id}}}\n\n[id={new_id}, revises={old_id}]\n新裁定は旧裁定を改定する。\n"
    );
    let out = build(&src).expect("well-formed doc with revises attr");
    assert!(
        out.graph
            .edges
            .iter()
            .any(|e| e.from == NodeId(new_id) && e.to == NodeId(old_id) && e.rel == Rel::Revises),
        "got edges: {:#?}",
        out.graph.edges
    );
}

#[test]
fn revises_attr_accepts_alias_and_multiple_targets() {
    let a_id = Ulid::new();
    let b_id = Ulid::new();
    let new_id = Ulid::new();
    let src = format!(
        "# A {{#{a_id} alias=old-a}}\n\n## B {{#{b_id} alias=old-b}}\n\n[id={new_id}, revises=[old-a, old-b]]\n新裁定は A と B を改定する。\n"
    );
    let out = build(&src).expect("well-formed doc with multi-target revises attr");
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(new_id) && e.to == NodeId(a_id) && e.rel == Rel::Revises));
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(new_id) && e.to == NodeId(b_id) && e.rel == Rel::Revises));
}

#[test]
fn revises_is_a_known_attr_key_and_does_not_warn() {
    let old_id = Ulid::new();
    let new_id = Ulid::new();
    let src = format!(
        "# 旧裁定 {{#{old_id}}}\n\n[id={new_id}, revises={old_id}]\n新裁定は旧裁定を改定する。\n"
    );
    let out = build(&src).expect("well-formed doc with revises attr");
    assert!(out.warnings.is_empty(), "revises= should not produce UnknownAttrKey warnings: {:#?}", out.warnings);
}
