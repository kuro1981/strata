//! ワークスペース層 v0(M7、D41〜D43)の統合テスト(WP-W1/WP-W2)。
//!
//! - フロントマター `alias` キーの受理と Document ノードへの反映
//! - 単一ファイル build における doc 修飾参照 → `CrossDocRef`(黙って落とさない)
//! - `build_workspace`: 横断参照解決・文書 alias 重複・ファイル間 ULID 衝突・
//!   doc 修飾の未解決(文書 alias 不明 / ブロック alias 不明)・Term の自然合流

use strata_build::{build, build_workspace, BuildError, Member, WorkspaceError};
use strata_core::{NodePayload, NodeId};
use ulid::Ulid;

fn member(path: &str, src: String) -> Member {
    Member { path: path.to_string(), src }
}

// ---- WP-W1: フロントマター alias -------------------------------------------

#[test]
fn frontmatter_alias_is_stored_on_document_node() {
    let doc_id = Ulid::new();
    let heading_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\nalias: resume\n---\n\n# Title {{#{heading_id}}}\n");
    let out = build(&src).expect("must build");
    let node = &out.graph.nodes[&NodeId(doc_id)];
    assert!(matches!(node.payload, NodePayload::Document(_)));
    assert_eq!(node.alias.as_deref(), Some("resume"));
}

#[test]
fn frontmatter_alias_bad_charset_is_diagnosed() {
    let out = strata_sml::parse("---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\nalias: bad.alias\n---\n");
    assert!(out.diags.iter().any(|d| d.kind == strata_sml::DiagKind::BadKeyCharset), "{:?}", out.diags);
}

// ---- WP-W1.3: 単一ファイル build の doc 修飾参照 → CrossDocRef --------------

#[test]
fn doc_qualified_ref_in_single_file_build_is_cross_doc_ref_error() {
    let doc_id = Ulid::new();
    let src = format!(
        "---\nid: {doc_id}\nalias: resume\n---\n\n[職務経歴書](ref:work-history/summary)\n"
    );
    let errors = build(&src).expect_err("doc-qualified ref without --workspace must fail");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::CrossDocRef { doc, alias, .. } if doc == "work-history" && alias == "summary")),
        "{:?}",
        errors
    );
}

#[test]
fn doc_qualified_supports_attr_in_single_file_build_is_cross_doc_ref_error() {
    let a = Ulid::new();
    let src = format!("[id={a}, supports=work-history/summary]\n本文。\n");
    let errors = build(&src).expect_err("doc-qualified supports= without --workspace must fail");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::CrossDocRef { doc, alias, .. } if doc == "work-history" && alias == "summary")),
        "{:?}",
        errors
    );
}

// ---- WP-W2: build_workspace の横断参照解決 ----------------------------------

#[test]
fn workspace_build_resolves_doc_qualified_ref_across_members() {
    let resume_id = Ulid::new();
    let wh_id = Ulid::new();
    let summary_id = Ulid::new();
    let para_id = Ulid::new();

    let resume_src = format!(
        "---\nid: {resume_id}\nalias: resume\n---\n\n[id={para_id}]\n[職務経歴書](ref:work-history/summary)\n"
    );
    let wh_src = format!(
        "---\nid: {wh_id}\nalias: work-history\n---\n\n# 概要 {{#{summary_id} alias=summary}}\n"
    );

    let out = build_workspace(&[member("resume.sml", resume_src), member("work_history.sml", wh_src)])
        .expect("workspace build must succeed");

    // resume.sml の Ref エッジが work_history.sml の summary セクションへ実際に張られている。
    let target = NodeId(summary_id);
    assert!(out.graph.nodes.contains_key(&target));
    assert!(out
        .graph
        .edges
        .iter()
        .any(|e| e.to == target && e.rel == strata_core::Rel::RefersTo));

    // roots は2件、どちらも解決されている。
    assert_eq!(out.roots.len(), 2);
    assert!(out.roots.iter().all(|r| r.root.is_some()));
}

#[test]
fn workspace_build_resolves_doc_qualified_supports_attr() {
    let resume_id = Ulid::new();
    let wh_id = Ulid::new();
    let claim_id = Ulid::new();
    let summary_id = Ulid::new();

    let resume_src = format!(
        "---\nid: {resume_id}\nalias: resume\n---\n\n[id={claim_id}, supports=work-history/summary]\n主張。\n"
    );
    let wh_src = format!(
        "---\nid: {wh_id}\nalias: work-history\n---\n\n# 概要 {{#{summary_id} alias=summary}}\n"
    );

    let out = build_workspace(&[member("resume.sml", resume_src), member("work_history.sml", wh_src)])
        .expect("workspace build must succeed");

    assert!(out.graph.edges.iter().any(|e| {
        e.from == NodeId(claim_id) && e.to == NodeId(summary_id) && e.rel == strata_core::Rel::Supports
    }));
}

/// 無修飾 alias は同一文書のみ(D42)。他文書のブロック alias が同名であっても、
/// 無修飾で他文書を突き抜けて解決してはいけない。
#[test]
fn unqualified_alias_does_not_leak_across_documents() {
    let a_id = Ulid::new();
    let a_block = Ulid::new();
    let b_id = Ulid::new();
    let b_block = Ulid::new();

    let para_id = Ulid::new();
    let a_src = format!(
        "---\nid: {a_id}\nalias: doc-a\n---\n\n# X {{#{a_block} alias=shared}}\n\n[id={para_id}]\n[text](ref:shared)\n"
    );
    let b_src = format!("---\nid: {b_id}\nalias: doc-b\n---\n\n# Y {{#{b_block} alias=shared}}\n");

    let out = build_workspace(&[member("a.sml", a_src), member("b.sml", b_src)]).expect("must build");
    // a.sml 内の無修飾 ref:shared は a.sml 自身の shared ブロックに解決される。
    assert!(out.graph.edges.iter().any(|e| e.to == NodeId(a_block) && e.rel == strata_core::Rel::RefersTo));
    assert!(!out.graph.edges.iter().any(|e| e.to == NodeId(b_block) && e.rel == strata_core::Rel::RefersTo));
}

// ---- WP-W2.3: 診断 ----------------------------------------------------------

#[test]
fn duplicate_doc_alias_across_members_is_diagnosed() {
    let a_id = Ulid::new();
    let b_id = Ulid::new();
    let a_src = format!("---\nid: {a_id}\nalias: dup\n---\n\n# A\n");
    let b_src = format!("---\nid: {b_id}\nalias: dup\n---\n\n# B\n");

    let errors = build_workspace(&[member("a.sml", a_src), member("b.sml", b_src)])
        .expect_err("duplicate doc alias must fail");
    assert!(
        errors.iter().any(|e| matches!(e, WorkspaceError::DuplicateDocAlias { alias, paths } if alias == "dup" && paths.len() == 2)),
        "{:?}",
        errors
    );
}

#[test]
fn cross_file_ulid_collision_is_diagnosed() {
    let shared = Ulid::new();
    let a_src = format!("# A {{#{shared}}}\n");
    let b_src = format!("# B {{#{shared}}}\n");

    let errors = build_workspace(&[member("a.sml", a_src), member("b.sml", b_src)])
        .expect_err("cross-file ULID collision must fail");
    assert!(
        errors.iter().any(|e| matches!(e, WorkspaceError::UlidCollision { id, paths } if *id == NodeId(shared) && paths.len() == 2)),
        "{:?}",
        errors
    );
}

#[test]
fn unknown_doc_alias_in_workspace_is_diagnosed() {
    let a_id = Ulid::new();
    let a_src = format!("---\nid: {a_id}\nalias: resume\n---\n\n[text](ref:no-such-doc/thing)\n");

    let errors = build_workspace(&[member("a.sml", a_src)]).expect_err("unknown doc alias must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            WorkspaceError::Member { error: BuildError::UnknownDocAlias { doc, alias, .. }, .. }
                if doc == "no-such-doc" && alias == "thing"
        )),
        "{:?}",
        errors
    );
}

#[test]
fn unknown_block_alias_in_workspace_is_diagnosed() {
    let a_id = Ulid::new();
    let b_id = Ulid::new();
    let a_src = format!("---\nid: {a_id}\nalias: resume\n---\n\n[text](ref:work-history/no-such-block)\n");
    let b_src = format!("---\nid: {b_id}\nalias: work-history\n---\n\n# X\n");

    let errors = build_workspace(&[member("a.sml", a_src), member("b.sml", b_src)])
        .expect_err("unknown block alias must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            WorkspaceError::Member { error: BuildError::UnknownBlockAlias { doc, alias, .. }, .. }
                if doc == "work-history" && alias == "no-such-block"
        )),
        "{:?}",
        errors
    );
}

// ---- WP-W2.4: Term の自然合流 -----------------------------------------------

#[test]
fn same_term_across_files_converges_to_one_node() {
    let a_id = Ulid::new();
    let b_id = Ulid::new();
    let a_para = Ulid::new();
    let b_para = Ulid::new();
    let a_src = format!("---\nid: {a_id}\n---\n\n[id={a_para}]\n[text](term:機械学習)\n");
    let b_src = format!("---\nid: {b_id}\n---\n\n[id={b_para}]\n[text](term:機械学習)\n");

    let out = build_workspace(&[member("a.sml", a_src), member("b.sml", b_src)]).expect("must build");
    let term_nodes: Vec<_> =
        out.graph.nodes.values().filter(|n| matches!(&n.payload, NodePayload::Term(t) if t.name == "機械学習")).collect();
    assert_eq!(term_nodes.len(), 1, "term ノードは1つに自然合流するはず: {:?}", term_nodes);
}

// ---- 空のワークスペース ------------------------------------------------------

#[test]
fn empty_workspace_builds_empty_graph() {
    let out = build_workspace(&[]).expect("empty workspace is valid");
    assert!(out.graph.nodes.is_empty());
    assert!(out.roots.is_empty());
}
