//! `strata-cli fmt` サブコマンドの統合テスト(docs/sml-fmt-m2-handoff.md WP-F3)。
//!
//! fixture(docs/sml_example_draft.sml)を一時ディレクトリにコピーして実行し、
//! exit code・ファイル内容・冪等性・パースエラー時の無傷性を検証する。

mod common;

use common::*;

fn copy_draft_to(tmp: &TempDir) -> std::path::PathBuf {
    let src = repo_root().join("docs/sml_example_draft.sml");
    assert!(src.exists(), "fixture missing: {}", src.display());
    let dst = tmp.path().join("draft.sml");
    std::fs::copy(&src, &dst).expect("copy fixture");
    dst
}

/// fmt 実行 → exit 0、ファイルが変わり、再パース可能(diags ゼロ)。
/// 再実行 → exit 0 で無変更(冪等)。
#[test]
fn fmt_formats_in_place_and_is_idempotent() {
    let tmp = TempDir::new("fmt-idempotent");
    let file = copy_draft_to(&tmp);
    let original = std::fs::read_to_string(&file).unwrap();

    let out = run(&["fmt", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));

    let formatted = std::fs::read_to_string(&file).unwrap();
    assert_ne!(formatted, original, "fmt should modify the draft fixture");

    // 再パース可能で、診断ゼロ。
    let reparsed = strata_sml::parse(&formatted);
    assert!(reparsed.diags.is_empty(), "reparsed diags: {:?}", reparsed.diags);

    // 冪等: 再実行しても exit 0 かつ無変更。
    let out2 = run(&["fmt", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out2), 0, "stderr: {}", stderr_str(&out2));
    let formatted2 = std::fs::read_to_string(&file).unwrap();
    assert_eq!(formatted2, formatted, "fmt must be idempotent");
}

/// `--check`: 未整形なら exit 1(パッチ内容を表示、ファイルは書かない)、
/// 整形済みなら exit 0。
#[test]
fn fmt_check_reports_without_writing() {
    let tmp = TempDir::new("fmt-check");
    let file = copy_draft_to(&tmp);
    let original = std::fs::read_to_string(&file).unwrap();

    // 未整形 → exit 1、パッチ内容(位置と挿入/置換テキスト)が表示され、ファイル不変。
    let out = run(&["fmt", "--check", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 1, "stderr: {}", stderr_str(&out));
    let report = stdout_str(&out);
    assert!(report.contains("insert"), "patch report should show inserts: {report}");
    assert!(
        report.lines().next().unwrap_or("").split(':').count() >= 3,
        "patch report lines should start with line:col: {report}"
    );
    assert_eq!(std::fs::read_to_string(&file).unwrap(), original, "--check must not write");

    // 整形してから --check → exit 0。
    let out_fmt = run(&["fmt", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out_fmt), 0);
    let out2 = run(&["fmt", "--check", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out2), 0, "formatted file must pass --check");
}

/// パースエラーのある .sml → exit 2、ファイル無傷、stderr に「行:列: 種別: メッセージ」を全件。
#[test]
fn fmt_parse_error_exits_2_and_leaves_file_intact() {
    let tmp = TempDir::new("fmt-parse-error");
    let file = tmp.path().join("bad.sml");
    // OrphanAttrLine(1行目)と UnclosedFence(5行目)の2診断を含む。
    let bad = "[id=foo]\n\nOrphan.\n\n::math\nx = 1\n";
    std::fs::write(&file, bad).unwrap();

    let out = run(&["fmt", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), bad, "file must be untouched");

    let err = stderr_str(&out);
    // 全件出力(2診断とも)。「行:列: 種別: メッセージ」形式。
    assert!(err.contains("1:1: OrphanAttrLine:"), "stderr: {err}");
    assert!(err.contains("5:1: UnclosedFence:"), "stderr: {err}");
}

/// --check でもパースエラーは同じ扱い(exit 2、ファイル無傷)。
#[test]
fn fmt_check_parse_error_also_exits_2() {
    let tmp = TempDir::new("fmt-check-parse-error");
    let file = tmp.path().join("bad.sml");
    let bad = "::table\n@rows:\n  - m: [a]\n"; // UnclosedFence
    std::fs::write(&file, bad).unwrap();

    let out = run(&["fmt", "--check", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), bad);
    assert!(stderr_str(&out).contains("UnclosedFence"));
}

/// 存在しないファイル → exit 1(読み取り失敗)。
#[test]
fn fmt_missing_file_exits_1() {
    let out = run(&["fmt", "/nonexistent/no-such.sml"]);
    assert_eq!(exit_code(&out), 1);
    assert!(stderr_str(&out).contains("Failed to read input file"));
}

/// 整形後、一時ファイルが残っていないこと(原子的書き込みの後始末)。
#[test]
fn fmt_leaves_no_temp_files_behind() {
    let tmp = TempDir::new("fmt-no-tmpfiles");
    let file = copy_draft_to(&tmp);

    let out = run(&["fmt", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0);

    let entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec!["draft.sml".to_string()], "unexpected leftovers: {entries:?}");
}
