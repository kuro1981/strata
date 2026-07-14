//! ゴールデン契約テスト(sml-render-m4-handoff.md WP-R2)。
//!
//! `docs/sml_example_formatted.sml` を `strata_build::build` に通し、得られた
//! canonical グラフを `render_to_typst` に渡した結果が `docs/sml_example_formatted.typ`
//! と完全一致することを固定する。build は決定的(ULID はソースに書かれた固定値、
//! Term ID は名前からの決定的導出)なので、レンダラの出力も決定的に固定できる
//! (sml-render-m4-handoff.md WP-R2)。
//!
//! `docs/sml_example_formatted.typ` は本 WP で新規に追加したゴールデン成果物であり、
//! `docs/sml_example_*.sml` フィクスチャ自体は変更していない(スコープ境界を遵守)。
//! strata-cli 側の統合テスト(WP-R3)もこの同じファイルと突き合わせる — ゴールデンの
//! 二重管理を避けるため。

use super::render_to_typst;

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize().expect("repo root exists")
}

#[test]
fn formatted_fixture_renders_byte_for_byte_identical_to_golden_typ() {
    let root = repo_root();
    let src = std::fs::read_to_string(root.join("docs/sml_example_formatted.sml")).expect("fixture readable");
    let out = strata_build::build(&src).expect("fixture must build cleanly");
    let doc_root = out.root.expect("formatted fixture has frontmatter, so Document root exists");

    // フォールバック名は formatted fixture では使われない(Document.title は無いが
    // 最初の H1「機械学習モデルの評価レポート」が採用されるため)。ここでは
    // フォールバック経路を踏んでいないことも兼ねて確認する。
    let actual = render_to_typst(&out.graph, doc_root, "sml_example_formatted").expect("render must succeed");

    let expected = std::fs::read_to_string(root.join("docs/sml_example_formatted.typ"))
        .expect("golden fixture docs/sml_example_formatted.typ must exist");

    assert_eq!(actual, expected, "render_to_typst output drifted from the golden .typ fixture");
    assert!(actual.contains("#set document(title: \"機械学習モデルの評価レポート\")"));
}
