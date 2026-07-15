//! ゴールデン契約テスト(WP-M3)。`docs/sml_example_formatted.sml` を build →
//! `render_to_md` した結果が `docs/sml_example_formatted.md` と完全一致することを
//! 固定する(strata-typst::golden と同じ流儀、二重管理を避けるため strata-cli 側の
//! 統合テストもこの同じファイルと突き合わせる)。
//!
//! `docs/sml_example_formatted.md` は本 WP で新規に追加したゴールデン成果物であり、
//! `docs/sml_example_*.sml` フィクスチャ自体は変更していない(スコープ境界を遵守)。

use super::render_to_md;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repo root exists")
}

#[test]
fn formatted_fixture_renders_byte_for_byte_identical_to_golden_md() {
    let root = repo_root();
    let src = std::fs::read_to_string(root.join("docs/sml_example_formatted.sml")).expect("fixture readable");
    let out = strata_build::build(&src).expect("fixture must build cleanly");
    let doc_root = out.root.expect("formatted fixture has frontmatter, so Document root exists");
    let actual = render_to_md(&out.graph, doc_root, "sml_example_formatted").expect("render must succeed");

    let expected = std::fs::read_to_string(root.join("docs/sml_example_formatted.md"))
        .expect("golden fixture docs/sml_example_formatted.md must exist");

    assert_eq!(actual, expected, "render_to_md output drifted from the golden .md fixture");
    // D38: {#ULID} タグ・alias は一切出さない(context との役割分担)。
    assert!(!actual.contains("{#"));
    assert!(!actual.contains("01J2T8"));
}
