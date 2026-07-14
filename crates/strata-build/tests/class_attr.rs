//! D23(2026-07-14 裁定): class 属性の build 側テスト(WP-X1)。
//!
//! - class は前置属性行 `[class=...]` から `Node.classes` に格納される
//! - 単一 / リスト値の両方
//! - 見出し・段落・リスト全体・フェンス・コードフェンスのいずれでも付与できる
//! - build の成否・グラフ構造は class に非依存(字句が正しければ常に成功)
//! - 字句違反(`[A-Za-z0-9_-]+` の外)は `BuildError::BadClass`(全か無かで build 失敗)

use strata_build::{build, BuildError};
use strata_core::NodeId;
use ulid::Ulid;

#[test]
fn class_on_paragraph_is_stored_on_node() {
    let id = Ulid::new();
    let src = format!("[id={id}, class=note]\n本文です。\n");
    let out = build(&src).expect("well-formed doc with class attr");
    let node = &out.graph.nodes[&NodeId(id)];
    assert_eq!(node.classes, vec!["note".to_string()]);
}

#[test]
fn class_list_value_is_stored_as_multiple_classes() {
    let id = Ulid::new();
    let src = format!("[id={id}, class=[note, actual-name]]\n本文です。\n");
    let out = build(&src).expect("well-formed doc with class list");
    let node = &out.graph.nodes[&NodeId(id)];
    assert_eq!(node.classes, vec!["note".to_string(), "actual-name".to_string()]);
}

#[test]
fn class_on_heading_is_stored_on_section_node() {
    let id = Ulid::new();
    let src = format!("[class=note]\n# Title {{#{id}}}\n");
    let out = build(&src).expect("well-formed doc with class on heading");
    let node = &out.graph.nodes[&NodeId(id)];
    assert_eq!(node.classes, vec!["note".to_string()]);
}

#[test]
fn class_on_fence_is_stored_on_math_node() {
    let id = Ulid::new();
    let src = format!("[class=note]\n::math {{#{id}}}\nx = 1\n::\n");
    let out = build(&src).expect("well-formed doc with class on fence");
    let node = &out.graph.nodes[&NodeId(id)];
    assert_eq!(node.classes, vec!["note".to_string()]);
}

#[test]
fn class_on_code_fence_is_stored_on_code_node() {
    let id = Ulid::new();
    let src = format!("[class=note]\n```rust {{#{id}}}\nfn main() {{}}\n```\n");
    let out = build(&src).expect("well-formed doc with class on code fence");
    let node = &out.graph.nodes[&NodeId(id)];
    assert_eq!(node.classes, vec!["note".to_string()]);
}

/// build の成否・グラフ構造は class の有無・値に非依存(全ノード常時格納、D23)。
/// class が無いノードは classes が空 Vec のまま。
#[test]
fn build_succeeds_regardless_of_class_presence() {
    let id = Ulid::new();
    let src = format!("[id={id}]\nクラス無しの段落。\n");
    let out = build(&src).expect("class の有無に build の成否は依存しない");
    let node = &out.graph.nodes[&NodeId(id)];
    assert!(node.classes.is_empty());
}

/// class タグの字句違反(`[A-Za-z0-9_-]+` の外)は `BuildError::BadClass`。
/// 全か無かにより build 全体が失敗する。
#[test]
fn bad_class_charset_is_reported_and_fails_the_whole_build() {
    let id = Ulid::new();
    let src = format!("[id={id}, class=bad.class]\n本文です。\n");
    let errors = build(&src).expect_err("非字句の class は build を失敗させる");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::BadClass { .. })),
        "got: {errors:#?}"
    );
}
