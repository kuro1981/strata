//! WP-M3 要素別単体テスト(md-render-handoff.md「テスト」節: 表平坦化・record・
//! 数式・Ref 退化・--hide)。strata-typst::tests と同じ流儀(合成 Graph を直接
//! 組み立てて `render_to_md` に渡す)。ゴールデン契約テスト(formatted fixture 全体)
//! は golden.rs に分離。

use super::*;
use strata_core::{
    Cell, CellCoord, CellValue, Chart, Dim, Document, Encoding, Figure, Graph, ImageFigure, Inline, List, Mark,
    Member, Node, NodeId, NodePayload, Para, Rel, Scalar, Section, Table, Term, Value,
};

fn para(id: NodeId, inline: Vec<Inline>) -> Node {
    Node::new(id, NodePayload::Para(Para { inline, checked: None }))
}

fn sec(id: NodeId, text: &str) -> Node {
    Node::new(id, NodePayload::Section(Section { heading: vec![Inline::Text { s: text.into() }] }))
}

// --- 見出し(D38: GFM アンカー) ---------------------------------------------------

#[test]
fn heading_renders_as_atx_with_depth_hashes() {
    let doc_id = NodeId::new();
    let h1_id = NodeId::new();
    let h2_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(sec(h1_id, "導入"));
    g.insert(sec(h2_id, "背景"));
    g.link(doc_id, Rel::Contains, h1_id, Some(0));
    g.link(h1_id, Rel::Contains, h2_id, Some(0));

    let out = render_to_md(&g, doc_id, "fallback").unwrap();
    assert!(out.contains("# 導入\n\n"), "{out}");
    assert!(out.contains("## 背景\n\n"), "{out}");
}

/// D38: {#ULID} タグ・alias は一切出さない。
#[test]
fn output_never_contains_ulid_tags() {
    let doc_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(para(p_id, vec![Inline::Text { s: "本文".into() }]));
    g.link(doc_id, Rel::Contains, p_id, Some(0));

    let out = render_to_md(&g, doc_id, "fallback").unwrap();
    assert!(!out.contains("{#"), "{out}");
    assert!(!out.contains(&p_id.0.to_string()), "{out}");
}

/// Document.title はメタ扱い。本文には出さない(GFM に別チャンネルが無いため、
/// 見出し自体と二重表示しない裁量)。
#[test]
fn document_title_is_not_duplicated_in_body() {
    let doc_id = NodeId::new();
    let h1_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: Some("メタタイトル".into()) })));
    g.insert(sec(h1_id, "本文見出し"));
    g.link(doc_id, Rel::Contains, h1_id, Some(0));

    let out = render_to_md(&g, doc_id, "fallback").unwrap();
    assert!(!out.contains("メタタイトル"), "{out}");
    assert!(out.contains("# 本文見出し"), "{out}");
}

// --- Ref: 見出しへは GFM アンカーリンク(D38) ------------------------------------

#[test]
fn ref_to_heading_renders_as_anchor_link() {
    let doc_id = NodeId::new();
    let h1_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(sec(h1_id, "評価結果（多次元表）"));
    g.insert(para(
        p_id,
        vec![
            Inline::Text { s: "詳細は".into() },
            Inline::Ref { to: h1_id, rel: Rel::RefersTo, coord: None, text: "こちら".into() },
            Inline::Text { s: "を参照。".into() },
        ],
    ));
    g.link(doc_id, Rel::Contains, h1_id, Some(0));
    g.link(doc_id, Rel::Contains, p_id, Some(1));

    let out = render_to_md(&g, doc_id, "fallback").unwrap();
    assert!(out.contains("[こちら](#評価結果多次元表)"), "{out}");
}

/// text 無しの見出し参照は見出しテキストそのものをリンクラベルにする。
#[test]
fn ref_to_heading_without_text_uses_heading_text_as_label() {
    let doc_id = NodeId::new();
    let h1_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(sec(h1_id, "導入"));
    g.insert(para(p_id, vec![Inline::Ref { to: h1_id, rel: Rel::RefersTo, coord: None, text: String::new() }]));
    g.link(doc_id, Rel::Contains, h1_id, Some(0));
    g.link(doc_id, Rel::Contains, p_id, Some(1));

    let out = render_to_md(&g, doc_id, "fallback").unwrap();
    assert!(out.contains("[導入](#導入)"), "{out}");
}

/// 重複する見出しテキストは GitHub 方式で `-1`/`-2` を付番する(見出し行自体には
/// アンカーは書かれない — GFM は見出しテキストから読者側で自動算出するため。
/// 付番の効果は「2つ目の見出しへの Ref」がどう解決されるかで確認する)。
#[test]
fn duplicate_headings_get_deduplicated_anchor_suffixes() {
    let doc_id = NodeId::new();
    let a = NodeId::new();
    let b = NodeId::new();
    let ref_to_b = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(sec(a, "概要"));
    g.insert(sec(b, "概要"));
    g.insert(para(ref_to_b, vec![Inline::Ref { to: b, rel: Rel::RefersTo, coord: None, text: String::new() }]));
    g.link(doc_id, Rel::Contains, a, Some(0));
    g.link(doc_id, Rel::Contains, b, Some(1));
    g.link(b, Rel::Contains, ref_to_b, Some(0));

    let out = render_to_md(&g, doc_id, "fallback").unwrap();
    assert_eq!(out.matches("# 概要\n").count(), 2, "{out}");
    // 2つ目の見出しの直後にアンカー衝突回避の -1 が付く。
    assert!(out.contains("[概要](#概要-1)"), "{out}");
}

// --- Ref: 表・数式・段落などはテキスト退化(D38) ----------------------------------

#[test]
fn ref_to_table_degenerates_to_text_with_caption() {
    let table_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table {
        rows: vec![],
        cols: vec![],
        cells: vec![],
        caption: Some(vec![Inline::Text { s: "性能比較".into() }]),
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));
    g.insert(para(
        p_id,
        vec![Inline::Ref { to: table_id, rel: Rel::RefersTo, coord: None, text: "評価結果の表".into() }],
    ));

    let out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(out.contains("評価結果の表（表: 性能比較）"), "{out}");
    assert!(!out.contains("](#"), "table ref must not be a link: {out}");
}

/// `cell:` 参照(coord あり)は「表」ではなく「セル」と明示する。
#[test]
fn cell_ref_with_coord_uses_cell_kind_and_appends_coord() {
    let table_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table { rows: vec![], cols: vec![], cells: vec![], caption: Some(vec![Inline::Text { s: "表A".into() }]) };
    g.insert(Node::new(table_id, NodePayload::Table(table)));
    g.insert(para(
        p_id,
        vec![Inline::Ref {
            to: table_id,
            rel: Rel::RefersTo,
            coord: Some(CellCoord { row_path: vec!["Opt-v2".into()], col_path: vec!["Dataset-A".into(), "Latency".into()] }),
            text: "レイテンシ".into(),
        }],
    ));

    let out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(out.contains("レイテンシ（セル: 表A） (Opt-v2, Dataset-A.Latency)"), "{out}");
}

#[test]
fn ref_to_math_degenerates_to_text() {
    let math_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(
        math_id,
        NodePayload::Math(strata_core::MathBlock { tree: MathNode::Ident { v: "x".into() } }),
    ));
    g.insert(para(p_id, vec![Inline::Ref { to: math_id, rel: Rel::RefersTo, coord: None, text: "損失関数".into() }]));

    let out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(out.contains("損失関数（数式:"), "{out}");
}

// --- Term(D38: プレーンテキスト、リンクなし) -------------------------------------

#[test]
fn term_with_text_renders_plain_no_link() {
    let term_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(term_id, NodePayload::Term(Term { name: "予測精度".into() })));
    g.insert(para(p_id, vec![Inline::Term { to: term_id, text: "予測精度".into() }]));

    let out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(out.contains("予測精度"), "{out}");
    assert!(!out.contains("]("), "{out}");
    assert!(!out.contains('*'), "Term display must be plain, not emphasized: {out}");
}

// --- 数式(WP-M1: MathNode → TeX 逆直列化) ----------------------------------------

#[test]
fn inline_math_uses_single_dollar_and_block_math_uses_double_dollar() {
    let p_id = NodeId::new();
    let math_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(para(
        p_id,
        vec![Inline::Text { s: "式は".into() }, Inline::Math { tree: MathNode::Ident { v: "x".into() } }],
    ));
    g.insert(Node::new(
        math_id,
        NodePayload::Math(strata_core::MathBlock {
            tree: MathNode::Frac { num: Box::new(MathNode::Num { v: "1".into() }), den: Box::new(MathNode::Num { v: "2".into() }) },
        }),
    ));

    let p_out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(p_out.contains("$x$"), "{p_out}");

    let m_out = render_to_md(&g, math_id, "fallback").unwrap();
    assert!(m_out.contains("$$\n\\frac{1}{2}\n$$"), "{m_out}");
}

// --- record(D28、2列 GFM 表) ------------------------------------------------------

#[test]
fn record_renders_as_two_column_gfm_table_with_ordered_entries() {
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

    let out = render_to_md(&g, record_id, "fallback").unwrap();
    assert!(out.contains("| キー | 値 |"), "{out}");
    assert!(out.contains("| 姓 | 山田 |"), "{out}");
    assert!(out.contains("| 生年月日 | 1997-03-15 |"), "{out}");
    let pos_sei = out.find("姓").unwrap();
    let pos_seinengappi = out.find("生年月日").unwrap();
    assert!(pos_sei < pos_seinengappi, "record entries must render in source order: {out}");
}

// --- 表平坦化(D38: 多次元表 → GFM 表) ---------------------------------------------

fn dim(name: &str, keys: &[&str]) -> Dim {
    Dim { name: name.into(), members: keys.iter().map(|k| Member { key: (*k).into(), label: None, children: vec![] }).collect() }
}

/// ネスト列(dataset > metric)は "Dataset-A / F1-Score" のようにパス連結ヘッダになり、
/// ネスト無しの行軸(model)は1列の行ヘッダとして各行に key を出す。
#[test]
fn nested_table_flattens_to_multi_index_gfm_table() {
    let table_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table {
        rows: vec![dim("model", &["Baseline-v1", "Opt-v2"])],
        cols: vec![Dim {
            name: "dataset".into(),
            members: vec![Member {
                key: "Dataset-A".into(),
                label: None,
                children: vec![dim("metric", &["F1-Score", "Latency"])],
            }],
        }],
        cells: vec![
            Cell { row_path: vec!["Baseline-v1".into()], col_path: vec!["Dataset-A".into(), "F1-Score".into()], value: CellValue::Number { v: 0.82 } },
            Cell { row_path: vec!["Opt-v2".into()], col_path: vec!["Dataset-A".into(), "Latency".into()], value: CellValue::Quantity { v: 12.0, unit: "ms".into() } },
        ],
        caption: Some(vec![Inline::Text { s: "性能比較".into() }]),
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_md(&g, table_id, "fallback").unwrap();
    assert!(out.contains("**表: 性能比較**"), "{out}");
    assert!(out.contains("| model | Dataset-A / F1-Score | Dataset-A / Latency |"), "{out}");
    assert!(out.contains("| Baseline-v1 | 0.82 |"), "{out}");
    assert!(out.contains("| Opt-v2 |"), "{out}");
    assert!(out.contains("12 ms"), "{out}");
}

/// 回帰テスト: 行軸がネストしている場合(company → project)、`table.rows`(トップ
/// レベルの `Vec<Dim>`)の要素数は1のままだが実際の深さは2であり、行ヘッダ列も
/// 2列出さなければならない(ドッグフーディングの `project-index` 表で実際に
/// ヘッダ4列・データ5列というズレとして発現したバグの再現、修正後は一致する)。
#[test]
fn nested_row_axis_under_a_single_top_level_dim_gets_one_header_column_per_depth_level() {
    let table_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table {
        rows: vec![Dim {
            name: "company".into(),
            members: vec![Member {
                key: "acme".into(),
                label: None,
                children: vec![dim("project", &["alpha", "beta"])],
            }],
        }],
        cols: vec![dim("attr", &["period"])],
        cells: vec![
            Cell { row_path: vec!["acme".into(), "alpha".into()], col_path: vec!["period".into()], value: CellValue::Text { v: "2020".into() } },
            Cell { row_path: vec!["acme".into(), "beta".into()], col_path: vec!["period".into()], value: CellValue::Text { v: "2021".into() } },
        ],
        caption: None,
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_md(&g, table_id, "fallback").unwrap();
    let header = out.lines().find(|l| l.starts_with('|')).unwrap();
    assert_eq!(header, "| company | project | period |", "{out}");
    for line in out.lines().skip(2).filter(|l| l.starts_with('|')) {
        assert_eq!(
            line.matches('|').count(),
            header.matches('|').count(),
            "data row column count must match header: {line:?} vs {header:?}\nfull table:\n{out}"
        );
    }
    assert!(out.contains("| acme | alpha | 2020 |"), "{out}");
    assert!(out.contains("| acme | beta | 2021 |"), "{out}");
}

/// 列軸が無い表(record 的な縦表)は行ヘッダ+単一の「値」列になる。
#[test]
fn table_without_col_axis_renders_single_value_column() {
    let table_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table {
        rows: vec![dim("field", &["birth", "tenure"])],
        cols: vec![],
        cells: vec![
            Cell { row_path: vec!["birth".into()], col_path: vec![], value: CellValue::Date(strata_core::DateValue { y: 1997, m: 3, d: Some(15) }) },
            Cell { row_path: vec!["tenure".into()], col_path: vec![], value: CellValue::Period { from: strata_core::DateValue { y: 2020, m: 10, d: None }, to: None } },
        ],
        caption: None,
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_md(&g, table_id, "fallback").unwrap();
    assert!(out.contains("| field | 値 |"), "{out}");
    assert!(out.contains("| birth | 1997-03-15 |"), "{out}");
    assert!(out.contains("| tenure | 2020-10 〜 現在 |"), "{out}");
}

/// D38 裁量: GFM パイプ表ブリッジ由来の「合成 row 軸」(name="row"、r1..rN、ラベル無し)
/// は行ヘッダ列を出力しない(素の GFM 表を round-trip させるための特別扱い)。
#[test]
fn synthetic_gfm_bridge_row_axis_is_not_rendered_as_a_header_column() {
    let table_id = NodeId::new();
    let mut g = Graph::default();
    let table = Table {
        rows: vec![dim("row", &["r1", "r2"])],
        cols: vec![dim("col", &["a", "b"])],
        cells: vec![
            Cell { row_path: vec!["r1".into()], col_path: vec!["a".into()], value: CellValue::Text { v: "x1".into() } },
            Cell { row_path: vec!["r1".into()], col_path: vec!["b".into()], value: CellValue::Text { v: "y1".into() } },
            Cell { row_path: vec!["r2".into()], col_path: vec!["a".into()], value: CellValue::Text { v: "x2".into() } },
            Cell { row_path: vec!["r2".into()], col_path: vec!["b".into()], value: CellValue::Text { v: "y2".into() } },
        ],
        caption: None,
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_md(&g, table_id, "fallback").unwrap();
    assert!(out.contains("| a | b |\n"), "row-axis header column must be suppressed: {out}");
    assert!(!out.contains("row"), "synthetic dim name 'row' must not leak into output: {out}");
    assert!(out.contains("| x1 | y1 |"), "{out}");
    assert!(out.contains("| x2 | y2 |"), "{out}");
}

// --- Value / cell-ref (D8/D28) ----------------------------------------------------

#[test]
fn value_ref_cell_renders_referenced_scalar_as_plain_text() {
    let table_id = NodeId::new();
    let value_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(value_id, NodePayload::Value(Value { scalar: Scalar::Number(42.0), unit: Some("kg".into()) })));
    let table = Table {
        rows: vec![dim("field", &["weight"])],
        cols: vec![],
        cells: vec![Cell { row_path: vec!["weight".into()], col_path: vec![], value: CellValue::Ref { to: value_id } }],
        caption: None,
    };
    g.insert(Node::new(table_id, NodePayload::Table(table)));

    let out = render_to_md(&g, table_id, "fallback").unwrap();
    assert!(out.contains("42kg"), "{out}");
    assert!(!out.contains("]("), "cell ref must not be a link: {out}");
}

// --- リスト(D24 ネスト、M6 タスクリスト) ------------------------------------------

#[test]
fn nested_list_renders_with_indented_gfm_markers() {
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

    let out = render_to_md(&g, list_id, "fallback").unwrap();
    assert!(out.contains("- 親項目\n"), "{out}");
    assert!(out.contains("  - 子項目\n"), "{out}");
}

#[test]
fn ordered_list_with_start_value_continues_numbering() {
    let list_id = NodeId::new();
    let a = NodeId::new();
    let b = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(list_id, NodePayload::List(List { ordered: true, start: Some(5) })));
    g.insert(para(a, vec![Inline::Text { s: "五番目".into() }]));
    g.insert(para(b, vec![Inline::Text { s: "六番目".into() }]));
    g.link(list_id, Rel::Contains, a, Some(0));
    g.link(list_id, Rel::Contains, b, Some(1));

    let out = render_to_md(&g, list_id, "fallback").unwrap();
    assert!(out.contains("5. 五番目"), "{out}");
    assert!(out.contains("6. 六番目"), "{out}");
}

#[test]
fn task_list_renders_checkbox_syntax() {
    let list_id = NodeId::new();
    let done = NodeId::new();
    let todo = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(list_id, NodePayload::List(List { ordered: false, start: None })));
    g.insert(Node::new(done, NodePayload::Para(Para { inline: vec![Inline::Text { s: "完了".into() }], checked: Some(true) })));
    g.insert(Node::new(todo, NodePayload::Para(Para { inline: vec![Inline::Text { s: "未完了".into() }], checked: Some(false) })));
    g.link(list_id, Rel::Contains, done, Some(0));
    g.link(list_id, Rel::Contains, todo, Some(1));

    let out = render_to_md(&g, list_id, "fallback").unwrap();
    assert!(out.contains("- [x] 完了"), "{out}");
    assert!(out.contains("- [ ] 未完了"), "{out}");
}

// --- M6 新語彙(D40): blockquote / 水平線 / 取消線 / 外部リンク・画像 -----------------

#[test]
fn quote_renders_with_gt_prefix() {
    let quote_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(quote_id, NodePayload::Quote(strata_core::Quote {})));
    g.insert(para(p_id, vec![Inline::Text { s: "引用された文章".into() }]));
    g.link(quote_id, Rel::Contains, p_id, Some(0));

    let out = render_to_md(&g, quote_id, "fallback").unwrap();
    assert!(out.contains("> 引用された文章"), "{out}");
}

#[test]
fn thematic_break_renders_as_hr() {
    let hr_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(hr_id, NodePayload::ThematicBreak(strata_core::ThematicBreak {})));

    let out = render_to_md(&g, hr_id, "fallback").unwrap();
    assert_eq!(out, "---\n\n");
}

#[test]
fn strikethrough_renders_as_double_tilde() {
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(para(
        p_id,
        vec![Inline::Emph { kind: EmphKind::Strike, children: vec![Inline::Text { s: "取り消し".into() }] }],
    ));

    let out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(out.contains("~~取り消し~~"), "{out}");
}

#[test]
fn external_link_and_image_render_as_native_gfm_syntax() {
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(para(
        p_id,
        vec![
            Inline::Link { url: "https://example.com".into(), text: "サイト".into() },
            Inline::Text { s: " ".into() },
            Inline::Image { url: "https://example.com/a.png".into(), alt: "説明".into() },
        ],
    ));

    let out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(out.contains("[サイト](https://example.com)"), "{out}");
    assert!(out.contains("![説明](https://example.com/a.png)"), "{out}");
}

#[test]
fn autolink_style_link_renders_with_angle_brackets() {
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(para(p_id, vec![Inline::Link { url: "https://example.com".into(), text: "https://example.com".into() }]));

    let out = render_to_md(&g, p_id, "fallback").unwrap();
    assert!(out.contains("<https://example.com>"), "{out}");
}

// --- 図(D38: chart = プレースホルダ引用 / image = ネイティブ画像記法) --------------

#[test]
fn image_figure_renders_as_native_gfm_image() {
    let fig_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(
        fig_id,
        NodePayload::Figure(Figure::Image(ImageFigure {
            src: "asset://photo.jpg".into(),
            alt: "雪山でスキーをする人物".into(),
            depicts: Default::default(),
            caption: None,
        })),
    ));

    let out = render_to_md(&g, fig_id, "fallback").unwrap();
    assert!(out.contains("![雪山でスキーをする人物](asset://photo.jpg)"), "{out}");
}

#[test]
fn chart_figure_renders_as_blockquote_placeholder() {
    let table_id = NodeId::new();
    let fig_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(table_id, NodePayload::Table(Table { rows: vec![], cols: vec![], cells: vec![], caption: None })));
    let mut depicts = std::collections::BTreeMap::new();
    depicts.insert("description".to_string(), "棒グラフの説明".to_string());
    g.insert(Node::new(
        fig_id,
        NodePayload::Figure(Figure::Chart(Chart {
            data_ref: table_id,
            mark: Mark::Bar,
            encode: Encoding { x: "model".into(), y: "F1".into(), color: None },
            caption: None,
            depicts,
        })),
    ));

    let out = render_to_md(&g, fig_id, "fallback").unwrap();
    assert!(out.contains("> 棒グラフの説明"), "{out}");
}

// --- D23: `render --format md --hide <class>` ------------------------------------

fn classed(mut node: Node, classes: Vec<&str>) -> Node {
    node.classes = classes.into_iter().map(str::to_string).collect();
    node
}

#[test]
fn hide_removes_subtree_and_its_content() {
    let doc_id = NodeId::new();
    let sec_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(classed(sec(sec_id, "面接メモ"), vec!["note"]));
    g.insert(para(p_id, vec![Inline::Text { s: "【補足】これは非表示になるはず".into() }]));
    g.link(doc_id, Rel::Contains, sec_id, Some(0));
    g.link(sec_id, Rel::Contains, p_id, Some(0));

    let out = render_to_md_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(!out.text.contains("面接メモ"), "{}", out.text);
    assert!(!out.text.contains("【補足】"), "{}", out.text);
    assert!(out.warnings.is_empty(), "{:?}", out.warnings);
}

#[test]
fn hide_with_empty_list_matches_plain_render_to_md() {
    let doc_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(para(p_id, vec![Inline::Text { s: "本文".into() }]));
    g.link(doc_id, Rel::Contains, p_id, Some(0));

    let plain = render_to_md(&g, doc_id, "fallback").unwrap();
    let via_hide = render_to_md_with_hide(&g, doc_id, "fallback", &[]).unwrap();
    assert_eq!(plain, via_hide.text);
    assert!(via_hide.warnings.is_empty());
}

#[test]
fn ref_to_hidden_node_strips_link_and_emits_warning() {
    let doc_id = NodeId::new();
    let hidden_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(classed(para(hidden_id, vec![Inline::Text { s: "隠れた実名メモ".into() }]), vec!["note"]));
    g.insert(para(
        p_id,
        vec![
            Inline::Text { s: "詳細は".into() },
            Inline::Ref { to: hidden_id, rel: Rel::RefersTo, coord: None, text: "こちら".into() },
            Inline::Text { s: "を参照。".into() },
        ],
    ));
    g.link(doc_id, Rel::Contains, hidden_id, Some(0));
    g.link(doc_id, Rel::Contains, p_id, Some(1));

    let out = render_to_md_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(!out.text.contains("隠れた実名メモ"), "{}", out.text);
    assert!(out.text.contains("こちら"), "text 表示は残る: {}", out.text);
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(out.warnings[0].contains("warning") && out.warnings[0].contains("HiddenRef"), "{:?}", out.warnings);
}

#[test]
fn ref_without_text_to_hidden_node_uses_short_fallback() {
    let doc_id = NodeId::new();
    let hidden_id = NodeId::new();
    let p_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(classed(para(hidden_id, vec![Inline::Text { s: "隠れた実名メモ".into() }]), vec!["note"]));
    g.insert(para(p_id, vec![Inline::Ref { to: hidden_id, rel: Rel::RefersTo, coord: None, text: String::new() }]));
    g.link(doc_id, Rel::Contains, hidden_id, Some(0));
    g.link(doc_id, Rel::Contains, p_id, Some(1));

    let out = render_to_md_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(out.text.contains("(非表示)"), "{}", out.text);
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
}

/// D46: 実効 class(自身+祖先の和集合)を strata-core の共有ヘルパへ一本化した後も、
/// 「コンテナに class を1回書けばサブツリー全体が --hide の対象になる」という
/// D23 の既存契約が保たれること(3階層: Section → List → 項目Para、strata-typst の
/// 同名テストと対で固定する)。
#[test]
fn hide_inherits_through_multiple_container_levels_class_on_top_only() {
    let doc_id = NodeId::new();
    let sec_id = NodeId::new();
    let list_id = NodeId::new();
    let item_id = NodeId::new();
    let mut g = Graph::default();
    g.insert(Node::new(doc_id, NodePayload::Document(Document { title: None })));
    g.insert(classed(sec(sec_id, "面接メモ"), vec!["note"]));
    g.insert(Node::new(list_id, NodePayload::List(List { ordered: false, start: None })));
    g.insert(para(item_id, vec![Inline::Text { s: "深いネストの補足項目".into() }]));
    g.link(doc_id, Rel::Contains, sec_id, Some(0));
    g.link(sec_id, Rel::Contains, list_id, Some(0));
    g.link(list_id, Rel::Contains, item_id, Some(0));

    let out = render_to_md_with_hide(&g, doc_id, "fallback", &["note".to_string()]).unwrap();
    assert!(!out.text.contains("面接メモ"), "{}", out.text);
    assert!(!out.text.contains("深いネストの補足項目"), "コンテナの class が孫リスト項目まで継承されるはず: {}", out.text);
}
