//! ゴールデンテスト(WP-B4 完了チェックリスト、前提修正1でtex2mathに`\hat`追加後):
//! 改版後 formatted fixture → グラフ。ノード数/エッジ数/root と、代表ノード・エッジの
//! 内容を固定する。
//!
//! `docs/sml_example_formatted.sml` の `::math` 本体は `\hat{y}_i` を含む。tex2math が
//! `\hat` を UnderOver(over: Op("^")) にマップするようになったため(sml-build-m3-handoff
//! 前提修正1)、フィクスチャそのままで `build` が成功する。[`golden_structural_shape`] は
//! フィクスチャを一切改変せずグラフ構造(ノード数/エッジ数/root/代表ノード・エッジ)を
//! 固定し、[`loss_formula_math_tree_has_hat_as_underover`] は `::math` の本体木を
//! `\hat` の UnderOver 形まで含めて厳密に固定する。

use std::collections::HashSet;

use strata_build::build;
use strata_core::{CellValue, Figure, Inline, Mark, MathNode, NodePayload, Rel};
use ulid::Ulid;

const FORMATTED: &str = include_str!("../../../docs/sml_example_formatted.sml");

fn zid(suffix: char) -> strata_core::NodeId {
    let s = format!("01J2T8Z{suffix}000000000000000000");
    strata_core::NodeId(Ulid::from_string(&s).expect("golden fixture ULIDs are well-formed"))
}

fn term_id(name: &str) -> strata_core::NodeId {
    // term.rs のハードコード固定テストと同じ値(sml-build-m3-handoff.md D9)。
    let s = match name {
        "予測モデル" => "0S0P70CAD95REZJ1ZFZW4RDKHT",
        "予測精度" => "6MNYNRE7W9QBQSW9JGQS9R2CT6",
        "推論速度" => "6S7KKGH307X3QA0PTPYPQ7HVD3",
        other => panic!("unexpected term name in test: {other}"),
    };
    strata_core::NodeId(Ulid::from_string(s).unwrap())
}

/// `::math` 本体(loss-formula, ZF)の木を `\hat` の UnderOver 形まで含めて固定する。
/// 式全体: `L = \frac{1}{N} \sum_{i=1}^{N} (y_i - \hat{y}_i)^2`
#[test]
fn loss_formula_math_tree_has_hat_as_underover() {
    let out = build(FORMATTED).expect("formatted fixture builds now that \\hat is supported");

    fn ident(v: &str) -> MathNode {
        MathNode::Ident { v: v.into() }
    }
    fn num(v: &str) -> MathNode {
        MathNode::Num { v: v.into() }
    }
    fn op(v: &str) -> MathNode {
        MathNode::Op { v: v.into() }
    }

    let hat_y_i = MathNode::Sub {
        base: Box::new(MathNode::UnderOver {
            base: Box::new(ident("y")),
            under: None,
            over: Some(Box::new(op("^"))),
        }),
        sub: Box::new(ident("i")),
    };

    let expected = MathNode::Row {
        items: vec![
            ident("L"),
            op("="),
            MathNode::Frac { num: Box::new(num("1")), den: Box::new(ident("N")) },
            MathNode::UnderOver {
                base: Box::new(op("∑")),
                under: Some(Box::new(MathNode::Row {
                    items: vec![ident("i"), op("="), num("1")],
                })),
                over: Some(Box::new(ident("N"))),
            },
            op("("),
            MathNode::Sub { base: Box::new(ident("y")), sub: Box::new(ident("i")) },
            op("-"),
            hat_y_i,
            MathNode::Sup { base: Box::new(op(")")), sup: Box::new(num("2")) },
        ],
    };

    match &out.graph.nodes[&zid('F')].payload {
        NodePayload::Math(m) => assert_eq!(m.tree, expected, "loss-formula tree mismatch"),
        other => panic!("expected Math, got {other:?}"),
    }
}

#[test]
fn golden_structural_shape() {
    let out = build(FORMATTED).expect("formatted fixture builds now that \\hat is supported");

    // --- ノード数/エッジ数/root -------------------------------------------------
    assert_eq!(out.graph.nodes.len(), 18 + 3, "18 SML blocks/items + 3 distinct terms");
    assert_eq!(
        out.graph.edges.len(),
        17 /* contains */ + 3 /* supports/depends-on */ + 6, /* inline refs/terms */
        "unexpected edge count: {:#?}",
        out.graph.edges
    );
    assert_eq!(out.root, Some(zid('0')));

    // --- Document ----------------------------------------------------------------
    let doc_node = &out.graph.nodes[&zid('0')];
    match &doc_node.payload {
        NodePayload::Document(d) => assert_eq!(d.title, None, "formatted fixture has no title: key"),
        other => panic!("expected Document, got {other:?}"),
    }

    // --- Section ネスト(H1 は3つの H2 を contains、レベル飛び無しの基本ケース) ---
    let h1_children: HashSet<_> = out.graph.children_of(zid('1')).into_iter().collect();
    assert_eq!(h1_children, HashSet::from([zid('2'), zid('8'), zid('B')]));
    assert_eq!(out.graph.children_of(zid('2')), vec![zid('3'), zid('4'), zid('7')]);
    assert_eq!(out.graph.children_of(zid('4')), vec![zid('5'), zid('6')]);
    assert_eq!(out.graph.children_of(zid('8')), vec![zid('9'), zid('A')]);
    assert_eq!(
        out.graph.children_of(zid('B')),
        vec![zid('C'), zid('D'), zid('E'), zid('F'), zid('G'), zid('H')]
    );
    assert_eq!(out.graph.children_of(zid('0')), vec![zid('1')], "Document contains only the top-level H1");

    // --- List ----------------------------------------------------------------
    match &out.graph.nodes[&zid('4')].payload {
        NodePayload::List(l) => assert!(!l.ordered),
        other => panic!("expected List, got {other:?}"),
    }

    // --- Table (ZA): 次元木と代表セル --------------------------------------------
    match &out.graph.nodes[&zid('A')].payload {
        NodePayload::Table(t) => {
            assert_eq!(t.rows.len(), 1);
            assert_eq!(t.rows[0].name, "model");
            let row_keys: Vec<_> = t.rows[0].members.iter().map(|m| m.key.as_str()).collect();
            assert_eq!(row_keys, vec!["Baseline-v1", "Opt-v2"]);

            assert_eq!(t.cols.len(), 1);
            assert_eq!(t.cols[0].name, "dataset");
            let ds_a = &t.cols[0].members[0];
            assert_eq!(ds_a.key, "Dataset-A");
            assert_eq!(ds_a.children[0].name, "metric");

            assert_eq!(t.cells.len(), 8);
            let opt_v2_latency = t
                .cells
                .iter()
                .find(|c| c.row_path == vec!["Opt-v2".to_string()] && c.col_path == vec!["Dataset-A".to_string(), "Latency".to_string()])
                .expect("Opt-v2 | Dataset-A.Latency cell exists");
            assert_eq!(opt_v2_latency.value, CellValue::Quantity { v: 12.0, unit: "ms".to_string() });
        }
        other => panic!("expected Table, got {other:?}"),
    }

    // --- Figure (ZH): chart, data_ref はエイリアス解決済みで table を指す ----------
    match &out.graph.nodes[&zid('H')].payload {
        NodePayload::Figure(Figure::Chart(chart)) => {
            assert_eq!(chart.data_ref, zid('A'));
            assert_eq!(chart.mark, Mark::Bar);
            assert_eq!(chart.encode.x, "model");
            assert_eq!(chart.encode.y, "Dataset-A.F1-Score");
            assert_eq!(chart.encode.color, None);
            assert!(chart.caption.is_some());
        }
        other => panic!("expected Figure::Chart, got {other:?}"),
    }

    // --- Term 集約: 3件のみ、安定 ID で参照される -------------------------------
    let term_nodes: Vec<_> = out
        .graph
        .nodes
        .values()
        .filter(|n| matches!(n.payload, NodePayload::Term(_)))
        .collect();
    assert_eq!(term_nodes.len(), 3);
    for name in ["予測モデル", "予測精度", "推論速度"] {
        let expected_id = term_id(name);
        let node = out.graph.nodes.get(&expected_id).unwrap_or_else(|| panic!("term node for {name} missing"));
        match &node.payload {
            NodePayload::Term(t) => assert_eq!(t.name, name),
            other => panic!("expected Term, got {other:?}"),
        }
    }

    // 予測精度 は list item(Z5) の inline 使用と、Z7 の `supports=term:予測精度` の
    // 両方から同じノードへ集約されること(rel が異なる2本のエッジになる)。
    let pred_acc = term_id("予測精度");
    assert!(out.graph.edges.iter().any(|e| e.from == zid('5') && e.to == pred_acc && e.rel == Rel::TermRef));
    assert!(out.graph.edges.iter().any(|e| e.from == zid('7') && e.to == pred_acc && e.rel == Rel::Supports));

    // --- 意味エッジ(属性行由来) -------------------------------------------------
    assert!(out.graph.edges.iter().any(|e| e.from == zid('D') && e.to == zid('A') && e.rel == Rel::Supports));
    assert!(out.graph.edges.iter().any(|e| e.from == zid('G') && e.to == zid('F') && e.rel == Rel::DependsOn));

    // --- インライン参照(ナビゲーション弱参照) -----------------------------------
    assert!(out.graph.edges.iter().any(|e| e.from == zid('C') && e.to == zid('A') && e.rel == Rel::RefersTo));
    assert!(out.graph.edges.iter().any(|e| e.from == zid('G') && e.to == zid('F') && e.rel == Rel::RefersTo));

    // cell: 参照の座標保持(§9-2, §5.3)。
    let cell_ref = match &out.graph.nodes[&zid('D')].payload {
        NodePayload::Para(p) => p
            .inline
            .iter()
            .find_map(|i| match i {
                Inline::Ref { to, coord: Some(c), .. } if *to == zid('A') => Some(c.clone()),
                _ => None,
            })
            .expect("Para ZD has a cell: reference with coord"),
        other => panic!("expected Para, got {other:?}"),
    };
    assert_eq!(cell_ref.row_path, vec!["Opt-v2".to_string()]);
    assert_eq!(cell_ref.col_path, vec!["Dataset-A".to_string(), "Latency".to_string()]);

    // invariants::validate は build 内部で既に通過済み(Ok が返っている時点で保証)。
}
