//! `strata-cli render --workspace` / `context --workspace` の統合テスト
//! (WP-Z1、sml-spec.md §1.11 D44)。fmt_cli.rs / render_cli.rs と同じ流儀。
//!
//! fixture は改版禁止(docs/sml_example_*)のため、ここでは小さな合成ワークスペース
//! (2文書、cross-doc `ref:` 参照1本)を各テストで一時ディレクトリに書き下ろす。

mod common;

use common::*;
use std::path::PathBuf;

/// 2文書ワークスペース(a.sml → b.sml への cross-doc 参照)を一時ディレクトリに
/// フォーマット済みの状態で書き出す。`strata fmt` を経由して ULID を発行させてから
/// 使う(ドラフト記法をそのままテストに埋め込まない、他の *_cli.rs と同じ方針)。
fn write_two_doc_workspace(tmp: &TempDir) -> PathBuf {
    let toml_path = tmp.path().join("strata.toml");
    std::fs::write(&toml_path, "[workspace]\nmembers = [\"a.sml\", \"b.sml\"]\n").unwrap();

    let a_path = tmp.path().join("a.sml");
    std::fs::write(
        &a_path,
        "---\ntitle: 文書A\nalias: doca\n---\n\n# 文書A\n\n参照: [work-detail](ref:docb/work-detail)。\n\n## セクションA\n\n内容A。\n",
    )
    .unwrap();

    let b_path = tmp.path().join("b.sml");
    std::fs::write(
        &b_path,
        "---\ntitle: 文書B\nalias: docb\n---\n\n# 文書B\n\n## 詳細 {#work-detail}\n\nwork detail here.\n",
    )
    .unwrap();

    for p in [&a_path, &b_path] {
        let out = run(&["fmt", p.to_str().unwrap()]);
        assert_eq!(exit_code(&out), 0, "fmt must succeed: {}", stderr_str(&out));
    }
    toml_path
}

// --- render --workspace (D44) -----------------------------------------------------

#[test]
fn render_workspace_typst_writes_one_file_per_member_with_degenerate_cross_doc_text() {
    let tmp = TempDir::new("render-ws-typst");
    let toml = write_two_doc_workspace(&tmp);
    let outdir = tmp.path().join("out");

    let out = run(&[
        "render",
        "--workspace",
        toml.to_str().unwrap(),
        "--format",
        "typst",
        "-o",
        outdir.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));

    let a_typ = std::fs::read_to_string(outdir.join("a.typ")).expect("a.typ must exist");
    let b_typ = std::fs::read_to_string(outdir.join("b.typ")).expect("b.typ must exist");
    // D44: Typst は他文書へ実リンクを張れないため、文書名付きの退化テキストになる。
    assert!(a_typ.contains("work-detail（文書B）"), "{a_typ}");
    assert!(!a_typ.contains("#link"), "cross-doc 参照はリンクにならないはず: {a_typ}");
    assert!(b_typ.contains("詳細"));
}

#[test]
fn render_workspace_md_writes_relative_link_with_anchor_for_cross_doc_heading_ref() {
    let tmp = TempDir::new("render-ws-md");
    let toml = write_two_doc_workspace(&tmp);
    let outdir = tmp.path().join("out");

    let out = run(&[
        "render",
        "--workspace",
        toml.to_str().unwrap(),
        "--format",
        "md",
        "-o",
        outdir.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));

    let a_md = std::fs::read_to_string(outdir.join("a.md")).expect("a.md must exist");
    // D44: MD のクロスドキュメント見出し参照は相対 .md リンク+アンカー。
    assert!(a_md.contains("[work-detail](b.md#詳細)"), "{a_md}");

    let b_md = std::fs::read_to_string(outdir.join("b.md")).expect("b.md must exist");
    assert!(b_md.contains("## 詳細"));
}

#[test]
fn render_workspace_doc_flag_restricts_output_to_one_member() {
    let tmp = TempDir::new("render-ws-doc");
    let toml = write_two_doc_workspace(&tmp);
    let outdir = tmp.path().join("out");

    let out = run(&[
        "render",
        "--workspace",
        toml.to_str().unwrap(),
        "--doc",
        "docb",
        "--format",
        "md",
        "-o",
        outdir.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(outdir.join("b.md").exists());
    assert!(!outdir.join("a.md").exists(), "--doc docb は a.md を書かないはず");
}

#[test]
fn render_workspace_unknown_doc_alias_exits_2() {
    let tmp = TempDir::new("render-ws-baddoc");
    let toml = write_two_doc_workspace(&tmp);

    let out = run(&["render", "--workspace", toml.to_str().unwrap(), "--doc", "no-such-doc"]);
    assert_eq!(exit_code(&out), 2);
    assert!(stderr_str(&out).contains("no-such-doc"));
}

/// このワークスペースの `a.sml` は単一ファイル `build`/`render` では
/// `CrossDocRef` エラーになる(M7 の既知副作用) — その案内文言に
/// `render --workspace` も含まれること(WP-Z1 item 4、文言更新の確認)。
#[test]
fn single_file_render_of_a_cross_doc_member_mentions_render_workspace_in_guidance() {
    let tmp = TempDir::new("render-crossdoc-guidance");
    let _toml = write_two_doc_workspace(&tmp);
    let a_path = tmp.path().join("a.sml");

    let out = run(&["render", a_path.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    let stderr = stderr_str(&out);
    assert!(stderr.contains("CrossDocRef"), "{stderr}");
    assert!(stderr.contains("render --workspace"), "{stderr}");
}

// --- context --workspace (D44) -----------------------------------------------------

#[test]
fn context_workspace_with_no_scope_concatenates_every_member() {
    let tmp = TempDir::new("context-ws-all");
    let toml = write_two_doc_workspace(&tmp);

    let out = run(&["context", "--workspace", toml.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let stdout = stdout_str(&out);
    assert!(stdout.contains("文書A"));
    assert!(stdout.contains("文書B"));
    // cross-doc の意味エッジ(refers-to)がワークスペース全体で1回出ること。
    assert_eq!(stdout.matches("refers-to:").count(), 1, "{stdout}");
}

#[test]
fn context_workspace_doc_flag_restricts_to_one_member_and_excludes_other_docs_orphans() {
    let tmp = TempDir::new("context-ws-doc");
    let toml = write_two_doc_workspace(&tmp);

    let out = run(&["context", "--workspace", toml.to_str().unwrap(), "--doc", "doca"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let stdout = stdout_str(&out);
    assert!(stdout.contains("文書A"));
    assert!(!stdout.contains("文書B"), "--doc doca は文書Bの本文を含まないはず: {stdout}");
}

#[test]
fn context_workspace_node_scope_crosses_document_boundary() {
    let tmp = TempDir::new("context-ws-node");
    let toml = write_two_doc_workspace(&tmp);

    let out = run(&["context", "--workspace", toml.to_str().unwrap(), "--node", "work-detail", "--hops", "1"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let stdout = stdout_str(&out);
    assert!(stdout.contains("詳細"), "chunk 本体(文書Bの見出し): {stdout}");
    assert!(stdout.contains("参照: work-detail"), "文書Aからの近傍(文書境界を跨ぐ): {stdout}");
}
