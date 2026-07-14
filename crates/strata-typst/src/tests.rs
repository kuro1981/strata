//! WP-R2 単体テスト(sml-render-m4-handoff.md「テスト」節)。
//!
//! - Document title フォールバック3段
//! - `Ref` の text有無 × 番号有無の4形
//! - `Term` フォールバック
//! - `Quantity` のセル描画
//! - Chart プレースホルダ
//! - `coord` 付き cell 参照の表示
//!
//! ゴールデン契約テスト(formatted fixture 全体)は `golden.rs` に分離。

use super::*;
use strata_core::{
    Cell, CellCoord, CellValue, Chart, Dim, Document, Encoding, Figure, Graph, ImageFigure, Inline, List, Mark,
    Member, Node, NodeId, NodePayload, Para, Rel, Scalar, Section, Table, Term, Value,
};
use std::collections::BTreeMap;

fn para(id: NodeId, inline: Vec<Inline>) -> Node {
    Node { id, payload: NodePayload::Para(Para { inline }) }
}

// --- Document title フォールバック3段(D21) -----------------------------------

#[test]
fn document_title_uses_explicit_title_when_present() {
    let doc_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: doc_id, payload: NodePayload::Document(Document { title: Some("明示タイトル".into()) }) });

    let out = render_to_typst(&g, doc_id, "fallback").unwrap();
    assert!(out.contains("#set document(title: \"明示タイトル\")"), "{out}");
}

#[test]
fn document_title_falls_back_to_first_top_level_heading_when_no_title() {
    let doc_id = NodeId::new();
    let h1_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: doc_id, payload: NodePayload::Document(Document { title: None }) });
    g.insert(Node {
        id: h1_id,
        payload: NodePayload::Section(Section { heading: vec![Inline::Text { s: "見出しテキスト".into() }] }),
    });
    g.link(doc_id, Rel::Contains, h1_id, Some(0));

    let out = render_to_typst(&g, doc_id, "fallback").unwrap();
    assert!(out.contains("#set document(title: \"見出しテキスト\")"), "{out}");
}

#[test]
fn document_title_falls_back_to_caller_provided_name_when_no_title_and_no_heading() {
    let doc_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: doc_id, payload: NodePayload::Document(Document { title: None }) });
    g.insert(para(para_id, vec![Inline::Text { s: "本文だけ".into() }]));
    g.link(doc_id, Rel::Contains, para_id, Some(0));

    let out = render_to_typst(&g, doc_id, "my-file").unwrap();
    assert!(out.contains("#set document(title: \"my-file\")"), "{out}");
}

// --- Ref: text有無 × 番号有無の4形(D22) ---------------------------------------

#[test]
fn ref_with_text_and_numbered_target_uses_link_not_at_sign() {
    let table_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node {
        id: table_id,
        payload: NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    });
    g.insert(para(
        para_id,
        vec![
            Inline::Text { s: "見よ".into() },
            Inline::Ref { to: table_id, rel: Rel::RefersTo, coord: None, text: "この表".into() },
        ],
    ));
    g.link(para_id, Rel::RefersTo, table_id, None);

    let out = render_to_typst(&g, para_id, "fallback").unwrap();
    assert!(out.contains(&format!("#link(<{}>)[この表]", table_id.0)), "{out}");
}

#[test]
fn ref_with_text_and_unnumbered_target_uses_link() {
    let list_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: list_id, payload: NodePayload::List(List { ordered: false }) });
    g.insert(para(
        para_id,
        vec![Inline::Ref { to: list_id, rel: Rel::RefersTo, coord: None, text: "上のリスト".into() }],
    ));

    let out = render_to_typst(&g, para_id, "fallback").unwrap();
    assert!(out.contains(&format!("#link(<{}>)[上のリスト]", list_id.0)), "{out}");
}

#[test]
fn ref_without_text_and_numbered_target_uses_at_sign() {
    let table_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node {
        id: table_id,
        payload: NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    });
    g.insert(para(para_id, vec![Inline::Ref { to: table_id, rel: Rel::RefersTo, coord: None, text: String::new() }]));

    let out = render_to_typst(&g, para_id, "fallback").unwrap();
    assert!(out.contains(&format!("@{}", table_id.0)), "{out}");
    assert!(!out.contains("#link"), "numbered targets without text must not use #link: {out}");
}

#[test]
fn ref_without_text_and_unnumbered_target_uses_link_with_short_fallback() {
    let code_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: code_id, payload: NodePayload::Code(strata_core::Code { lang: "rust".into(), src: "()".into() }) });
    g.insert(para(para_id, vec![Inline::Ref { to: code_id, rel: Rel::RefersTo, coord: None, text: String::new() }]));

    let out = render_to_typst(&g, para_id, "fallback").unwrap();
    // 番号を持たない対象への text 無し参照は @ref にできない(Typst がコンパイルエラーに
    // する)。#link + 短い代替表記へ倒す(sml-render-m4-handoff.md D-R2 5.)。
    assert!(out.contains(&format!("#link(<{}>)[§]", code_id.0)), "{out}");
}

/// Section 対象への text 無し参照は、番号を振らない代わりに見出しテキストを
/// 代替表記として使う(裁量。§記号よりも情報量が多いと判断した)。
#[test]
fn ref_without_text_to_section_uses_heading_text_as_link_label() {
    let doc_id = NodeId::new();
    let sec_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: doc_id, payload: NodePayload::Document(Document { title: Some("t".into()) }) });
    g.insert(Node {
        id: sec_id,
        payload: NodePayload::Section(Section { heading: vec![Inline::Text { s: "導入".into() }] }),
    });
    g.insert(para(para_id, vec![Inline::Ref { to: sec_id, rel: Rel::RefersTo, coord: None, text: String::new() }]));
    g.link(doc_id, Rel::Contains, sec_id, Some(0));
    g.link(sec_id, Rel::Contains, para_id, Some(0));

    let out = render_to_typst(&g, doc_id, "fallback").unwrap();
    assert!(out.contains(&format!("#link(<{}>)[導入]", sec_id.0)), "{out}");
}

// --- coord 付き cell 参照(§5.3) ------------------------------------------------

#[test]
fn ref_with_coord_appends_row_and_col_path_to_display_text() {
    let table_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node {
        id: table_id,
        payload: NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    });
    let coord = CellCoord { row_path: vec!["Opt-v2".into()], col_path: vec!["Dataset-A".into(), "Latency".into()] };
    g.insert(para(
        para_id,
        vec![Inline::Ref { to: table_id, rel: Rel::RefersTo, coord: Some(coord), text: "12 ms".into() }],
    ));

    let out = render_to_typst(&g, para_id, "fallback").unwrap();
    assert!(
        out.contains(&format!("#link(<{}>)[12 ms (Opt-v2, Dataset-A.Latency)]", table_id.0)),
        "{out}"
    );
}

// --- Term フォールバック(D22) --------------------------------------------------

#[test]
fn term_with_text_uses_display_text_plain_no_emphasis() {
    let term_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: term_id, payload: NodePayload::Term(Term { name: "予測精度".into() }) });
    g.insert(para(para_id, vec![Inline::Term { to: term_id, text: "精度".into() }]));

    let out = render_to_typst(&g, para_id, "fallback").unwrap();
    assert!(out.contains("精度"), "{out}");
    assert!(!out.contains("*精度*"), "Term display must be plain, not emphasized: {out}");
}

#[test]
fn term_without_text_falls_back_to_term_node_name() {
    let term_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: term_id, payload: NodePayload::Term(Term { name: "予測精度".into() }) });
    g.insert(para(para_id, vec![Inline::Term { to: term_id, text: String::new() }]));

    let out = render_to_typst(&g, para_id, "fallback").unwrap();
    assert!(out.contains("予測精度"), "{out}");
    assert!(!out.contains("*予測精度*"), "Term display must be plain, not emphasized: {out}");
}

// --- Quantity セル(D4/D8) ------------------------------------------------------

#[test]
fn quantity_cell_renders_as_value_space_unit() {
    let table_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table {
        rows: vec![Dim { name: "model".into(), members: vec![Member { key: "Opt-v2".into(), label: None, children: vec![] }] }],
        cols: vec![Dim { name: "metric".into(), members: vec![Member { key: "Latency".into(), label: None, children: vec![] }] }],
        cells: vec![Cell {
            row_path: vec!["Opt-v2".into()],
            col_path: vec!["Latency".into()],
            value: CellValue::Quantity { v: 12.0, unit: "ms".into() },
        }],
        caption: None,
    };
    g.insert(Node { id: table_id, payload: NodePayload::Table(table) });

    let out = render_to_typst(&g, table_id, "fallback").unwrap();
    assert!(out.contains("[12 ms]"), "{out}");
}

// --- Chart プレースホルダ(D22 4.) ----------------------------------------------

#[test]
fn chart_figure_renders_placeholder_box_with_depicts_and_data_ref() {
    let chart_id = NodeId::new();
    let table_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node {
        id: table_id,
        payload: NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    });
    let mut depicts = BTreeMap::new();
    depicts.insert("description".to_string(), "棒グラフの説明".to_string());
    g.insert(Node {
        id: chart_id,
        payload: NodePayload::Figure(Figure::Chart(Chart {
            data_ref: table_id,
            mark: Mark::Bar,
            encode: Encoding { x: "model".into(), y: "F1-Score".into(), color: None },
            caption: Some(vec![Inline::Text { s: "図のキャプション".into() }]),
            depicts,
        })),
    });

    let out = render_to_typst(&g, chart_id, "fallback").unwrap();
    assert!(out.contains("#figure("), "{out}");
    assert!(out.contains("box("), "chart must render a placeholder box: {out}");
    assert!(out.contains("棒グラフの説明"), "{out}");
    assert!(out.contains(&format!("@{}", table_id.0)), "chart must reference data_ref via @ref: {out}");
    assert!(out.contains("bar: model × F1-Score"), "{out}");
    assert!(out.contains("caption: [図のキャプション]"), "{out}");
    assert!(out.contains(&format!("<{}>", chart_id.0)), "{out}");
}

#[test]
fn image_figure_renders_placeholder_box_with_alt_and_src() {
    let img_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node {
        id: img_id,
        payload: NodePayload::Figure(Figure::Image(ImageFigure {
            src: "asset://photos/x.jpg".into(),
            alt: "雪山でスキーをする人物".into(),
            depicts: BTreeMap::new(),
            caption: None,
        })),
    });

    let out = render_to_typst(&g, img_id, "fallback").unwrap();
    assert!(out.contains("box("), "{out}");
    assert!(out.contains("雪山でスキーをする人物"), "{out}");
    assert!(out.contains("asset://photos/x.jpg"), "{out}");
}

// --- Value::Ref セル(表内の値ノード参照) ---------------------------------------

#[test]
fn value_ref_cell_links_to_value_node() {
    let table_id = NodeId::new();
    let value_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node { id: value_id, payload: NodePayload::Value(Value { scalar: Scalar::Number(42.0), unit: None }) });
    let table = Table {
        rows: vec![Dim { name: "r".into(), members: vec![Member { key: "a".into(), label: None, children: vec![] }] }],
        cols: vec![Dim { name: "c".into(), members: vec![Member { key: "b".into(), label: None, children: vec![] }] }],
        cells: vec![Cell { row_path: vec!["a".into()], col_path: vec!["b".into()], value: CellValue::Ref { to: value_id } }],
        caption: None,
    };
    g.insert(Node { id: table_id, payload: NodePayload::Table(table) });

    let out = render_to_typst(&g, table_id, "fallback").unwrap();
    assert!(out.contains(&format!("#link(<{}>)[42]", value_id.0)), "{out}");
}
