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
    Node::new(id, NodePayload::Para(Para { inline, checked: None }))
}

// --- Document title フォールバック3段(D21) -----------------------------------

#[test]
fn document_title_uses_explicit_title_when_present() {
    let doc_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("明示タイトル".into()) })));

    let out = render_to_typst(&g, doc_id, "fallback").unwrap();
    assert!(out.contains("#set document(title: \"明示タイトル\")"), "{out}");
}

#[test]
fn document_title_falls_back_to_first_top_level_heading_when_no_title() {
    let doc_id = NodeId::new();
    let h1_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(Node::new(
        h1_id,
        NodePayload::Section(Section { heading: vec![Inline::Text { s: "見出しテキスト".into() }] }),
    ));
    g.link(doc_id, Rel::Contains, h1_id, Some(0));

    let out = render_to_typst(&g, doc_id, "fallback").unwrap();
    assert!(out.contains("#set document(title: \"見出しテキスト\")"), "{out}");
}

#[test]
fn document_title_falls_back_to_caller_provided_name_when_no_title_and_no_heading() {
    let doc_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
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
    g.insert(Node::new(
        table_id,
        NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    ));
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
    g.insert(Node::new(list_id, NodePayload::List(List { ordered: false, start: None })));
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
    g.insert(Node::new(
        table_id,
        NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    ));
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
    g.insert(Node::new(code_id, NodePayload::Code(strata_core::Code { lang: "rust".into(), src: "()".into() })));
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
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("t".into()) })));
    g.insert(Node::new(
        sec_id,
        NodePayload::Section(Section { heading: vec![Inline::Text { s: "導入".into() }] }),
    ));
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
    g.insert(Node::new(
        table_id,
        NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    ));
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
    g.insert(Node::new(term_id, NodePayload::Term(Term { name: "予測精度".into() })));
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
    g.insert(Node::new(term_id, NodePayload::Term(Term { name: "予測精度".into() })));
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
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_typst(&g, table_id, "fallback").unwrap();
    assert!(out.contains("[12 ms]"), "{out}");
}

// --- Date/Period セル描画(D29、M4y) --------------------------------------------

#[test]
fn date_and_period_cells_render_as_plain_iso_like_text() {
    let table_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table {
        rows: vec![Dim { name: "field".into(), members: vec![
            Member { key: "birth".into(), label: None, children: vec![] },
            Member { key: "tenure".into(), label: None, children: vec![] },
        ] }],
        cols: vec![],
        cells: vec![
            Cell {
                row_path: vec!["birth".into()],
                col_path: vec![],
                value: CellValue::Date(strata_core::DateValue { y: 1997, m: 3, d: Some(15) }),
            },
            Cell {
                row_path: vec!["tenure".into()],
                col_path: vec![],
                value: CellValue::Period {
                    from: strata_core::DateValue { y: 2020, m: 10, d: None },
                    to: None,
                },
            },
        ],
        caption: None,
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_typst(&g, table_id, "fallback").unwrap();
    assert!(out.contains("[1997-03-15]"), "{out}");
    assert!(out.contains("[2020-10 〜 現在]"), "{out}");
}

// --- Record 描画(D28、M4y) ------------------------------------------------------

#[test]
fn record_renders_as_two_column_table_with_ordered_entries() {
    let record_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(
        record_id,
        NodePayload::Record(strata_core::Record {
            entries: vec![
                strata_core::RecordEntry { key: "姓".into(), value: CellValue::Text { v: "山田".into() } },
                strata_core::RecordEntry {
                    key: "生年月日".into(),
                    value: CellValue::Date(strata_core::DateValue { y: 1997, m: 3, d: Some(15) }),
                },
            ],
        }),
    ));

    let out = render_to_typst(&g, record_id, "fallback").unwrap();
    assert!(out.contains("#figure("), "{out}");
    assert!(out.contains("table("), "{out}");
    assert!(out.contains(&format!("<{}>", record_id.0)), "{out}");
    assert!(out.contains("[*姓*], [山田]"), "{out}");
    assert!(out.contains("[*生年月日*], [1997-03-15]"), "{out}");
    // 姓 が 生年月日 より先に出力されること(順序保存)。
    let pos_sei = out.find("姓").unwrap();
    let pos_seinengappi = out.find("生年月日").unwrap();
    assert!(pos_sei < pos_seinengappi, "record entries must render in source order: {out}");
}

// --- Chart プレースホルダ(D22 4.) ----------------------------------------------

#[test]
fn chart_figure_renders_placeholder_box_with_depicts_and_data_ref() {
    let chart_id = NodeId::new();
    let table_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(
        table_id,
        NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None }),
    ));
    let mut depicts = BTreeMap::new();
    depicts.insert("description".to_string(), "棒グラフの説明".to_string());
    g.insert(Node::new(
        chart_id,
        NodePayload::Figure(Figure::Chart(Chart {
            data_ref: table_id,
            mark: Mark::Bar,
            encode: Encoding { x: "model".into(), y: "F1-Score".into(), color: None },
            caption: Some(vec![Inline::Text { s: "図のキャプション".into() }]),
            depicts,
        })),
    ));

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
    g.insert(Node::new(
        img_id,
        NodePayload::Figure(Figure::Image(ImageFigure {
            src: "asset://photos/x.jpg".into(),
            alt: "雪山でスキーをする人物".into(),
            depicts: BTreeMap::new(),
            caption: None,
        })),
    ));

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
    g.insert(Node::new(value_id, NodePayload::Value(Value { scalar: Scalar::Number(42.0), unit: None })));
    let table = Table {
        rows: vec![Dim { name: "r".into(), members: vec![Member { key: "a".into(), label: None, children: vec![] }] }],
        cols: vec![Dim { name: "c".into(), members: vec![Member { key: "b".into(), label: None, children: vec![] }] }],
        cells: vec![Cell { row_path: vec!["a".into()], col_path: vec!["b".into()], value: CellValue::Ref { to: value_id } }],
        caption: None,
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_typst(&g, table_id, "fallback").unwrap();
    assert!(out.contains(&format!("#link(<{}>)[42]", value_id.0)), "{out}");
}

// --- D24: ネストリストの描画 ------------------------------------------------------

/// 子 List を持つ項目は、Typst のネストリスト記法(2スペース/レベルのインデント)で
/// 直後に展開される。項目ラベルは全項目に付き、ネストした List ノード自体には
/// 付かない(裁量: 自動生成 ID で参照不能のため)。
#[test]
fn nested_list_renders_with_indented_items() {
    let list_id = NodeId::new();
    let top_id = NodeId::new();
    let sub_list_id = NodeId::new();
    let sub_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(list_id, NodePayload::List(List { ordered: false, start: None })));
    g.insert(para(top_id, vec![Inline::Text { s: "親項目".into() }]));
    g.insert(Node::new(sub_list_id, NodePayload::List(List { ordered: false, start: None })));
    g.insert(para(sub_id, vec![Inline::Text { s: "子項目".into() }]));
    g.link(list_id, Rel::Contains, top_id, Some(0));
    g.link(top_id, Rel::Contains, sub_list_id, Some(0));
    g.link(sub_list_id, Rel::Contains, sub_id, Some(0));

    let out = render_to_typst(&g, list_id, "fallback").unwrap();
    assert!(out.contains(&format!("- 親項目 <{}>\n", top_id.0)), "{out}");
    assert!(out.contains(&format!("  - 子項目 <{}>\n", sub_id.0)), "{out}");
    // ネストした List ノード自体にはラベルを付けない(裁量)。
    assert!(!out.contains(&format!("<{}>", sub_list_id.0)), "{out}");
    // リスト全体は従来どおり #block[...] <label> に包まれる。
    assert!(out.contains(&format!("] <{}>", list_id.0)), "{out}");
}

/// 番号付き子リスト(ordered)は `+` マーカーで描画される(`-` と混在可)。
#[test]
fn ordered_nested_list_uses_plus_marker() {
    let list_id = NodeId::new();
    let top_id = NodeId::new();
    let sub_list_id = NodeId::new();
    let sub_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(list_id, NodePayload::List(List { ordered: false, start: None })));
    g.insert(para(top_id, vec![Inline::Text { s: "親".into() }]));
    g.insert(Node::new(sub_list_id, NodePayload::List(List { ordered: true, start: None })));
    g.insert(para(sub_id, vec![Inline::Text { s: "番号付き子".into() }]));
    g.link(list_id, Rel::Contains, top_id, Some(0));
    g.link(top_id, Rel::Contains, sub_list_id, Some(0));
    g.link(sub_list_id, Rel::Contains, sub_id, Some(0));

    let out = render_to_typst(&g, list_id, "fallback").unwrap();
    assert!(out.contains("  + 番号付き子"), "{out}");
}

// --- D23: `render --hide <class>` -----------------------------------------------

fn classed(mut node: Node, classes: Vec<&str>) -> Node {
    node.classes = classes.into_iter().map(str::to_string).collect();
    node
}

/// class を1つでも持つブロックは contains サブツリーごと非描画になり、warnings は
/// 空(非表示ノードへの Ref が無ければ警告は出ない)。
#[test]
fn hide_removes_subtree_and_its_content() {
    let doc_id = NodeId::new();
    let sec_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("t".into()) })));
    g.insert(classed(
        Node::new(sec_id, NodePayload::Section(Section { heading: vec![Inline::Text { s: "面接メモ".into() }] })),
        vec!["note"],
    ));
    g.insert(para(para_id, vec![Inline::Text { s: "【補足】これは非表示になるはず".into() }]));
    g.link(doc_id, Rel::Contains, sec_id, Some(0));
    g.link(sec_id, Rel::Contains, para_id, Some(0));

    let out = render_to_typst_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(!out.text.contains("面接メモ"), "{}", out.text);
    assert!(!out.text.contains("【補足】"), "{}", out.text);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
}

/// `hide` が空なら `render_to_typst`(既存 API)と完全に同じ本文を返す。
#[test]
fn hide_with_empty_list_matches_plain_render_to_typst() {
    let doc_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("t".into()) })));
    g.insert(para(para_id, vec![Inline::Text { s: "本文".into() }]));
    g.link(doc_id, Rel::Contains, para_id, Some(0));

    let plain = render_to_typst(&g, doc_id, "fallback").unwrap();
    let via_hide = render_to_typst_with_hide(&g, doc_id, "fallback", &[]).unwrap();
    assert_eq!(plain, via_hide.text);
    assert!(via_hide.warnings.is_empty());
}

/// 非表示ノードへの `Ref` は Warning を出しつつリンクを剥がしてプレーンテキスト化
/// する(表示 text があればそれを使う)。
#[test]
fn ref_to_hidden_node_strips_link_and_emits_warning() {
    let doc_id = NodeId::new();
    let hidden_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("t".into()) })));
    g.insert(classed(para(hidden_id, vec![Inline::Text { s: "隠れた実名メモ".into() }]), vec!["note"]));
    g.insert(para(
        para_id,
        vec![
            Inline::Text { s: "詳細は".into() },
            Inline::Ref { to: hidden_id, rel: Rel::RefersTo, coord: None, text: "こちら".into() },
            Inline::Text { s: "を参照。".into() },
        ],
    ));
    g.link(doc_id, Rel::Contains, hidden_id, Some(0));
    g.link(doc_id, Rel::Contains, para_id, Some(1));

    let out = render_to_typst_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(!out.text.contains("隠れた実名メモ"), "{}", out.text);
    assert!(!out.text.contains(&format!("#link(<{}>)", hidden_id.0)), "{}", out.text);
    assert!(out.text.contains("こちら"), "text 表示は残る: {}", out.text);
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(out.warnings[0].contains("warning"), "{:?}", out.warnings);
}

/// text 無しの `Ref` が非表示ノードを指す場合、短い代替表記(「(非表示)」)へ倒す。
#[test]
fn ref_without_text_to_hidden_node_uses_short_fallback() {
    let doc_id = NodeId::new();
    let hidden_id = NodeId::new();
    let para_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("t".into()) })));
    g.insert(classed(para(hidden_id, vec![Inline::Text { s: "隠れた実名メモ".into() }]), vec!["note"]));
    g.insert(para(
        para_id,
        vec![Inline::Ref { to: hidden_id, rel: Rel::RefersTo, coord: None, text: String::new() }],
    ));
    g.link(doc_id, Rel::Contains, hidden_id, Some(0));
    g.link(doc_id, Rel::Contains, para_id, Some(1));

    let out = render_to_typst_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(out.text.contains("(非表示)"), "{}", out.text);
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
}

/// D46: 実効 class(自身+祖先の和集合)を strata-core の共有ヘルパへ一本化した後も、
/// 「コンテナに class を1回書けばサブツリー全体が --hide の対象になる」という
/// D23 の既存契約が保たれること(3階層: Section → List → 項目Para)を固定する。
/// class はコンテナ(Section)にだけ付け、孫(リスト項目)は自身の class を持たない —
/// それでも実効 class 経由で非表示サブツリーに含まれるはず。
#[test]
fn hide_inherits_through_multiple_container_levels_class_on_top_only() {
    let doc_id = NodeId::new();
    let sec_id = NodeId::new();
    let list_id = NodeId::new();
    let item_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("t".into()) })));
    g.insert(classed(
        Node::new(sec_id, NodePayload::Section(Section { heading: vec![Inline::Text { s: "面接メモ".into() }] })),
        vec!["note"],
    ));
    g.insert(Node::new(list_id, NodePayload::List(List { ordered: false, start: None })));
    g.insert(para(item_id, vec![Inline::Text { s: "深いネストの補足項目".into() }]));
    g.link(doc_id, Rel::Contains, sec_id, Some(0));
    g.link(sec_id, Rel::Contains, list_id, Some(0));
    g.link(list_id, Rel::Contains, item_id, Some(0));

    let out = render_to_typst_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(!out.text.contains("面接メモ"), "{}", out.text);
    assert!(!out.text.contains("深いネストの補足項目"), "コンテナの class が孫リスト項目まで継承されるはず: {}", out.text);
}
