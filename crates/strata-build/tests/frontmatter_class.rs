//! D61(2026-07-29 裁定、sml-spec §1.19): フロントマター `class:` の build 側テスト。
//!
//! - フロントマターの class は Document ノードの `Node.classes` に格納される
//!   (単一値・リスト値の両方、D23 のブロック `class=` と同じ `apply_class_attr` を再利用)
//! - D46(実効 class = 自身+祖先)により、Document が最上位祖先なので文書直下の
//!   全ブロックへ自然に継承される(`effective_classes` / `has_effective_class`)
//! - class の有無・値に build の成否は非依存。字句違反(`[A-Za-z0-9_-]+` の外)は
//!   `BuildError::BadClass`(全か無かで build 失敗、ブロック class= と同じ扱い)

use std::collections::HashSet;

use strata_build::{build, BuildError};
use strata_core::{effective_classes, has_effective_class, parent_index, NodeId, NodePayload};
use ulid::Ulid;

#[test]
fn frontmatter_single_class_is_stored_on_document_node() {
    let doc_id = Ulid::new();
    let heading_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\nclass: note\n---\n\n# Title {{#{heading_id}}}\n");
    let out = build(&src).expect("well-formed doc with frontmatter class");
    let node = &out.graph.nodes[&NodeId(doc_id)];
    assert!(matches!(node.payload, NodePayload::Document(_)));
    assert_eq!(node.classes, vec!["note".to_string()]);
}

#[test]
fn frontmatter_class_list_is_stored_as_multiple_classes() {
    let doc_id = Ulid::new();
    let heading_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\nclass: [note, actual-name]\n---\n\n# Title {{#{heading_id}}}\n");
    let out = build(&src).expect("well-formed doc with frontmatter class list");
    let node = &out.graph.nodes[&NodeId(doc_id)];
    assert_eq!(node.classes, vec!["note".to_string(), "actual-name".to_string()]);
}

#[test]
fn build_succeeds_regardless_of_frontmatter_class_presence() {
    let doc_id = Ulid::new();
    let heading_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\n---\n\n# Title {{#{heading_id}}}\n");
    let out = build(&src).expect("class の有無に build の成否は依存しない");
    let node = &out.graph.nodes[&NodeId(doc_id)];
    assert!(node.classes.is_empty());
}

/// D46: Document に付けた class は文書直下の全ブロック(見出し・段落・孫ブロック)へ
/// 実効的に継承される — コンテナに1回書けば子孫すべてに効く、という D46 の統一
/// セマンティクスが Document でも成り立つことの確認。
#[test]
fn frontmatter_class_is_inherited_by_all_top_level_blocks() {
    let doc_id = Ulid::new();
    let heading_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!(
        "---\nid: {doc_id}\nclass: note\n---\n\n\
         # Title {{#{heading_id}}}\n\n\
         [id={para_id}]\n本文です。\n"
    );
    let out = build(&src).expect("well-formed doc with frontmatter class");
    let parents = parent_index(&out.graph);

    // Document 自身の実効 class にも note が含まれる(自身+祖先の和集合、祖先が無くても自身は含む)。
    assert_eq!(effective_classes(&out.graph, &parents, NodeId(doc_id)), HashSet::from(["note".to_string()]));
    // 直下の見出し(Section)・段落(Para)双方に継承される。
    assert_eq!(effective_classes(&out.graph, &parents, NodeId(heading_id)), HashSet::from(["note".to_string()]));
    assert_eq!(effective_classes(&out.graph, &parents, NodeId(para_id)), HashSet::from(["note".to_string()]));

    let tags: HashSet<&str> = HashSet::from(["note"]);
    assert!(has_effective_class(&out.graph, &parents, NodeId(heading_id), &tags));
    assert!(has_effective_class(&out.graph, &parents, NodeId(para_id), &tags));
}

/// フロントマターの class の字句違反は `BuildError::BadClass`(全か無かで build 失敗)。
/// ブロック前置属性行の class= と同じ検証経路(`apply_class_attr`)を通ることの確認。
#[test]
fn bad_frontmatter_class_charset_is_reported_and_fails_the_whole_build() {
    let doc_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\nclass: bad.class\n---\n");
    let errors = build(&src).expect_err("非字句の frontmatter class は build を失敗させる");
    assert!(errors.iter().any(|e| matches!(e, BuildError::BadClass { .. })), "got: {errors:#?}");
}
