//! D59(2026-07-29 裁定、sml-spec.md §1.18): Obsidian `[[wikilink]]` 対応。
//!
//! 解決方式: ワークスペース内の文書**タイトル**(frontmatter `title:` → 無ければ
//! 最初の H1 → 無ければファイル名 stem、ファイル名 stem はワークスペース build のみ)
//! との完全一致。
//!
//! - 単一ファイル build: 自文書のタイトルとの一致(自己参照)のみ解決可
//! - 単一ファイル build: 自文書以外のタイトルを指すと `WikilinkNeedsWorkspace`
//!   (`doc:` の `DocRefNeedsWorkspace` と同型)
//! - ワークスペース build: 他文書のタイトルも解決できる
//! - ワークスペース build: どのメンバーのタイトルにも一致しなければ `UnresolvedWikilink`
//! - ワークスペース build: 2つ以上のメンバーが同じタイトルを持てば `AmbiguousWikilink`
//!   (黙って先頭を選ばない)
//! - canonical グラフでは通常の Document 向け `Ref`(`rel: refers-to`)になる
//!   (wikilink 由来という情報は保持しない)

use strata_build::{build, build_workspace, BuildError, Member, WorkspaceError};
use strata_core::{NodeId, NodePayload, Rel};
use ulid::Ulid;

fn member(path: &str, src: String) -> Member {
    Member { path: path.to_string(), src }
}

#[test]
fn single_file_self_reference_by_frontmatter_title_resolves() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!(
        "---\nid: {doc_id}\ntitle: 迷路のしおり\n---\n\n[id={para_id}]\nこの文書は[[迷路のしおり]]である。\n"
    );
    let out = build(&src).expect("self-reference by own frontmatter title must resolve in single-file build");
    let target = NodeId(doc_id);
    assert!(matches!(&out.graph.nodes[&target].payload, NodePayload::Document(_)));
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == target && e.rel == Rel::RefersTo));
}

#[test]
fn single_file_self_reference_by_h1_fallback_resolves() {
    let doc_id = Ulid::new();
    let heading_id = Ulid::new();
    let para_id = Ulid::new();
    // frontmatter に title: が無いので最初の H1 がタイトルになる。
    let src = format!(
        "---\nid: {doc_id}\n---\n\n# 見出しの題 {{#{heading_id}}}\n\n[id={para_id}]\n[[見出しの題]]を参照。\n"
    );
    let out = build(&src).expect("self-reference by H1 fallback title must resolve in single-file build");
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == NodeId(doc_id) && e.rel == Rel::RefersTo));
}

#[test]
fn single_file_wikilink_to_foreign_title_needs_workspace() {
    let doc_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\ntitle: 自分の題\n---\n\n[[よその文書]]への参照。\n");
    let errors = build(&src).expect_err("wikilink to a foreign title without --workspace must fail");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::WikilinkNeedsWorkspace { title, .. } if title == "よその文書")),
        "{:?}",
        errors
    );
}

#[test]
fn workspace_build_resolves_wikilink_across_members_by_title() {
    let home_id = Ulid::new();
    let other_id = Ulid::new();
    let para_id = Ulid::new();
    let other_para_id = Ulid::new();

    let home_src = format!("---\nid: {home_id}\ntitle: ホーム\n---\n\n[id={para_id}]\n[[目的の文書]]を見よ。\n");
    let other_src = format!("---\nid: {other_id}\ntitle: 目的の文書\n---\n\n[id={other_para_id}]\n本文。\n");

    let out = build_workspace(&[member("home.sml", home_src), member("target.sml", other_src)])
        .expect("workspace build must resolve wikilink across members by title");

    let target = NodeId(other_id);
    assert!(matches!(&out.graph.nodes[&target].payload, NodePayload::Document(_)));
    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == target && e.rel == Rel::RefersTo));
}

#[test]
fn workspace_build_resolves_wikilink_via_h1_fallback_title() {
    let home_id = Ulid::new();
    let other_id = Ulid::new();
    let heading_id = Ulid::new();
    let para_id = Ulid::new();
    let other_para_id = Ulid::new();

    let home_src = format!("---\nid: {home_id}\ntitle: ホーム\n---\n\n[id={para_id}]\n[[見出しタイトル]]を見よ。\n");
    // target.sml は title: を持たず、最初の H1 がタイトルになる。
    let other_src =
        format!("---\nid: {other_id}\n---\n\n# 見出しタイトル {{#{heading_id}}}\n\n[id={other_para_id}]\n本文。\n");

    let out = build_workspace(&[member("home.sml", home_src), member("target.sml", other_src)])
        .expect("workspace build must resolve wikilink via H1 fallback title");

    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == NodeId(other_id) && e.rel == Rel::RefersTo));
}

#[test]
fn workspace_build_resolves_wikilink_via_filename_stem_fallback() {
    let home_id = Ulid::new();
    let other_id = Ulid::new();
    let para_id = Ulid::new();
    let other_para_id = Ulid::new();

    let home_src = format!("---\nid: {home_id}\ntitle: ホーム\n---\n\n[id={para_id}]\n[[filename-only]]を見よ。\n");
    // target.sml は title: も H1 も持たない。ファイル名 stem(拡張子抜き)が
    // タイトルのフォールバックになる(D59、ワークスペース build のみ)。
    let other_src = format!("---\nid: {other_id}\n---\n\n[id={other_para_id}]\n本文だけ。\n");

    let out = build_workspace(&[member("home.sml", home_src), member("filename-only.sml", other_src)])
        .expect("workspace build must resolve wikilink via filename stem fallback");

    assert!(out.graph.edges.iter().any(|e| e.from == NodeId(para_id) && e.to == NodeId(other_id) && e.rel == Rel::RefersTo));
}

#[test]
fn workspace_build_unresolved_wikilink_is_diagnosed() {
    let home_id = Ulid::new();
    let home_src = format!("---\nid: {home_id}\ntitle: ホーム\n---\n\n[[存在しない文書]]を見よ。\n");

    let errors = build_workspace(&[member("home.sml", home_src)]).expect_err("unresolved wikilink must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            WorkspaceError::Member { error: BuildError::UnresolvedWikilink { title, .. }, .. }
                if title == "存在しない文書"
        )),
        "{:?}",
        errors
    );
}

/// 同名衝突(D59): 2つ以上のメンバーが同じタイトルを持つ場合、黙って先頭を選ばず
/// 曖昧性を診断 Error で報告する。
#[test]
fn workspace_build_ambiguous_title_collision_is_diagnosed() {
    let home_id = Ulid::new();
    let dup1_id = Ulid::new();
    let dup2_id = Ulid::new();
    let dup1_para_id = Ulid::new();
    let dup2_para_id = Ulid::new();

    let home_src = format!("---\nid: {home_id}\ntitle: ホーム\n---\n\n[[重複タイトル]]を見よ。\n");
    let dup1_src = format!("---\nid: {dup1_id}\ntitle: 重複タイトル\n---\n\n[id={dup1_para_id}]\n本文A。\n");
    let dup2_src = format!("---\nid: {dup2_id}\ntitle: 重複タイトル\n---\n\n[id={dup2_para_id}]\n本文B。\n");

    let errors = build_workspace(&[member("home.sml", home_src), member("a.sml", dup1_src), member("b.sml", dup2_src)])
        .expect_err("ambiguous title collision must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            WorkspaceError::Member { error: BuildError::AmbiguousWikilink { title, .. }, .. }
                if title == "重複タイトル"
        )),
        "{:?}",
        errors
    );
}

/// 同名衝突の判定は自己参照にも一貫して適用される(裁量、最終報告参照):
/// 自分自身を含めて同じタイトルの文書が複数あれば、たとえ自己参照であっても曖昧と
/// 報告する(タイトルの一意性はワークスペース全体の性質として判定する)。
#[test]
fn ambiguous_title_collision_applies_even_to_self_reference() {
    let home_id = Ulid::new();
    let dup_id = Ulid::new();
    let dup_para_id = Ulid::new();

    let home_src = format!("---\nid: {home_id}\ntitle: 同じ題\n---\n\n[[同じ題]]という自己参照。\n");
    let dup_src = format!("---\nid: {dup_id}\ntitle: 同じ題\n---\n\n[id={dup_para_id}]\n別文書。\n");

    let errors = build_workspace(&[member("home.sml", home_src), member("dup.sml", dup_src)])
        .expect_err("ambiguous title collision must fail even for a self-reference");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            WorkspaceError::Member { error: BuildError::AmbiguousWikilink { title, .. }, .. }
                if title == "同じ題"
        )),
        "{:?}",
        errors
    );
}

#[test]
fn piped_wikilink_display_text_is_used_but_resolution_uses_target_title() {
    let home_id = Ulid::new();
    let other_id = Ulid::new();
    let para_id = Ulid::new();
    let other_para_id = Ulid::new();

    let home_src =
        format!("---\nid: {home_id}\ntitle: ホーム\n---\n\n[id={para_id}]\n[[目的の文書|ここ]]をクリック。\n");
    let other_src = format!("---\nid: {other_id}\ntitle: 目的の文書\n---\n\n[id={other_para_id}]\n本文。\n");

    let out = build_workspace(&[member("home.sml", home_src), member("target.sml", other_src)])
        .expect("piped wikilink must resolve against the target (pre-pipe) title");

    let inline_ref_text = out
        .graph
        .nodes
        .get(&NodeId(para_id))
        .and_then(|n| match &n.payload {
            NodePayload::Para(p) => p.inline.iter().find_map(|i| match i {
                strata_core::Inline::Ref { to, text, .. } if *to == NodeId(other_id) => Some(text.clone()),
                _ => None,
            }),
            _ => None,
        });
    assert_eq!(inline_ref_text.as_deref(), Some("ここ"));
}

/// wikilink は解決後、通常の Document 向け `doc:` 参照と区別のつかない canonical
/// 表現(`Edge(rel: refers-to)`、`Inline::Ref`)になる(D59: wikilink 由来という
/// ソース記法の情報は保持しない)。
#[test]
fn resolved_wikilink_is_indistinguishable_from_doc_scheme_ref_in_canonical_graph() {
    let home_id = Ulid::new();
    let other_id = Ulid::new();
    let para_id = Ulid::new();
    let other_para_id = Ulid::new();

    let wikilink_src =
        format!("---\nid: {home_id}\nalias: home\ntitle: ホーム\n---\n\n[id={para_id}]\n[[目的の文書]]。\n");
    let doc_scheme_src =
        format!("---\nid: {home_id}\nalias: home\ntitle: ホーム\n---\n\n[id={para_id}]\n[x](doc:target)。\n");
    let target_src =
        format!("---\nid: {other_id}\nalias: target\ntitle: 目的の文書\n---\n\n[id={other_para_id}]\n本文。\n");

    let via_wikilink =
        build_workspace(&[member("home.sml", wikilink_src), member("target.sml", target_src.clone())]).unwrap();
    let via_doc_scheme =
        build_workspace(&[member("home.sml", doc_scheme_src), member("target.sml", target_src)]).unwrap();

    let edges_of = |out: &strata_build::WorkspaceBuildOutput| {
        out.graph.edges.iter().filter(|e| e.rel == Rel::RefersTo).map(|e| (e.from, e.to)).collect::<Vec<_>>()
    };
    assert_eq!(edges_of(&via_wikilink), edges_of(&via_doc_scheme));
}
