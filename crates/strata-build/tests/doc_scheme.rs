//! D53(2026-07-16 裁定、sml-spec.md §1.14): `doc:` スキーム(Document ノード直指し)。
//!
//! - 単一ファイル build: 自文書 alias のみ解決可、ULID も解決可
//! - 単一ファイル build: 他文書 alias は `DocRefNeedsWorkspace`(黙って落とさず
//!   `--workspace` を案内、`CrossDocRef` と同型)
//! - ワークスペース build: 他文書 alias も解決できる。未知 alias は `UnknownDoc`
//! - 対象が Document でないノードを `doc:` で指すと `RefTypeMismatch`

use strata_build::{build, build_workspace, BuildError, Member};
use strata_core::{NodeId, NodePayload, Rel};
use ulid::Ulid;

fn member(path: &str, src: String) -> Member {
    Member { path: path.to_string(), src }
}

#[test]
fn single_file_build_resolves_doc_ref_to_own_alias() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!(
        "---\nid: {doc_id}\nalias: home\n---\n\n[id={para_id}]\n[このカード](doc:home)\n"
    );
    let out = build(&src).expect("doc: to own alias must resolve in single-file build");
    let target = NodeId(doc_id);
    assert!(matches!(&out.graph.nodes[&target].payload, NodePayload::Document(_)));
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == target && e.rel == Rel::RefersTo));
}

#[test]
fn single_file_build_resolves_doc_ref_by_ulid() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\n---\n\n[id={para_id}]\n[x](doc:{doc_id})\n");
    let out = build(&src).expect("doc: by ULID must always resolve");
    assert!(out.graph.edges.iter().any(|e| e.to == NodeId(doc_id) && e.rel == Rel::RefersTo));
}

#[test]
fn single_file_build_doc_ref_to_other_alias_needs_workspace() {
    let doc_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\nalias: home\n---\n\n[他のカード](doc:other-card)\n");
    let errors = build(&src).expect_err("doc: to a foreign alias without --workspace must fail");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::DocRefNeedsWorkspace { alias, .. } if alias == "other-card")),
        "{:?}",
        errors
    );
}

#[test]
fn workspace_build_resolves_doc_ref_across_members() {
    let home_id = Ulid::new();
    let typed_links_id = Ulid::new();
    let para_id = Ulid::new();

    let home_src =
        format!("---\nid: {home_id}\nalias: home\n---\n\n[id={para_id}]\n[型付きリンク](doc:typed-links)\n");
    let heading_id = Ulid::new();
    let tl_src = format!("---\nid: {typed_links_id}\nalias: typed-links\n---\n\n# 型付きリンク {{#{heading_id}}}\n");

    let out = build_workspace(&[member("home.sml", home_src), member("typed-links.sml", tl_src)])
        .expect("workspace build must resolve doc: across members");

    let target = NodeId(typed_links_id);
    assert!(matches!(&out.graph.nodes[&target].payload, NodePayload::Document(_)));
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == target && e.rel == Rel::RefersTo));
}

#[test]
fn workspace_build_unknown_doc_alias_is_diagnosed() {
    let home_id = Ulid::new();
    let home_src = format!("---\nid: {home_id}\nalias: home\n---\n\n[x](doc:no-such-card)\n");

    let errors = build_workspace(&[member("home.sml", home_src)]).expect_err("unknown doc alias must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            strata_build::WorkspaceError::Member { error: BuildError::UnknownDoc { alias, .. }, .. }
                if alias == "no-such-card"
        )),
        "{:?}",
        errors
    );
}

/// `doc:` は Document ノードだけを指せる。ブロック alias を `doc:` で参照すると
/// `RefTypeMismatch`(黙認しない、D14 と同じ方針)。
#[test]
fn doc_ref_to_non_document_node_is_ref_type_mismatch() {
    let doc_id = Ulid::new();
    let section_id = Ulid::new();
    let src = format!(
        "---\nid: {doc_id}\nalias: home\n---\n\n# 見出し {{#{section_id} alias=not-a-doc}}\n\n[x](doc:not-a-doc)\n"
    );
    let errors = build(&src).expect_err("doc: pointing at a non-document alias must fail");
    assert!(errors.iter().any(|e| matches!(e, BuildError::RefTypeMismatch { .. })), "{:?}", errors);
}

/// ワークスペース build でも自文書 alias(ローカル alias 表)を優先的に解決できる
/// (`doc_index` を引く前に `reg.alias_table` を見るため、他文書に同名 alias が
/// 存在してもここでは自分自身を指す)。
#[test]
fn workspace_build_self_reference_resolves_locally() {
    let home_id = Ulid::new();
    let other_id = Ulid::new();
    let para_id = Ulid::new();
    let home_src = format!("---\nid: {home_id}\nalias: home\n---\n\n[id={para_id}]\n[自分自身](doc:home)\n");
    let other_heading_id = Ulid::new();
    let other_src = format!("---\nid: {other_id}\nalias: other\n---\n\n# X {{#{other_heading_id}}}\n");

    let out = build_workspace(&[member("home.sml", home_src), member("other.sml", other_src)])
        .expect("workspace build must succeed");
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == NodeId(home_id) && e.rel == Rel::RefersTo));
}
