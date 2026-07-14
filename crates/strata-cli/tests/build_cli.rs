//! `strata-cli build` サブコマンドの統合テスト(docs/sml-build-m3-handoff.md WP-B5)。
//!
//! fmt_cli.rs と同じ流儀: fixture を一時ディレクトリにコピーして実行し、exit code・
//! 出力内容(stdout の JSON / `-o` のファイル書き込み)・エラー時の stderr 表示を検証する。

mod common;

use common::*;

fn copy_formatted_to(tmp: &TempDir) -> std::path::PathBuf {
    let src = repo_root().join("docs/sml_example_formatted.sml");
    assert!(src.exists(), "fixture missing: {}", src.display());
    let dst = tmp.path().join("formatted.sml");
    std::fs::copy(&src, &dst).expect("copy fixture");
    dst
}

fn copy_draft_to(tmp: &TempDir) -> std::path::PathBuf {
    let src = repo_root().join("docs/sml_example_draft.sml");
    assert!(src.exists(), "fixture missing: {}", src.display());
    let dst = tmp.path().join("draft.sml");
    std::fs::copy(&src, &dst).expect("copy fixture");
    dst
}

/// formatted fixture の build → exit 0、stdout が `BuildOutput` の JSON として
/// 再パース(ラウンドトリップ)できる。
#[test]
fn build_formatted_fixture_succeeds_and_json_roundtrips() {
    let tmp = TempDir::new("build-formatted");
    let file = copy_formatted_to(&tmp);

    let out = run(&["build", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));

    let stdout = stdout_str(&out);
    let parsed: strata_build::BuildOutput =
        serde_json::from_str(&stdout).expect("stdout must be a BuildOutput JSON");

    // 独立に build した結果と一致すること(ラウンドトリップの中身も確認)。
    let src = std::fs::read_to_string(&file).unwrap();
    let expected = strata_build::build(&src).expect("fixture must build");
    assert_eq!(parsed, expected);

    // 元ファイルは触られていないこと。
    assert_eq!(std::fs::read_to_string(&file).unwrap(), src);
}

/// draft fixture(ULID 未付与)→ exit 2、stderr に `MissingId` と
/// 「strata fmt を先に実行してください」という案内が全件分含まれる。stdout は空。
#[test]
fn build_draft_fixture_exits_2_with_missing_id_guidance() {
    let tmp = TempDir::new("build-draft");
    let file = copy_draft_to(&tmp);

    let out = run(&["build", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert!(stdout_str(&out).is_empty(), "stdout must be empty on error");

    let err = stderr_str(&out);
    assert!(err.contains("MissingId"), "stderr: {err}");
    assert!(err.contains("strata fmt"), "stderr should guide to run strata fmt: {err}");
    // 「行:列: 種別: メッセージ」形式であることの最低限の確認。
    assert!(
        err.lines().next().unwrap_or("").split(':').count() >= 3,
        "error lines should start with line:col: {err}"
    );
    // draft は全ブロックが ID 未付与なので、複数件の MissingId が全件収集されている。
    let missing_id_count = err.lines().filter(|l| l.contains("MissingId")).count();
    assert!(missing_id_count > 5, "expected many MissingId lines, got {missing_id_count}: {err}");
}

/// `-o`: グラフ JSON をファイルへ原子的に書き込む(一時ファイルが残らない)。
#[test]
fn build_dash_o_writes_file_atomically() {
    let tmp = TempDir::new("build-dash-o");
    let file = copy_formatted_to(&tmp);
    let out_path = tmp.path().join("out.json");

    let out = run(&["build", file.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(stdout_str(&out).is_empty(), "with -o, JSON goes to the file, not stdout");

    let written = std::fs::read_to_string(&out_path).expect("output file must exist");
    let parsed: strata_build::BuildOutput =
        serde_json::from_str(&written).expect("output file must be BuildOutput JSON");
    let src = std::fs::read_to_string(&file).unwrap();
    assert_eq!(parsed, strata_build::build(&src).unwrap());

    // 一時ファイルが残っていないこと(fmt の write_atomic 同様)。
    let entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let mut expected = vec!["formatted.sml".to_string(), "out.json".to_string()];
    expected.sort();
    let mut got = entries;
    got.sort();
    assert_eq!(got, expected, "unexpected leftovers");
}

/// `-o` 指定時、build エラーがあればファイルは作られず exit 2。
#[test]
fn build_dash_o_does_not_write_file_on_error() {
    let tmp = TempDir::new("build-dash-o-error");
    let file = copy_draft_to(&tmp);
    let out_path = tmp.path().join("out.json");

    let out = run(&["build", file.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert!(!out_path.exists(), "output file must not be created on build error");
}

/// 存在しないファイル → exit 1(読み取り失敗)。
#[test]
fn build_missing_file_exits_1() {
    let out = run(&["build", "/nonexistent/no-such.sml"]);
    assert_eq!(exit_code(&out), 1);
    assert!(stderr_str(&out).contains("Failed to read input file"));
}

/// 既存フローの退行がないこと: `fmt` サブコマンドと従来の YAML フローが `build` 追加後も
/// 動作すること(YAML フローはヘルプ表示で `--input` 必須エラーになる呼び出しを避け、
/// 代わりに `fmt` の既存動作を smoke テストする)。
#[test]
fn build_addition_does_not_regress_fmt_subcommand() {
    let tmp = TempDir::new("build-no-regress-fmt");
    let src = repo_root().join("docs/sml_example_draft.sml");
    let dst = tmp.path().join("draft.sml");
    std::fs::copy(&src, &dst).expect("copy fixture");

    let out = run(&["fmt", dst.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
}
