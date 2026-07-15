//! WP-M3 受け入れテスト(sml-spec.md §1.8 D38/D39 の品質基準)。
//!
//! 「素の Markdown だけで書かれた SML(M6 で全部通るようになった)を
//! `render --format md` すると、意味的に同等の Markdown が返る」ことの検証。
//! 形は裁量(ハンドオフに明記): CommonMark/GFM サンプル文書を
//! fmt → build → render_to_md → 再度 fmt → build し、得られた2つの canonical
//! グラフが ID を無視して構造同値であることを確認する。
//!
//! スコープの裁量(最終報告参照): サンプルは **SML 固有の語彙(`::table`・`(ref:...)`
//! 等)を含まない**「素の CommonMark/GFM」に限定する。理由: `::table` のような
//! 多次元表は GFM パイプ表へ**平坦化**して書き出す(D38)ため、次元階層(dim 名・
//! ネスト構造)は意図的に失われる(データ格子は保存される)。これは「素の Markdown
//! の round-trip」(D39)の対象外 — D39 が要求するのは「素の .md ファイルがそのまま
//! 有効な SML ドラフトであり、情報を失わず戻ること」であって、SML 固有の高階語彙
//! (`::table`/`(ref:...)`/`(term:...)` 等)はそもそも「素の Markdown」ではない。
//! GFM パイプ表(`bridge_gfm_table` 由来の「合成 row 軸」)は本サンプルに含めており、
//! これはちょうどこの round-trip 契約が成立するように flatten 側で特別扱いしてある
//! (`is_synthetic_row_axis`)。

use strata_core::{Graph, NodeId, NodePayload};

fn build_graph(src: &str) -> (Graph, Option<NodeId>) {
    let fmted = strata_sml::format(src).unwrap_or_else(|d| panic!("fmt failed: {d:?}\n--- src ---\n{src}"));
    assert!(fmted.warnings.is_empty(), "fmt warnings: {:?}\n--- src ---\n{src}", fmted.warnings);
    let built = strata_build::build(&fmted.text)
        .unwrap_or_else(|e| panic!("build failed: {e:?}\n--- fmt'd src ---\n{}", fmted.text));
    assert!(built.warnings.is_empty(), "build warnings: {:?}\n--- fmt'd src ---\n{}", built.warnings, fmted.text);
    (built.graph, built.root)
}

/// `contains` を辿った文書順の `(payload, classes, alias)` 列。`NodeId` そのものは
/// 比較しない(fmt は毎回新しい ULID を発行するため)。サンプルは `Ref`/`Term`/
/// `Value`/`Figure`/`Anchor`(NodeId を内部に埋め込む語彙)を含まないため、この
/// 比較だけで構造同値の判定に十分(ID の付け替えで変わりうる情報が無い)。
fn canonical_dump(graph: &Graph, root: NodeId) -> Vec<(NodePayload, Vec<String>, Option<String>)> {
    fn walk(graph: &Graph, id: NodeId, out: &mut Vec<(NodePayload, Vec<String>, Option<String>)>) {
        let node = &graph.nodes[&id];
        out.push((node.payload.clone(), node.classes.clone(), node.alias.clone()));
        for child in graph.children_of(id) {
            walk(graph, child, out);
        }
    }
    let mut out = Vec::new();
    walk(graph, root, &mut out);
    out
}

/// 素の CommonMark コア+GFM 拡張(D40 Tier1/Tier2)を一通り含む文書。SML 固有の
/// 語彙(`{#...}` / `[id=...]` / `::table` / `(ref:...)` 等)は一切使わない —
/// これがまさに D39 の前提(「素の .md ファイルがそのまま有効な SML ドラフト」)。
const SAMPLE: &str = r#"# Round-trip Sample

## Introduction

This report covers *emphasis*, **strong emphasis**, `inline_code(x)`, and
~~a retracted claim~~. It also escapes a literal \* asterisk that is not
emphasis.

See the [project homepage](https://example.com/project) for details, or
just visit <https://example.com/project> directly.

![A diagram](https://example.com/diagram.png)

> Blockquotes are supported too.

---

## Lists

Unordered items:

- First item
- Second item
  - Nested child item

Ordered items starting at 3:

3. Third
4. Fourth

Tasks:

- [x] Done already
- [ ] Not done yet

## Data

| Model | Score | Released |
| --- | --- | --- |
| Baseline | 0.82 | 1997-03 |
| Opt-v2 | 12 ms | 2020-10 |

## Conclusion

That is all.
"#;

#[test]
fn plain_commonmark_document_round_trips_through_render_to_md() {
    let (graph1, root1) = build_graph(SAMPLE);
    let root1 = root1.expect("fmt inserts frontmatter, so a Document root must exist");
    let rendered = super::render_to_md(&graph1, root1, "roundtrip").expect("render must succeed");

    let (graph2, root2) = build_graph(&rendered);
    let root2 = root2.expect("re-fmt/build must also produce a Document root");

    let dump1 = canonical_dump(&graph1, root1);
    let dump2 = canonical_dump(&graph2, root2);
    assert_eq!(
        dump1.len(),
        dump2.len(),
        "node count differs after round-trip.\n--- rendered ---\n{rendered}\n--- dump1 ---\n{dump1:#?}\n--- dump2 ---\n{dump2:#?}"
    );
    for (i, (a, b)) in dump1.iter().zip(dump2.iter()).enumerate() {
        assert_eq!(a, b, "node #{i} differs after round-trip.\n--- rendered ---\n{rendered}");
    }
}
