//! `strata-cli context` サブコマンドの統合テスト(M5-A、D36、
//! docs/context-m5a-handoff.md WP-A2)。
//!
//! fmt_cli.rs / build_cli.rs / render_cli.rs と同じ流儀: fixture を一時ディレクトリに
//! コピーして実行し、exit code・stdout/ファイル内容・エラー時の stderr 表示を検証する。
//! 全文書スコープのゴールデン比較は `docs/sml_example_formatted.context.md`
//! (strata-context 側のゴールデンと同一ファイル、二重管理を避ける)と突き合わせる。

mod common;

use common::*;

fn copy_formatted_to(tmp: &TempDir) -> std::path::PathBuf {
    let src = repo_root().join("docs/sml_example_formatted.sml");
    assert!(src.exists(), "fixture missing: {}", src.display());
    let dst = tmp.path().join("formatted.sml");
    std::fs::copy(&src, &dst).expect("copy fixture");
    dst
}

fn golden_context_md() -> String {
    let path = repo_root().join("docs/sml_example_formatted.context.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden fixture missing at {}: {e}", path.display()))
}

/// 無指定(全文書スコープ): exit 0、stdout が golden .context.md と完全一致。
#[test]
fn context_formatted_fixture_succeeds_and_matches_golden() {
    let tmp = TempDir::new("context-formatted");
    let file = copy_formatted_to(&tmp);

    let out = run(&["context", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert_eq!(stdout_str(&out), golden_context_md());
}

/// `-o`: 原子的にファイルへ書き込み、内容は golden と一致。stdout は空。
#[test]
fn context_dash_o_writes_file_atomically_and_matches_golden() {
    let tmp = TempDir::new("context-dash-o");
    let file = copy_formatted_to(&tmp);
    let out_path = tmp.path().join("out.md");

    let out = run(&["context", file.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(stdout_str(&out).is_empty(), "with -o, context Markdown goes to the file, not stdout");

    let written = std::fs::read_to_string(&out_path).expect("output file must exist");
    assert_eq!(written, golden_context_md());
}

/// `--node <alias> --hops 1`: チャンク本体に対象ノードの内容が出て、他セクションは出ない。
#[test]
fn context_node_scope_extracts_only_the_requested_subtree() {
    let tmp = TempDir::new("context-node");
    let file = copy_formatted_to(&tmp);

    let out = run(&["context", file.to_str().unwrap(), "--node", "eval-table"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let text = stdout_str(&out);
    assert!(text.contains("Baseline-v1 | Dataset-A.F1-Score: 0.82"));
    assert!(!text.contains("予測モデル の性能評価結果について報告する"));
}

/// `--node` に存在しない alias/ULID を渡すと、明確なエラーで exit 2(handoff 必須要件)。
#[test]
fn context_unknown_node_ref_exits_2_with_clear_error() {
    let tmp = TempDir::new("context-unknown-node");
    let file = copy_formatted_to(&tmp);

    let out = run(&["context", file.to_str().unwrap(), "--node", "no-such-alias"]);
    assert_eq!(exit_code(&out), 2);
    assert!(stdout_str(&out).is_empty(), "stdout must be empty on error");
    let err = stderr_str(&out);
    assert!(err.contains("ContextError"), "stderr: {err}");
    assert!(err.contains("no-such-alias"), "stderr: {err}");
}

/// `--class note`: class を持つブロックだけが横断列挙される。
#[test]
fn context_class_scope_extracts_classed_blocks_with_location() {
    let tmp = TempDir::new("context-class");
    let file = tmp.path().join("doc.sml");
    let src = "\
---
id: 01J2T8Z1000000000000000000
---

# 職務経歴 {#01J2T8Z2000000000000000000}

[id=01J2T8Z3000000000000000000, class=note]
【補足】これは面接用のメモで実名を含む。

[id=01J2T8Z4000000000000000000]
通常の本文。
";
    std::fs::write(&file, src).unwrap();

    let out = run(&["context", file.to_str().unwrap(), "--class", "note"]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let text = stdout_str(&out);
    assert!(text.contains("【補足】"));
    assert!(!text.contains("通常の本文"));
    assert!(text.contains("位置: 職務経歴"));
}

/// フロントマター無し(root: None)で全文書スコープを要求すると exit 2。
#[test]
fn context_without_frontmatter_and_no_scope_exits_2() {
    let tmp = TempDir::new("context-no-frontmatter");
    let file = tmp.path().join("no_front.sml");
    let src = "# 見出し {#01J2T8Z1000000000000000000}\n\n\
               [id=01J2T8Z3000000000000000000]\n本文です。\n";
    std::fs::write(&file, src).unwrap();

    let out = run(&["context", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert!(stdout_str(&out).is_empty(), "stdout must be empty on error");
    assert!(stderr_str(&out).contains("ContextError"));
}

/// 存在しないファイル → exit 1(読み取り失敗)。
#[test]
fn context_missing_file_exits_1() {
    let out = run(&["context", "/nonexistent/no-such.sml"]);
    assert_eq!(exit_code(&out), 1);
    assert!(stderr_str(&out).contains("Failed to read input file"));
}

/// 既存フローの退行がないこと: `context` 追加後も `fmt` / `build` / `render` が動作する。
#[test]
fn context_addition_does_not_regress_other_subcommands() {
    let tmp = TempDir::new("context-no-regress");
    let file = copy_formatted_to(&tmp);

    let render_out = run(&["render", file.to_str().unwrap()]);
    assert_eq!(exit_code(&render_out), 0, "stderr: {}", stderr_str(&render_out));

    let build_out = run(&["build", file.to_str().unwrap()]);
    assert_eq!(exit_code(&build_out), 0, "stderr: {}", stderr_str(&build_out));

    let fmt_out = run(&["fmt", file.to_str().unwrap(), "--check"]);
    assert_eq!(exit_code(&fmt_out), 0, "already formatted, stderr: {}", stderr_str(&fmt_out));
}
