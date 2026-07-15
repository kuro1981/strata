//! `strata-cli render` サブコマンドの統合テスト(docs/sml-render-m4-handoff.md WP-R3)。
//!
//! fmt_cli.rs / build_cli.rs と同じ流儀: fixture を一時ディレクトリにコピーして
//! 実行し、exit code・stdout/ファイル内容・エラー時の stderr 表示を検証する。
//! ゴールデン比較は `docs/sml_example_formatted.typ`(WP-R2 で追加した成果物)と
//! 突き合わせる — strata-typst 側のゴールデンテストと同一ファイルを使い、二重管理を
//! 避ける。

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

fn golden_typ() -> String {
    let path = repo_root().join("docs/sml_example_formatted.typ");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden fixture missing at {}: {e}", path.display()))
}

/// formatted fixture の render → exit 0、stdout が `docs/sml_example_formatted.typ`
/// と完全一致(strata-typst のゴールデン契約テストと同じ成果物を CLI 経由でも固定する)。
#[test]
fn render_formatted_fixture_succeeds_and_matches_golden_typ() {
    let tmp = TempDir::new("render-formatted");
    let file = copy_formatted_to(&tmp);

    let out = run(&["render", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert_eq!(stdout_str(&out), golden_typ());

    // 元ファイルは触られていないこと。
    let src = std::fs::read_to_string(&file).unwrap();
    let original = std::fs::read_to_string(repo_root().join("docs/sml_example_formatted.sml")).unwrap();
    assert_eq!(src, original);
}

/// draft fixture(ULID 未付与)→ exit 2、stderr に `MissingId` と
/// 「strata fmt を先に実行してください」という案内が含まれる。stdout は空。
#[test]
fn render_draft_fixture_exits_2_with_missing_id_guidance() {
    let tmp = TempDir::new("render-draft");
    let file = copy_draft_to(&tmp);

    let out = run(&["render", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert!(stdout_str(&out).is_empty(), "stdout must be empty on error");

    let err = stderr_str(&out);
    assert!(err.contains("MissingId"), "stderr: {err}");
    assert!(err.contains("strata fmt"), "stderr should guide to run strata fmt: {err}");
}

/// フロントマター無し・全ブロック ULID 手書きの合成入力(build 自体は成功し
/// `root: None` になる)→ exit 2、D21 の案内文言("フロントマターがありません。
/// `strata fmt` を先に実行してください")。
#[test]
fn render_without_frontmatter_exits_2_with_document_guidance() {
    let tmp = TempDir::new("render-no-frontmatter");
    let file = tmp.path().join("no_front.sml");
    // フロントマター無し。全ブロックに ULID が手書きで付与済み(MissingId は出ない)。
    let src = "# 見出し {#01J2T8Z1000000000000000000}\n\n\
               [id=01J2T8Z3000000000000000000]\n本文です。\n";
    std::fs::write(&file, src).unwrap();

    // 前提確認: build 自体は成功し root: None になること。
    let build_out = run(&["build", file.to_str().unwrap()]);
    assert_eq!(exit_code(&build_out), 0, "precondition: build must succeed; stderr: {}", stderr_str(&build_out));
    let build_json: strata_build::BuildOutput = serde_json::from_str(&stdout_str(&build_out)).unwrap();
    assert_eq!(build_json.root, None, "precondition: no frontmatter means no Document root");

    let out = run(&["render", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert!(stdout_str(&out).is_empty(), "stdout must be empty on error");
    let err = stderr_str(&out);
    assert!(err.contains("フロントマター"), "stderr: {err}");
    assert!(err.contains("strata fmt"), "stderr should guide to run strata fmt: {err}");
}

/// `-o`: Typst ソースをファイルへ原子的に書き込む(一時ファイルが残らない)。
/// 書き込まれた内容もゴールデンと一致すること。
#[test]
fn render_dash_o_writes_file_atomically_and_matches_golden() {
    let tmp = TempDir::new("render-dash-o");
    let file = copy_formatted_to(&tmp);
    let out_path = tmp.path().join("out.typ");

    let out = run(&["render", file.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(stdout_str(&out).is_empty(), "with -o, Typst source goes to the file, not stdout");

    let written = std::fs::read_to_string(&out_path).expect("output file must exist");
    assert_eq!(written, golden_typ());

    // 一時ファイルが残っていないこと(fmt/build の write_atomic 同様)。
    let entries: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    let mut expected = vec!["formatted.sml".to_string(), "out.typ".to_string()];
    expected.sort();
    let mut got = entries;
    got.sort();
    assert_eq!(got, expected, "unexpected leftovers");
}

/// `-o` 指定時、build エラーがあればファイルは作られず exit 2。
#[test]
fn render_dash_o_does_not_write_file_on_build_error() {
    let tmp = TempDir::new("render-dash-o-error");
    let file = copy_draft_to(&tmp);
    let out_path = tmp.path().join("out.typ");

    let out = run(&["render", file.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 2);
    assert!(!out_path.exists(), "output file must not be created on build error");
}

/// 存在しないファイル → exit 1(読み取り失敗)。
#[test]
fn render_missing_file_exits_1() {
    let out = run(&["render", "/nonexistent/no-such.sml"]);
    assert_eq!(exit_code(&out), 1);
    assert!(stderr_str(&out).contains("Failed to read input file"));
}

// ---- D23(2026-07-14 裁定): `render --hide <class>` -----------------------------

/// `--hide` 無しなら確認版と同じ本文が出る(既存 render の退行がないこと)。
/// `--hide note` を付けると class=note のブロックが消え、そこへの Ref も
/// warning 付きでプレーンテキスト化される。
#[test]
fn render_hide_removes_classed_subtree_and_warns_on_dangling_ref() {
    let tmp = TempDir::new("render-hide");
    let file = tmp.path().join("doc.sml");
    let src = "\
---
id: 01J2T8Z1000000000000000000
---

# 職務経歴 {#01J2T8Z2000000000000000000}

[id=01J2T8Z3000000000000000000, class=note]
【補足】これは面接用のメモで実名を含む。

[id=01J2T8Z4000000000000000000, supports=01J2T8Z3000000000000000000]
詳細は[こちら](ref:01J2T8Z3000000000000000000)を参照。
";
    std::fs::write(&file, src).unwrap();

    // 確認版(--hide なし): 補足がそのまま残る。
    let check = run(&["render", file.to_str().unwrap()]);
    assert_eq!(exit_code(&check), 0, "stderr: {}", stderr_str(&check));
    assert!(stdout_str(&check).contains("【補足】"));

    // 提出版(--hide note): 補足が消え、そこへの Ref が warning 付きで剥がされる。
    let hidden = run(&["render", "--hide", "note", file.to_str().unwrap()]);
    assert_eq!(exit_code(&hidden), 0, "stderr: {}", stderr_str(&hidden));
    let out = stdout_str(&hidden);
    assert!(!out.contains("【補足】"), "{out}");
    assert!(out.contains("こちら"), "text 表示は残る: {out}");
    let err = stderr_str(&hidden);
    assert!(err.contains("warning"), "stderr: {err}");
    assert!(err.contains("HiddenRef"), "stderr: {err}");
}

// ---- D38(md-render-handoff.md WP-M3): `render --format md` -----------------------

fn golden_md() -> String {
    let path = repo_root().join("docs/sml_example_formatted.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("golden fixture missing at {}: {e}", path.display()))
}

/// `--format md` は既定(typst)と別の内容を出し、`docs/sml_example_formatted.md`
/// と完全一致する(strata-md のゴールデン契約テストと同じ成果物を CLI 経由でも固定)。
#[test]
fn render_format_md_matches_golden_md() {
    let tmp = TempDir::new("render-format-md");
    let file = copy_formatted_to(&tmp);

    let out = run(&["render", "--format", "md", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert_eq!(stdout_str(&out), golden_md());
}

/// `--format` を省略すると既定(typst)のまま(D19 改定の非退行)。
#[test]
fn render_without_format_flag_still_defaults_to_typst() {
    let tmp = TempDir::new("render-format-default");
    let file = copy_formatted_to(&tmp);

    let out = run(&["render", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert_eq!(stdout_str(&out), golden_typ());
}

/// D38: `{#ULID}` タグ・alias は MD 出力に一切出さない(context との役割分担)。
#[test]
fn render_format_md_never_emits_ulid_tags() {
    let tmp = TempDir::new("render-format-md-no-ulid");
    let file = copy_formatted_to(&tmp);

    let out = run(&["render", "--format", "md", file.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    let text = stdout_str(&out);
    assert!(!text.contains("{#"), "{text}");
    assert!(!text.contains("01J2T8"), "{text}");
}

/// `--format md` でも `--hide` は typst と同じ挙動(サブツリー非描画+HiddenRef 警告)。
#[test]
fn render_format_md_hide_removes_classed_subtree_and_warns_on_dangling_ref() {
    let tmp = TempDir::new("render-format-md-hide");
    let file = tmp.path().join("doc.sml");
    let src = "\
---
id: 01J2T8Z1000000000000000000
---

# 職務経歴 {#01J2T8Z2000000000000000000}

[id=01J2T8Z3000000000000000000, class=note]
【補足】これは面接用のメモで実名を含む。

[id=01J2T8Z4000000000000000000, supports=01J2T8Z3000000000000000000]
詳細は[こちら](ref:01J2T8Z3000000000000000000)を参照。
";
    std::fs::write(&file, src).unwrap();

    let check = run(&["render", "--format", "md", file.to_str().unwrap()]);
    assert_eq!(exit_code(&check), 0, "stderr: {}", stderr_str(&check));
    assert!(stdout_str(&check).contains("【補足】"));

    let hidden = run(&["render", "--format", "md", "--hide", "note", file.to_str().unwrap()]);
    assert_eq!(exit_code(&hidden), 0, "stderr: {}", stderr_str(&hidden));
    let out = stdout_str(&hidden);
    assert!(!out.contains("【補足】"), "{out}");
    assert!(out.contains("こちら"), "text 表示は残る: {out}");
    let err = stderr_str(&hidden);
    assert!(err.contains("warning"), "stderr: {err}");
    assert!(err.contains("HiddenRef"), "stderr: {err}");
}

/// `-o` 指定時も MD テキストが原子的に書き込まれる(拡張子はユーザー指定のまま、
/// 自動推測しない)。
#[test]
fn render_format_md_dash_o_writes_file_atomically_and_matches_golden() {
    let tmp = TempDir::new("render-format-md-dash-o");
    let file = copy_formatted_to(&tmp);
    let out_path = tmp.path().join("out.md");

    let out = run(&["render", "--format", "md", file.to_str().unwrap(), "-o", out_path.to_str().unwrap()]);
    assert_eq!(exit_code(&out), 0, "stderr: {}", stderr_str(&out));
    assert!(stdout_str(&out).is_empty(), "with -o, Markdown source goes to the file, not stdout");

    let written = std::fs::read_to_string(&out_path).expect("output file must exist");
    assert_eq!(written, golden_md());
}

/// 既存フローの退行がないこと: `render` 追加後も `fmt` / `build` サブコマンドが
/// 動作すること。
#[test]
fn render_addition_does_not_regress_fmt_and_build_subcommands() {
    let tmp = TempDir::new("render-no-regress");
    let draft = copy_draft_to(&tmp);

    let fmt_out = run(&["fmt", draft.to_str().unwrap()]);
    assert_eq!(exit_code(&fmt_out), 0, "stderr: {}", stderr_str(&fmt_out));

    let build_out = run(&["build", draft.to_str().unwrap()]);
    assert_eq!(exit_code(&build_out), 0, "stderr: {}", stderr_str(&build_out));
    let parsed: strata_build::BuildOutput =
        serde_json::from_str(&stdout_str(&build_out)).expect("stdout must be a BuildOutput JSON");
    assert!(parsed.root.is_some(), "fmt'd draft should now have a Document root");
}
