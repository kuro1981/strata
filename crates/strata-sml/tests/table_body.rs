//! `::table` 本体パース(`table.rs`, WP3)のテスト(sml-parser-m1-handoff.md 受け入れ条件)。
//!
//! - ゴールデンペア2ファイルを文書ごとパースし、diags がゼロであること・
//!   表の次元木とセルが期待値と完全一致することを検証する(span は無視して比較する:
//!   spans は fmt(M2)向けの情報であり、この受け入れ条件が求めるのは構造の一致)
//! - フラット糖衣 / ネスト次元 / member ラベル / セル値6型 / 各 DiagKind /
//!   コメントと空行の混在の単体テスト

use strata_sml::{BlockKind, CellRaw, DiagKind, DimNode, FenceBody, FenceKind, MemberNode, RefTarget, TableBody};

fn read_doc(rel: &str) -> String {
    let path = format!("{}/../../docs/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// 文書中の最初の `::table` フェンスの `TableBody` を取り出す。
fn first_table_body(src: &str) -> (TableBody, Vec<strata_sml::Diag>) {
    let out = strata_sml::parse(src);
    for block in &out.doc.blocks {
        if let BlockKind::Fence(fb) = &block.kind
            && fb.fence_kind == FenceKind::Table
            && let FenceBody::Table(tb) = &fb.body
        {
            return (tb.clone(), out.diags);
        }
    }
    panic!("no ::table fence found in document");
}

// ---- スパンを無視した構造比較ヘルパ ------------------------------------------

fn eq_member(a: &MemberNode, b: &MemberNode) -> bool {
    a.key == b.key
        && a.label == b.label
        && a.children.len() == b.children.len()
        && a.children.iter().zip(&b.children).all(|(x, y)| eq_dim(x, y))
}

fn eq_dim(a: &DimNode, b: &DimNode) -> bool {
    a.name == b.name
        && a.members.len() == b.members.len()
        && a.members.iter().zip(&b.members).all(|(x, y)| eq_member(x, y))
}

fn eq_dims(a: &[DimNode], b: &[DimNode]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| eq_dim(x, y))
}

fn member(key: &str, children: Vec<DimNode>) -> MemberNode {
    MemberNode { key: key.to_string(), label: None, span: strata_sml::Span::new(0, 0), children }
}

fn leaf_dim(name: &str, keys: &[&str]) -> DimNode {
    DimNode {
        name: name.to_string(),
        span: strata_sml::Span::new(0, 0),
        members: keys.iter().map(|k| member(k, Vec::new())).collect(),
    }
}

fn dim(name: &str, members: Vec<MemberNode>) -> DimNode {
    DimNode { name: name.to_string(), span: strata_sml::Span::new(0, 0), members }
}

// ---- ゴールデンペア ------------------------------------------------------------

fn expected_rows() -> Vec<DimNode> {
    vec![leaf_dim("model", &["Baseline-v1", "Opt-v2"])]
}

fn expected_cols() -> Vec<DimNode> {
    vec![dim(
        "dataset",
        vec![
            member("Dataset-A", vec![leaf_dim("metric", &["F1-Score", "Latency"])]),
            member("Dataset-B", vec![leaf_dim("metric", &["F1-Score", "Latency"])]),
        ],
    )]
}

fn assert_golden_table(rel: &str) {
    let src = read_doc(rel);
    let (table, diags) = first_table_body(&src);
    assert!(diags.is_empty(), "{rel}: expected zero diags, got {diags:?}");

    assert!(eq_dims(&table.rows, &expected_rows()), "{rel}: rows mismatch: {:#?}", table.rows);
    assert!(eq_dims(&table.cols, &expected_cols()), "{rel}: cols mismatch: {:#?}", table.cols);

    assert_eq!(table.cells.len(), 8, "{rel}: expected 8 cells, got {:#?}", table.cells);

    let find = |row: &str, col: &[&str]| {
        table
            .cells
            .iter()
            .find(|c| c.row_path == vec![row.to_string()] && c.col_path == col.iter().map(|s| s.to_string()).collect::<Vec<_>>())
            .unwrap_or_else(|| panic!("{rel}: missing cell {row} | {}", col.join(".")))
    };

    for row in ["Baseline-v1", "Opt-v2"] {
        for ds in ["Dataset-A", "Dataset-B"] {
            let f1 = find(row, &[ds, "F1-Score"]);
            assert!(matches!(f1.value, CellRaw::Number(_)), "{rel}: {row}|{ds}.F1-Score should be Number, got {:?}", f1.value);

            let latency = find(row, &[ds, "Latency"]);
            match &latency.value {
                CellRaw::Quantity { unit, .. } => assert_eq!(unit, "ms"),
                other => panic!("{rel}: {row}|{ds}.Latency should be Quantity, got {other:?}"),
            }
        }
    }

    // 具体的な値も golden の記載通りであることを固定する。
    assert_eq!(find("Baseline-v1", &["Dataset-A", "F1-Score"]).value, CellRaw::Number(0.82));
    assert_eq!(find("Baseline-v1", &["Dataset-A", "Latency"]).value, CellRaw::Quantity { v: 45.0, unit: "ms".to_string() });
    assert_eq!(find("Baseline-v1", &["Dataset-B", "F1-Score"]).value, CellRaw::Number(0.78));
    assert_eq!(find("Baseline-v1", &["Dataset-B", "Latency"]).value, CellRaw::Quantity { v: 50.0, unit: "ms".to_string() });
    assert_eq!(find("Opt-v2", &["Dataset-A", "F1-Score"]).value, CellRaw::Number(0.91));
    assert_eq!(find("Opt-v2", &["Dataset-A", "Latency"]).value, CellRaw::Quantity { v: 12.0, unit: "ms".to_string() });
    assert_eq!(find("Opt-v2", &["Dataset-B", "F1-Score"]).value, CellRaw::Number(0.88));
    assert_eq!(find("Opt-v2", &["Dataset-B", "Latency"]).value, CellRaw::Quantity { v: 15.0, unit: "ms".to_string() });
}

#[test]
fn golden_draft_table_matches_expected_structure() {
    assert_golden_table("sml_example_draft.sml");
}

#[test]
fn golden_formatted_table_matches_expected_structure() {
    assert_golden_table("sml_example_formatted.sml");
}

// ---- 単体テスト ---------------------------------------------------------------

fn parse_body(src: &str) -> (TableBody, Vec<strata_sml::Diag>) {
    let mut diags = Vec::new();
    let table = strata_sml::table::parse_table_body(src, strata_sml::Span::new(0, src.len()), &mut diags);
    (table, diags)
}

#[test]
fn flat_sugar() {
    let (t, diags) = parse_body("@rows:\n  - model: [Baseline-v1, Opt-v2]\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.rows.len(), 1);
    assert_eq!(t.rows[0].name, "model");
    assert_eq!(t.rows[0].members.len(), 2);
    assert_eq!(t.rows[0].members[0].key, "Baseline-v1");
    assert_eq!(t.rows[0].members[1].key, "Opt-v2");
}

#[test]
fn nested_dims() {
    let src = "@cols:\n  - dataset:\n    - Dataset-A:\n      - metric: [F1-Score, Latency]\n    - Dataset-B:\n      - metric: [F1-Score, Latency]\n";
    let (t, diags) = parse_body(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert!(eq_dims(&t.cols, &expected_cols()));
}

#[test]
fn member_label_plain() {
    let (t, diags) = parse_body("@rows:\n  - quarter:\n    - q1 \"第1四半期\"\n    - q2 \"第2四半期\"\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.rows[0].members[0].label.as_deref(), Some("第1四半期"));
    assert_eq!(t.rows[0].members[1].label.as_deref(), Some("第2四半期"));
}

#[test]
fn member_label_with_children() {
    let src = "@cols:\n  - dataset:\n    - a \"データセットA\":\n      - metric: [x, y]\n";
    let (t, diags) = parse_body(src);
    assert!(diags.is_empty(), "{diags:?}");
    let m = &t.cols[0].members[0];
    assert_eq!(m.key, "a");
    assert_eq!(m.label.as_deref(), Some("データセットA"));
    assert_eq!(m.children.len(), 1);
    assert_eq!(m.children[0].name, "metric");
}

#[test]
fn cell_value_number() {
    let (t, diags) = parse_body("@cells:\n  a | b : 0.82\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.cells[0].value, CellRaw::Number(0.82));
}

#[test]
fn cell_value_negative_and_exponent() {
    let (t, diags) = parse_body("@cells:\n  a | b : -3\n  a | c : 1e5\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.cells[0].value, CellRaw::Number(-3.0));
    assert_eq!(t.cells[1].value, CellRaw::Number(1e5));
}

#[test]
fn cell_value_quantity() {
    let (t, diags) = parse_body("@cells:\n  a | b : 45 ms\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.cells[0].value, CellRaw::Quantity { v: 45.0, unit: "ms".to_string() });
}

#[test]
fn cell_value_quoted_text() {
    let (t, diags) = parse_body("@cells:\n  a | b : \"任意の テキスト\"\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.cells[0].value, CellRaw::Text("任意の テキスト".to_string()));
}

#[test]
fn cell_value_bare_text_fallback() {
    let (t, diags) = parse_body("@cells:\n  a | b : n/a\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.cells[0].value, CellRaw::Text("n/a".to_string()));
}

#[test]
fn cell_value_empty() {
    let (t, diags) = parse_body("@cells:\n  a | b : ~\n  a | c :\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.cells[0].value, CellRaw::Empty);
    assert_eq!(t.cells[1].value, CellRaw::Empty);
}

#[test]
fn cell_value_ref() {
    let (t, diags) = parse_body("@cells:\n  a | b : ref:some-node\n");
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.cells[0].value, CellRaw::Ref(RefTarget::Label("some-node".to_string())));
}

#[test]
fn bad_cell_coord_diag() {
    let (_, diags) = parse_body("@cells:\n  bad row | b : 1\n");
    assert!(diags.iter().any(|d| d.kind == DiagKind::BadCellCoord), "{diags:?}");
}

#[test]
fn inconsistent_indent_diag() {
    let (_, diags) = parse_body("@rows:\n   - model: [a]\n");
    assert!(diags.iter().any(|d| d.kind == DiagKind::InconsistentIndent), "{diags:?}");
}

#[test]
fn bad_key_charset_diag() {
    let (_, diags) = parse_body("@rows:\n  - mo del: [a]\n");
    assert!(diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{diags:?}");
}

#[test]
fn comments_and_blank_lines_mixed() {
    let src = "# top comment\n\n@rows:\n  # dim comment\n  - model: [a, b]\n\n# between sections\n@cells:\n  # cell comment\n  a | model.a : 1\n";
    let (t, diags) = parse_body(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(t.rows[0].members.len(), 2);
    assert_eq!(t.cells.len(), 1);
}
