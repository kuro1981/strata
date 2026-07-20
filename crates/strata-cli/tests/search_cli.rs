//! `strata-cli search` サブコマンドの統合テスト(D56、sml-spec.md §1.16)。
//!
//! fmt_cli.rs / context_cli.rs と同じ流儀: 単一ファイルは凍結フィクスチャ
//! (`docs/sml_example_formatted.sml`、無改変)をコピーして使う。ワークスペースは
//! workspace_cli.rs 同様、その場で合成した小さな2文書ワークスペースを使う。

mod common;

use common::*;

fn copy_formatted_to(tmp: &TempDir) -> std::path::PathBuf {
    let src = repo_root().join("docs/sml_example_formatted.sml");
    assert!(src.exists(), "fixture missing: {}", src.display());
    let dst = tmp.path().join("formatted.sml");
    std::fs::copy(&src, &dst).expect("copy fixture");
    dst
}

// --- 単一ファイル: 素のテキスト検索 --------------------------------------------------

#[test]
fn plain_text_search_hits_and_marks_snippet() {
    let tmp = TempDir::new("search-plain");
    let file = copy_formatted_to(&tmp);

    let out = run(&["search", "予測精度", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let stdout = stdout_str(&out);
    assert!(stdout.contains("[[予測精度]]"), "{stdout}");
    assert!(stdout.contains("eval-table") || stdout.contains("perf-chart"), "{stdout}");
}

#[test]
fn no_hits_prints_no_results_marker_and_still_exits_zero() {
    let tmp = TempDir::new("search-nohit");
    let file = copy_formatted_to(&tmp);

    let out = run(&["search", "そんざいしないぶんしょう", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(stdout_str(&out).contains("該当なし"));
}

// --- 構造述語 ------------------------------------------------------------------------

#[test]
fn alias_prefix_predicate_filters_by_alias() {
    let tmp = TempDir::new("search-alias");
    let file = copy_formatted_to(&tmp);

    let out = run(&["search", "alias:eval", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let stdout = stdout_str(&out);
    assert!(stdout.contains("eval-table"), "{stdout}");
    assert!(!stdout.contains("perf-chart"), "'eval' の接頭辞に一致しないはず: {stdout}");
}

#[test]
fn term_predicate_finds_the_term_node_and_its_usage() {
    let tmp = TempDir::new("search-term");
    let file = copy_formatted_to(&tmp);

    let out = run(&["search", "term:予測精度", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let stdout = stdout_str(&out);
    assert!(stdout.contains("[term]"), "{stdout}");
}

// --- --json --------------------------------------------------------------------------

#[test]
fn json_output_is_a_hit_array_with_expected_fields() {
    let tmp = TempDir::new("search-json");
    let file = copy_formatted_to(&tmp);

    let out = run(&["search", "予測精度", file.to_str().unwrap(), "--json"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let hits: Vec<strata_search::Hit> =
        serde_json::from_str(&stdout_str(&out)).expect("stdout must be a Vec<Hit> JSON array");
    assert!(!hits.is_empty());
    assert!(hits.iter().any(|h| h.label.contains("予測精度")), "{hits:?}");
    // 単一ファイル検索でも所属文書はファイルパスとして解決される。
    assert!(hits.iter().all(|h| h.doc.as_ref().is_none_or(|d| d.path.contains("formatted.sml"))), "{hits:?}");
}

// --- --limit ---------------------------------------------------------------------------

#[test]
fn limit_caps_the_number_of_hits() {
    let tmp = TempDir::new("search-limit");
    let file = copy_formatted_to(&tmp);

    // 表・図・見出し等、複数ブロックにまたがって出現する語で母数を確保する。
    let out = run(&["search", "モデル", file.to_str().unwrap(), "--json", "--limit", "1"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let hits: Vec<strata_search::Hit> = serde_json::from_str(&stdout_str(&out)).unwrap();
    assert_eq!(hits.len(), 1, "{hits:?}");
}

// --- --switcher ------------------------------------------------------------------------

#[test]
fn switcher_mode_restricts_to_headings_and_returns_switcher_hit_json() {
    let tmp = TempDir::new("search-switcher");
    let file = copy_formatted_to(&tmp);

    let out = run(&["search", "評価結果", file.to_str().unwrap(), "--switcher", "--json"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let hits: Vec<strata_search::SwitcherHit> =
        serde_json::from_str(&stdout_str(&out)).expect("stdout must be a Vec<SwitcherHit> JSON array");
    assert!(!hits.is_empty());
    assert!(hits.iter().all(|h| h.node_type == "section" || h.node_type == "document"), "{hits:?}");
}

// --- ワークスペース ----------------------------------------------------------------------

fn write_two_doc_workspace(tmp: &TempDir) -> std::path::PathBuf {
    let toml_path = tmp.path().join("strata.toml");
    std::fs::write(&toml_path, "[workspace]\nmembers = [\"a.sml\", \"b.sml\"]\n").unwrap();

    let a_path = tmp.path().join("a.sml");
    std::fs::write(
        &a_path,
        "---\ntitle: 文書A\nalias: doca\n---\n\n# 文書A\n\n参照: [work-detail](ref:docb/work-detail)。\n\n## セクションA\n\nプロジェクトαについての内容A。\n",
    )
    .unwrap();

    let b_path = tmp.path().join("b.sml");
    std::fs::write(
        &b_path,
        "---\ntitle: 文書B\nalias: docb\n---\n\n# 文書B\n\n## 詳細 {#work-detail}\n\nプロジェクトβの work detail here.\n",
    )
    .unwrap();

    for p in [&a_path, &b_path] {
        let out = run(&["fmt", p.to_str().unwrap()]);
        assert_eq!(exit_code(&out), 0, "fmt must succeed: {}", stderr_str(&out));
    }
    toml_path
}

#[test]
fn workspace_search_finds_hits_across_members_with_doc_alias() {
    let tmp = TempDir::new("search-ws");
    let toml = write_two_doc_workspace(&tmp);

    let out = run(&["search", "プロジェクト", "--workspace", toml.to_str().unwrap(), "--json"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let hits: Vec<strata_search::Hit> = serde_json::from_str(&stdout_str(&out)).unwrap();
    assert_eq!(hits.len(), 2, "{hits:?}");
    let docs: Vec<Option<String>> = hits.iter().map(|h| h.doc.as_ref().map(|d| d.display().to_string())).collect();
    assert!(docs.contains(&Some("doca".to_string())), "{docs:?}");
    assert!(docs.contains(&Some("docb".to_string())), "{docs:?}");
}

#[test]
fn file_and_workspace_together_is_a_usage_error() {
    let tmp = TempDir::new("search-both");
    let toml = write_two_doc_workspace(&tmp);
    let a = tmp.path().join("a.sml");

    let out = run(&["search", "x", a.to_str().unwrap(), "--workspace", toml.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
}

// --- 未読み込みファイル ------------------------------------------------------------------

#[test]
fn unreadable_file_exits_1() {
    let out = run(&["search", "x", "/no/such/file.sml"]);
    assert_eq!(exit_code(&out), 1);
}
