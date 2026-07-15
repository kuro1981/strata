use super::*;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repo root exists")
}

fn fixture_build() -> BuildOutput {
    let src = std::fs::read_to_string(repo_root().join("docs/sml_example_formatted.sml")).expect("fixture readable");
    strata_build::build(&src).expect("fixture must build cleanly")
}

// --- ゴールデン(全文書スコープ) ---------------------------------------------------
//
// `docs/sml_example_formatted.sml`(凍結フィクスチャ、無改変)を build → render_context
// した結果が `docs/sml_example_formatted.context.md`(本 WP で新規追加したゴールデン
// 成果物)と完全一致することを固定する。strata-typst の golden.rs(WP-R2)と同じ運用。

#[test]
fn full_document_scope_renders_byte_for_byte_identical_to_golden() {
    let build = fixture_build();
    let actual = render_context(&build, &ContextOptions::default()).expect("render must succeed");
    let expected = std::fs::read_to_string(repo_root().join("docs/sml_example_formatted.context.md"))
        .expect("golden fixture docs/sml_example_formatted.context.md must exist");
    assert_eq!(actual, expected, "render_context output drifted from the golden .context.md fixture");
}

#[test]
fn full_document_scope_is_deterministic() {
    let build = fixture_build();
    let a = render_context(&build, &ContextOptions::default()).unwrap();
    let b = render_context(&build, &ContextOptions::default()).unwrap();
    assert_eq!(a, b);
}

#[test]
fn full_document_scope_addresses_every_node_with_a_ulid_tag() {
    let build = fixture_build();
    let out = render_context(&build, &ContextOptions::default()).unwrap();
    for id in build.graph.nodes.keys() {
        let tag = format!("#{}", id.0);
        assert!(out.contains(&tag), "node {} has no address tag in the full-document output", id.0);
    }
}

#[test]
fn no_root_yields_no_root_error() {
    let build = strata_build::build("").expect("empty source builds");
    assert_eq!(build.root, None);
    let err = render_context(&build, &ContextOptions::default()).unwrap_err();
    assert_eq!(err, ContextError::NoRoot);
}

// --- --node + --hops ---------------------------------------------------------------

#[test]
fn node_scope_by_alias_renders_only_the_requested_subtree() {
    let build = fixture_build();
    let opts = ContextOptions { nodes: vec!["eval-table".to_string()], hops: 1, class: None };
    let out = render_context(&build, &opts).unwrap();

    // チャンク本体: 対象ノード自身の内容が出る。
    assert!(out.contains("Baseline-v1 | Dataset-A.F1-Score: 0.82"));
    // 別セクションの内容(導入節)はチャンクに含まれない。
    assert!(!out.contains("予測モデル の性能評価結果について報告する"));
}

/// ホップ境界(D36 スコープ2): `01J2T8Z7...`(「予測精度はモデルの実用性を…」)は
/// `supports` で term:予測精度 に1ホップで繋がり、term:予測精度 はさらに
/// `01J2T8Z5...`(リスト項目「予測精度 — F1スコアを基準とする」)から `term-ref` で
/// 2ホップ目に繋がる。hops=1 では前者だけ、hops=2 で両方が近傍に現れることを固定する。
#[test]
fn node_scope_hops_boundary_expands_at_exactly_n_hops() {
    let build = fixture_build();

    let one_hop = render_context(
        &build,
        &ContextOptions { nodes: vec!["01J2T8Z7000000000000000000".to_string()], hops: 1, class: None },
    )
    .unwrap();
    assert!(one_hop.contains("6MNYNRE7W9QBQSW9JGQS9R2CT6"), "1 hop must reach the term node via `supports`");
    assert!(!one_hop.contains("01J2T8Z5000000000000000000"), "1 hop must not reach the 2nd-hop list item yet");

    let two_hops = render_context(
        &build,
        &ContextOptions { nodes: vec!["01J2T8Z7000000000000000000".to_string()], hops: 2, class: None },
    )
    .unwrap();
    assert!(two_hops.contains("6MNYNRE7W9QBQSW9JGQS9R2CT6"), "2 hops must still include the 1st-hop neighbor");
    assert!(two_hops.contains("01J2T8Z5000000000000000000"), "2 hops must reach the 2nd-hop list item");
}

#[test]
fn node_scope_zero_hops_has_no_neighbors() {
    let build = fixture_build();
    let out = render_context(
        &build,
        &ContextOptions { nodes: vec!["01J2T8Z7000000000000000000".to_string()], hops: 0, class: None },
    )
    .unwrap();
    assert!(out.contains("(近傍ノードなし)"));
}

#[test]
fn node_scope_accepts_ulid_directly() {
    let build = fixture_build();
    let out = render_context(
        &build,
        &ContextOptions { nodes: vec!["01J2T8ZF000000000000000000".to_string()], hops: 1, class: None },
    )
    .unwrap();
    assert!(out.contains("alias=loss-formula"));
}

#[test]
fn unknown_node_ref_is_a_clear_error() {
    let build = fixture_build();
    let opts = ContextOptions { nodes: vec!["no-such-alias".to_string()], hops: 1, class: None };
    let err = render_context(&build, &opts).unwrap_err();
    assert_eq!(err, ContextError::UnknownNodeRef("no-such-alias".to_string()));
}

// --- --class -------------------------------------------------------------------------
//
// 凍結フィクスチャ(docs/sml_example_formatted.sml)は class 属性を使っていないため、
// このスコープはテスト内で組み立てた最小 SML で検証する(フィクスチャ改版禁止)。

const CLASS_FIXTURE: &str = "---\n\
id: 01HZZZZZZZZZZZZZZZZZZZZZZ0\n\
---\n\
\n\
# 文書 {#01HZZZZZZZZZZZZZZZZZZZZZZ1}\n\
\n\
## 章A {#01HZZZZZZZZZZZZZZZZZZZZZZ2}\n\
\n\
[id=01HZZZZZZZZZZZZZZZZZZZZZZ3]\n\
本文A。\n\
\n\
[id=01HZZZZZZZZZZZZZZZZZZZZZZ4, class=note]\n\
補足A。\n\
\n\
## 章B {#01HZZZZZZZZZZZZZZZZZZZZZZ5}\n\
\n\
[id=01HZZZZZZZZZZZZZZZZZZZZZZ6, class=note]\n\
補足B。\n\
\n\
[id=01HZZZZZZZZZZZZZZZZZZZZZZ7, class=other]\n\
別クラス。\n";

#[test]
fn class_scope_extracts_only_matching_blocks_in_document_order_with_location() {
    let build = strata_build::build(CLASS_FIXTURE).unwrap();
    let opts = ContextOptions { nodes: vec![], hops: 1, class: Some("note".to_string()) };
    let out = render_context(&build, &opts).unwrap();

    assert!(out.contains("補足A"));
    assert!(out.contains("補足B"));
    assert!(!out.contains("別クラス"), "class=other ブロックは note スコープに出ない");
    assert!(!out.contains("本文A"), "class 無しの通常段落は class スコープに出ない");

    // 文書順(章A → 章B)を保つこと。
    let pos_a = out.find("補足A").unwrap();
    let pos_b = out.find("補足B").unwrap();
    assert!(pos_a < pos_b);

    // 各項目に位置文脈(祖先見出しパス)が1行付くこと。
    assert!(out.contains("位置: 文書 > 章A"));
    assert!(out.contains("位置: 文書 > 章B"));
}

#[test]
fn class_scope_with_no_matches_says_so_explicitly() {
    let build = strata_build::build(CLASS_FIXTURE).unwrap();
    let opts = ContextOptions { nodes: vec![], hops: 1, class: Some("no-such-class".to_string()) };
    let out = render_context(&build, &opts).unwrap();
    assert!(out.contains("(該当ブロックなし)"));
}

#[test]
fn class_scope_is_deterministic() {
    let build = strata_build::build(CLASS_FIXTURE).unwrap();
    let opts = ContextOptions { nodes: vec![], hops: 1, class: Some("note".to_string()) };
    let a = render_context(&build, &opts).unwrap();
    let b = render_context(&build, &opts).unwrap();
    assert_eq!(a, b);
}

// --- --node + --class 併用(裁量: AND) ------------------------------------------------

#[test]
fn node_and_class_scope_intersects_to_only_matches_within_the_node_subtree() {
    let build = strata_build::build(CLASS_FIXTURE).unwrap();
    let opts = ContextOptions {
        nodes: vec!["01HZZZZZZZZZZZZZZZZZZZZZZ2".to_string()],
        hops: 1,
        class: Some("note".to_string()),
    };
    let out = render_context(&build, &opts).unwrap();
    assert!(out.contains("補足A"), "章A のサブツリー内の note は出る");
    assert!(!out.contains("補足B"), "章B のサブツリー外の note は出ない(AND 判定)");
}
