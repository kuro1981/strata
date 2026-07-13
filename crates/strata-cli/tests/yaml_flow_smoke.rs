//! 既存の YAML→HTML/Typst フローのスモークテスト(退行検知)。
//! WP-F3 の手順1: fmt サブコマンド追加の前後で挙動が変わっていないことを保証する。

mod common;

use common::*;

/// リポジトリ既存の YAML fixture(vault/resume.yaml)で従来呼び出し形式が成功する。
#[test]
fn legacy_yaml_to_html_flow_succeeds() {
    let tmp = TempDir::new("yaml-html");
    let input = repo_root().join("vault/resume.yaml");
    assert!(input.exists(), "fixture missing: {}", input.display());
    let out_path = tmp.path().join("out.html");

    let out = run(&["-i", input.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);

    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(stdout_str(&out).contains("Done! Successfully compiled Strata document."));
    let html = std::fs::read_to_string(&out_path).expect("output file written");
    assert!(!html.is_empty());
}

/// `.typ` 拡張子からの Typst 推論も従来通り動く。
#[test]
fn legacy_yaml_to_typst_flow_succeeds() {
    let tmp = TempDir::new("yaml-typ");
    let input = repo_root().join("vault/resume.yaml");
    let out_path = tmp.path().join("out.typ");

    let out = run(&["-i", input.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);

    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(stdout_str(&out).contains("Rendering to Typst..."));
    assert!(out_path.exists());
}

/// 引数なしは従来通り clap の usage エラー(--input が必須)。
#[test]
fn legacy_flow_still_requires_input_flag() {
    let out = run(&[]);
    assert_ne!(exit_code(&out), 0);
    let err = stderr_str(&out);
    assert!(err.contains("--input") || err.contains("required"), "stderr: {err}");
}

/// 存在しない入力ファイルは従来通り exit 1。
#[test]
fn legacy_flow_missing_input_file_exits_1() {
    let out = run(&["-i", "/nonexistent/no-such.yaml", "-o", "/tmp/never-written.html"]);
    assert_eq!(exit_code(&out), 1);
    assert!(stderr_str(&out).contains("Failed to read input file"));
}
