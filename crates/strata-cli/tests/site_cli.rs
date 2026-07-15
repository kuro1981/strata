//! `strata-cli site` サブコマンドの統合テスト(G1 WS-B、docs/graph-ui-g1-handoff.md、
//! sml-spec.md §1.13 D49/D50)。fmt_cli.rs / build_cli.rs / workspace_cli.rs と同じ流儀。
//!
//! `ui/dist` の実体(pnpm build の成果物)には依存しない: `--ui-dist` で毎回、
//! テスト用の一時ディレクトリに書いた最小限のダミー資産(index.html 等)を指す。
//! これにより `cargo test -p strata-cli` は Node/pnpm 環境が無くても常に完結する。

mod common;

use common::*;
use std::path::PathBuf;

/// ダミーの UI 資産ディレクトリ(`ui/dist` 相当)を書き出す。ネストしたファイルも
/// 1つ持たせて、コピーが再帰的であることを確認できるようにする。
fn write_dummy_ui_dist(tmp: &TempDir) -> PathBuf {
    let dist = tmp.path().join("dummy-dist");
    std::fs::create_dir_all(dist.join("assets")).unwrap();
    std::fs::write(dist.join("index.html"), "<!doctype html><title>dummy</title>").unwrap();
    std::fs::write(dist.join("favicon.svg"), "<svg/>").unwrap();
    std::fs::write(dist.join("assets/index-XXXX.js"), "console.log('dummy');").unwrap();
    dist
}

fn copy_formatted_to(tmp: &TempDir) -> PathBuf {
    let src = repo_root().join("docs/sml_example_formatted.sml");
    assert!(src.exists(), "fixture missing: {}", src.display());
    let dst = tmp.path().join("formatted.sml");
    std::fs::copy(&src, &dst).expect("copy fixture");
    dst
}

/// workspace_cli.rs と同じ流儀: 2文書の合成ワークスペースを一時ディレクトリに
/// フォーマット済みで書き出す。
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

/// `--ui-dist` が指すディレクトリが無ければ exit 2、案内文言つき。出力ディレクトリは
/// 作られない(前提条件エラーで即座に止まること)。
#[test]
fn site_missing_ui_dist_exits_2_with_guidance() {
    let tmp = TempDir::new("site-missing-ui-dist");
    let file = copy_formatted_to(&tmp);
    let outdir = tmp.path().join("out");
    let missing_dist = tmp.path().join("no-such-dist");

    let out = run(&[
        "site",
        file.to_str().unwrap(),
        "-o",
        outdir.to_str().unwrap(),
        "--ui-dist",
        missing_dist.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 2);
    let err = stderr_str(&out);
    assert!(err.contains("UI 資産が見つかりません"), "stderr: {err}");
    assert!(err.contains("pnpm build"), "stderr should guide to pnpm build: {err}");
    assert!(!outdir.exists(), "output directory must not be created on this error");
}

/// 単一ファイル: graph.json(正規化された `{graph, roots, doc_aliases}` 形)+
/// UI 資産(index.html・入れ子ファイル含む)が出力ディレクトリに合成される。
#[test]
fn site_single_file_writes_graph_json_and_copies_ui_assets() {
    let tmp = TempDir::new("site-single");
    let file = copy_formatted_to(&tmp);
    let dist = write_dummy_ui_dist(&tmp);
    let outdir = tmp.path().join("out");

    let out = run(&[
        "site",
        file.to_str().unwrap(),
        "-o",
        outdir.to_str().unwrap(),
        "--ui-dist",
        dist.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));

    // UI 資産が(入れ子含め)コピーされていること。
    assert_eq!(std::fs::read_to_string(outdir.join("index.html")).unwrap(), "<!doctype html><title>dummy</title>");
    assert_eq!(
        std::fs::read_to_string(outdir.join("assets/index-XXXX.js")).unwrap(),
        "console.log('dummy');"
    );

    // graph.json: 単一ファイルでも roots が1件、workspace 版と同じキー名の形。
    let graph_json = std::fs::read_to_string(outdir.join("graph.json")).expect("graph.json must exist");
    let parsed: serde_json::Value = serde_json::from_str(&graph_json).expect("graph.json must be valid JSON");
    assert!(parsed.get("graph").is_some(), "{parsed}");
    let roots = parsed.get("roots").and_then(|r| r.as_array()).expect("roots must be an array");
    assert_eq!(roots.len(), 1, "{parsed}");
    assert_eq!(roots[0].get("path").and_then(|p| p.as_str()), file.to_str());
    assert!(roots[0].get("root").is_some(), "single-file fixture has frontmatter, root must be Some: {parsed}");

    // graph.nodes は build 結果と一致する(中身が build をそのまま写していること)。
    let src = std::fs::read_to_string(&file).unwrap();
    let expected = strata_build::build(&src).unwrap();
    let node_count = parsed["graph"]["nodes"].as_object().unwrap().len();
    assert_eq!(node_count, expected.graph.nodes.len());
}

/// build エラー(未フォーマット draft fixture)→ exit 2、出力ディレクトリは作られない。
#[test]
fn site_build_error_exits_2_and_writes_nothing() {
    let tmp = TempDir::new("site-build-error");
    let src = repo_root().join("docs/sml_example_draft.sml");
    let dst = tmp.path().join("draft.sml");
    std::fs::copy(&src, &dst).unwrap();
    let dist = write_dummy_ui_dist(&tmp);
    let outdir = tmp.path().join("out");

    let out = run(&[
        "site",
        dst.to_str().unwrap(),
        "-o",
        outdir.to_str().unwrap(),
        "--ui-dist",
        dist.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 2);
    assert!(err_contains_missing_id(&out));
    assert!(!outdir.exists(), "output directory must not be created on build error");
}

fn err_contains_missing_id(out: &std::process::Output) -> bool {
    stderr_str(out).contains("MissingId")
}

/// ワークスペース: `WorkspaceBuildOutput` と同じ `roots`/`doc_aliases` の中身が
/// そのまま graph.json に写ること(cross-doc 参照込みの2文書ワークスペース)。
#[test]
fn site_workspace_writes_multi_root_graph_json() {
    let tmp = TempDir::new("site-workspace");
    let toml = write_two_doc_workspace(&tmp);
    let dist = write_dummy_ui_dist(&tmp);
    let outdir = tmp.path().join("out");

    let out = run(&[
        "site",
        "--workspace",
        toml.to_str().unwrap(),
        "-o",
        outdir.to_str().unwrap(),
        "--ui-dist",
        dist.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));

    assert!(outdir.join("index.html").exists());
    let graph_json = std::fs::read_to_string(outdir.join("graph.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&graph_json).unwrap();

    let roots = parsed["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 2, "{parsed}");
    let aliases: Vec<&str> = roots.iter().map(|r| r["alias"].as_str().unwrap()).collect();
    assert!(aliases.contains(&"doca"), "{aliases:?}");
    assert!(aliases.contains(&"docb"), "{aliases:?}");

    // doc_aliases: doca/docb ブロック alias の索引がそのまま出ていること
    // (a.sml → b.sml の `ref:docb/work-detail` を解決するのに使われた索引)。
    let doc_aliases = parsed["doc_aliases"].as_object().expect("doc_aliases must be present for a workspace");
    assert!(doc_aliases.contains_key("docb"), "{parsed}");
    assert!(doc_aliases["docb"].as_object().unwrap().contains_key("work-detail"), "{parsed}");
}

/// `-o` 直下に一時ファイルが残らないこと(write_atomic の慣習どおり)。
#[test]
fn site_leaves_no_temp_files_behind() {
    let tmp = TempDir::new("site-no-temp-files");
    let file = copy_formatted_to(&tmp);
    let dist = write_dummy_ui_dist(&tmp);
    let outdir = tmp.path().join("out");

    let out = run(&[
        "site",
        file.to_str().unwrap(),
        "-o",
        outdir.to_str().unwrap(),
        "--ui-dist",
        dist.to_str().unwrap(),
    ]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));

    let entries: Vec<String> =
        std::fs::read_dir(&outdir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    for name in &entries {
        assert!(!name.starts_with('.'), "leftover temp file: {name}");
    }
    let mut got = entries;
    got.sort();
    assert_eq!(got, vec!["assets", "favicon.svg", "graph.json", "index.html"]);
}
