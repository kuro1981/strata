//! image-support-handoff.md item 2: `![[target]]` / `![[target|alt]]` 画像embed対応の
//! 回帰テスト。D59 の wikilink テスト(`wikilink.rs`)と同型の構成:
//!
//! - 単一ファイル build: `assets` インデックス自体が無いので常に `AssetNeedsWorkspace`
//! - ワークスペース build: `assets` インデックスと基名一致で解決 → `Figure::Image` ノード
//! - ワークスペース build: 一致なし → `UnresolvedAsset`
//! - ワークスペース build: 同名複数一致(曖昧) → `AmbiguousAsset`
//! - 段落が embed 1つだけでない(他テキストと混在)場合は `Inline::Image` へ降格する

use std::collections::HashMap;

use strata_build::{build, build_workspace_with_assets, AssetIndex, BuildError, Member};
use strata_core::{Figure, Inline, NodeId, NodePayload};
use ulid::Ulid;

fn member(path: &str, src: String) -> Member {
    Member { path: path.to_string(), src }
}

fn assets_with(basename: &str, paths: &[&str]) -> AssetIndex {
    let mut idx: AssetIndex = HashMap::new();
    idx.insert(basename.to_string(), paths.iter().map(|p| p.to_string()).collect());
    idx
}

#[test]
fn single_file_build_reports_asset_needs_workspace() {
    let para_id = Ulid::new();
    let src = format!("[id={para_id}]\n![[foo.png]]\n");
    let errors = build(&src).expect_err("single-file build cannot resolve assets");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::AssetNeedsWorkspace { target, .. } if target == "foo.png")),
        "{errors:?}"
    );
}

#[test]
fn workspace_build_resolves_sole_embed_paragraph_into_image_figure_node() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\n---\n\n[id={para_id}]\n![[foo.png]]\n");
    let assets = assets_with("foo.png", &["/tmp/does-not-need-to-exist/foo.png"]);

    let out = build_workspace_with_assets(&[member("home.sml", src)], &assets)
        .expect("workspace build must resolve the asset embed");

    let node = &out.graph.nodes[&NodeId(para_id)];
    match &node.payload {
        NodePayload::Figure(Figure::Image(img)) => {
            assert_eq!(img.src, "/tmp/does-not-need-to-exist/foo.png");
            // alt省略時はbasenameから拡張子を除いたもの(item 2)。
            assert_eq!(img.alt, "foo");
        }
        other => panic!("expected the paragraph to be promoted to an image figure, got {other:?}"),
    }
}

#[test]
fn workspace_build_uses_explicit_alt_when_piped() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\n---\n\n[id={para_id}]\n![[foo.png|説明画像]]\n");
    let assets = assets_with("foo.png", &["/tmp/does-not-need-to-exist/foo.png"]);

    let out = build_workspace_with_assets(&[member("home.sml", src)], &assets)
        .expect("workspace build must resolve the asset embed");

    match &out.graph.nodes[&NodeId(para_id)].payload {
        NodePayload::Figure(Figure::Image(img)) => assert_eq!(img.alt, "説明画像"),
        other => panic!("expected image figure, got {other:?}"),
    }
}

#[test]
fn workspace_build_reports_unresolved_asset_when_no_file_matches() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\n---\n\n[id={para_id}]\n![[missing.png]]\n");

    let errors = build_workspace_with_assets(&[member("home.sml", src)], &AssetIndex::new())
        .expect_err("unresolved asset must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            strata_build::WorkspaceError::Member { error: BuildError::UnresolvedAsset { target, .. }, .. }
                if target == "missing.png"
        )),
        "{errors:?}"
    );
}

#[test]
fn workspace_build_reports_ambiguous_asset_when_two_files_share_a_basename() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\n---\n\n[id={para_id}]\n![[dup.png]]\n");
    let assets = assets_with("dup.png", &["/a/dup.png", "/b/dup.png"]);

    let errors =
        build_workspace_with_assets(&[member("home.sml", src)], &assets).expect_err("ambiguous asset must fail");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            strata_build::WorkspaceError::Member { error: BuildError::AmbiguousAsset { target, .. }, .. }
                if target == "dup.png"
        )),
        "{errors:?}"
    );
}

/// 画像embedが段落中の他のテキストと混在している場合は `Figure::Image` ノードへ
/// 昇格させず、通常のインライン画像(`Inline::Image`)として埋め込む(裁量、
/// image-support-handoff.md item 2 最終報告参照)。
#[test]
fn mixed_content_embed_falls_back_to_inline_image() {
    let doc_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!("---\nid: {doc_id}\n---\n\n[id={para_id}]\n見出し![[foo.png]]の後\n");
    let assets = assets_with("foo.png", &["/tmp/does-not-need-to-exist/foo.png"]);

    let out = build_workspace_with_assets(&[member("home.sml", src)], &assets)
        .expect("workspace build must resolve the asset embed");

    match &out.graph.nodes[&NodeId(para_id)].payload {
        NodePayload::Para(p) => {
            assert!(
                p.inline.iter().any(|i| matches!(i, Inline::Image { url, .. } if url == "/tmp/does-not-need-to-exist/foo.png")),
                "{:?}",
                p.inline
            );
        }
        other => panic!("expected a Para node (not promoted to Figure), got {other:?}"),
    }
}
