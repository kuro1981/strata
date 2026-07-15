//! M6(D40)受け入れテスト — CommonMark/GFM 互換(docs/md-compat-m6-handoff.md WP-C4)。
//!
//! `docs/md-compat-audit.md` の「② 静かに壊れる」9件の最小再現を入力そのままテスト化し、
//! 全件が解消されている(情報が落ちず・化けず・正しい構造に落ちる)ことを固定する。
//! 併せて、素の CommonMark サンプル文書(見出し・リスト・リンク・引用・表・コード)が
//! fmt → build を無診断で通ることを検証する。

use strata_core::{CellValue, EmphKind, Inline, NodePayload, Rel};
use strata_sml::format;

/// fmt → build を通し、警告ゼロの成功を要求する共通ヘルパ。
fn fmt_build(src: &str) -> strata_build::BuildOutput {
    let fmt = format(src).unwrap_or_else(|d| panic!("fmt failed: {d:?}\ninput: {src:?}"));
    assert!(fmt.warnings.is_empty(), "fmt warnings: {:?}", fmt.warnings);
    let out = strata_build::build(&fmt.text)
        .unwrap_or_else(|e| panic!("build failed: {e:?}\nformatted: {:?}", fmt.text));
    assert!(out.warnings.is_empty(), "build warnings: {:?}", out.warnings);
    out
}

fn paras(out: &strata_build::BuildOutput) -> Vec<&strata_core::Para> {
    out.graph
        .nodes
        .values()
        .filter_map(|n| match &n.payload {
            NodePayload::Para(p) => Some(p),
            _ => None,
        })
        .collect()
}

fn flat_text(inline: &[Inline]) -> String {
    let mut s = String::new();
    for i in inline {
        match i {
            Inline::Text { s: t } => s.push_str(t),
            Inline::Emph { children, .. } => s.push_str(&flat_text(children)),
            _ => {}
        }
    }
    s
}

// ---- 監査②1: エスケープ ------------------------------------------------------------

/// `\*not emphasis\*` — `\` の残存も `*` の強調誤発火も無く、グラフ上は unescape 済みの
/// リテラル `*not emphasis*` の Text 1個になる。
#[test]
fn audit_1_backslash_escape_neutralizes_emphasis() {
    let out = fmt_build("\\*not emphasis\\*\n");
    let paras = paras(&out);
    assert_eq!(paras.len(), 1);
    assert_eq!(paras[0].inline, vec![Inline::Text { s: "*not emphasis*".to_string() }]);
}

// ---- 監査②2: 外部リンク/画像の無診断不活性化 -----------------------------------------

/// `[Site](https://example.com)` が `Inline::Link` として構造化される(Text 化しない)。
#[test]
fn audit_2_external_link_is_structured() {
    let out = fmt_build("See [Site](https://example.com) here.\n");
    let paras = paras(&out);
    assert!(paras[0].inline.contains(&Inline::Link {
        url: "https://example.com".to_string(),
        text: "Site".to_string()
    }));
}

/// `![alt](url)` が `Inline::Image` として構造化される。
#[test]
fn audit_2_external_image_is_structured() {
    let out = fmt_build("![alt text](https://example.com/x.png)\n");
    let paras = paras(&out);
    assert!(paras[0].inline.contains(&Inline::Image {
        url: "https://example.com/x.png".to_string(),
        alt: "alt text".to_string()
    }));
}

/// `mailto:` と autolink `<https://…>` も Link になり、UnknownScheme を出さない。
#[test]
fn audit_2_mailto_and_autolink_are_structured() {
    let out = fmt_build("Mail [me](mailto:a@example.com) or visit <https://example.com>.\n");
    let links: Vec<_> = paras(&out)[0]
        .inline
        .iter()
        .filter(|i| matches!(i, Inline::Link { .. }))
        .collect();
    assert_eq!(links.len(), 2);
}

// ---- 監査②3: 画像+内部 ref の意味破壊 ------------------------------------------------

/// `![alt](ref:target)` — `!` が孤立せず、見せかけの Ref 解決・エッジも生成されない。
/// 当面は Error 診断(`ImageRefUnsupported`)で明示拒否する(裁量)。
#[test]
fn audit_3_image_with_internal_ref_is_explicitly_rejected() {
    let src = "# T {#target}\n\n![alt](ref:target)\n";
    let diags = format(src).expect_err("expected fmt to fail with ImageRefUnsupported");
    assert!(
        diags.iter().any(|d| d.kind == strata_sml::DiagKind::ImageRefUnsupported),
        "{diags:?}"
    );
}

// ---- 監査②4: 参照スタイルリンク --------------------------------------------------------

/// `[Example][ex]` + 定義行 — 使用側は Link に解決され、定義行は非可視メタ
/// (グラフに段落ノードとして出現しない)。
#[test]
fn audit_4_reference_style_link_resolves_and_definition_is_invisible() {
    let out = fmt_build("[Example][ex] link.\n\n[ex]: https://example.com/ref \"Title\"\n");
    let paras = paras(&out);
    // 定義行の Para が無いこと(段落は使用側の1つだけ)。
    assert_eq!(paras.len(), 1);
    assert!(paras[0].inline.contains(&Inline::Link {
        url: "https://example.com/ref".to_string(),
        text: "Example".to_string()
    }));
    // 定義行のテキストがどのノードにも漏れていないこと。
    for p in &paras {
        assert!(!flat_text(&p.inline).contains("https://example.com/ref"));
    }
}

/// 未解決ラベルはリテラル維持(CommonMark 準拠)。
#[test]
fn audit_4_unresolved_reference_label_stays_literal() {
    let out = fmt_build("[Example][missing] stays.\n");
    let paras = paras(&out);
    assert!(flat_text(&paras[0].inline).contains("[Example][missing]"));
}

// ---- 監査②5: 順序リストの開始値消失 ----------------------------------------------------

/// `5. fifth` — `List.start == Some(5)` として保存される。
#[test]
fn audit_5_ordered_list_start_is_preserved() {
    let out = fmt_build("5. fifth\n6. sixth\n");
    let list = out
        .graph
        .nodes
        .values()
        .find_map(|n| match &n.payload {
            NodePayload::List(l) => Some(l),
            _ => None,
        })
        .expect("expected a list node");
    assert!(list.ordered);
    assert_eq!(list.start, Some(5));
}

/// 1 始まりは既定なので `start` は None(後方互換: 既存文書のグラフ形が変わらない)。
#[test]
fn audit_5_default_start_is_omitted() {
    let out = fmt_build("1. first\n2. second\n");
    let list = out
        .graph
        .nodes
        .values()
        .find_map(|n| match &n.payload {
            NodePayload::List(l) => Some(l),
            _ => None,
        })
        .expect("expected a list node");
    assert_eq!(list.start, None);
}

// ---- 監査②6: ゆるいリストの分裂 --------------------------------------------------------

/// 空行区切りの同種リストが1つの List に統合される(項目数ぶんの独立 List にならない)。
#[test]
fn audit_6_loose_list_merges_into_one_list() {
    let out = fmt_build("- a\n\n- b\n\n- c\n");
    let lists: Vec<_> = out
        .graph
        .nodes
        .values()
        .filter(|n| matches!(n.payload, NodePayload::List(_)))
        .collect();
    assert_eq!(lists.len(), 1, "loose list should merge into a single List node");
    let items = out.graph.children_of(lists[0].id);
    assert_eq!(items.len(), 3);
}

/// ②5+②6 の複合: ゆるい順序リストでも start が保持され、全項目が1つの List に入る。
#[test]
fn audit_6_loose_ordered_list_keeps_start_and_items() {
    let out = fmt_build("3. third\n\n4. fourth\n");
    let (id, list) = out
        .graph
        .nodes
        .iter()
        .find_map(|(id, n)| match &n.payload {
            NodePayload::List(l) => Some((*id, l)),
            _ => None,
        })
        .expect("expected a list node");
    assert_eq!(list.start, Some(3));
    assert_eq!(out.graph.children_of(id).len(), 2);
}

// ---- 監査②7: `***bold italic***` 誤ネスト ---------------------------------------------

/// Strong(Em(text)) の正しい入れ子になり、`*` が本文へ漏れない。
#[test]
fn audit_7_triple_star_nests_correctly() {
    let out = fmt_build("***bold italic*** end.\n");
    let paras = paras(&out);
    let emph = paras[0]
        .inline
        .iter()
        .find_map(|i| match i {
            Inline::Emph { kind: EmphKind::Strong, children } => Some(children),
            _ => None,
        })
        .expect("expected strong");
    assert!(matches!(&emph[0], Inline::Emph { kind: EmphKind::Em, .. }));
    // 漏れた `*` が無い。
    assert!(!flat_text(&paras[0].inline).contains('*'));
}

// ---- 監査②8: 未対応ブロック内のインライン誤爆 ------------------------------------------

/// `~~~` フェンスがコードブロックとして扱われ、中身の `*` が強調として二次誤パース
/// されない(コードが変形されない)。
#[test]
fn audit_8_tilde_fence_is_a_code_block() {
    let out = fmt_build("~~~python\nx = a * 2 * b\n~~~\n");
    let code = out
        .graph
        .nodes
        .values()
        .find_map(|n| match &n.payload {
            NodePayload::Code(c) => Some(c),
            _ => None,
        })
        .expect("expected a code node");
    assert_eq!(code.lang, "python");
    assert_eq!(code.src, "x = a * 2 * b\n");
}

// ---- 監査②9: Setext 見出しの構造喪失 ---------------------------------------------------

/// `Title\n=====` が H1 Section になり、文書階層に参加する。
#[test]
fn audit_9_setext_h1_becomes_section() {
    let out = fmt_build("Title\n=====\n\nBody text.\n");
    let section = out
        .graph
        .nodes
        .values()
        .find(|n| matches!(&n.payload, NodePayload::Section(_)))
        .expect("expected a section node");
    let NodePayload::Section(s) = &section.payload else { unreachable!() };
    assert_eq!(flat_text(&s.heading), "Title");
    // 直後の段落が Section の子になる(文書階層への参加)。
    let children = out.graph.children_of(section.id);
    assert_eq!(children.len(), 1);
    assert!(matches!(out.graph.nodes[&children[0]].payload, NodePayload::Para(_)));
}

/// 段落直後の `---` は水平線ではなく Setext H2 が優先(CommonMark 準拠)。
#[test]
fn audit_9_setext_h2_wins_over_thematic_break() {
    let out = fmt_build("Subtitle\n---\n");
    assert!(out.graph.nodes.values().any(|n| matches!(&n.payload, NodePayload::Section(_))));
    assert!(!out.graph.nodes.values().any(|n| matches!(&n.payload, NodePayload::ThematicBreak(_))));
}

/// 段落に隣接しない単独行 `---` は水平線(ThematicBreak ノード)になる。
#[test]
fn thematic_break_between_blank_lines() {
    let out = fmt_build("Before.\n\n---\n\nAfter.\n");
    assert!(out.graph.nodes.values().any(|n| matches!(&n.payload, NodePayload::ThematicBreak(_))));
}

// ---- Tier 2: GFM パイプ表・タスクリスト・取消線 ----------------------------------------

/// GFM パイプ表がフラット2次元の Table ノードへブリッジされる。セル値は型付き
/// パース(数量 `12 ms` は Quantity)を通る。
#[test]
fn gfm_pipe_table_bridges_to_flat_table() {
    let out = fmt_build("| Model | Latency |\n| --- | --- |\n| Opt-v2 | 12 ms |\n");
    let table = out
        .graph
        .nodes
        .values()
        .find_map(|n| match &n.payload {
            NodePayload::Table(t) => Some(t),
            _ => None,
        })
        .expect("expected a table node");
    assert_eq!(table.cols.len(), 1);
    assert_eq!(table.cols[0].members.len(), 2);
    assert_eq!(table.cols[0].members[0].key, "Model");
    assert_eq!(table.rows[0].members[0].key, "r1");
    assert!(table
        .cells
        .iter()
        .any(|c| c.value == CellValue::Quantity { v: 12.0, unit: "ms".to_string() }));
}

/// タスクリストのチェック状態が項目 Para の `checked` に構造化される。
#[test]
fn task_list_checked_state_is_structured() {
    let out = fmt_build("- [ ] todo\n- [x] done\n");
    let mut states: Vec<(String, Option<bool>)> = paras(&out)
        .into_iter()
        .map(|p| (flat_text(&p.inline), p.checked))
        .collect();
    states.sort();
    assert_eq!(
        states,
        vec![("done".to_string(), Some(true)), ("todo".to_string(), Some(false))]
    );
}

/// 単独行 `[ ]` が属性行と誤認されず(BadKeyCharset 等を出さず)段落として通る。
#[test]
fn lone_checkbox_line_is_not_an_attr_line() {
    let out = fmt_build("[ ]\nchecklist marker above\n");
    assert!(!paras(&out).is_empty());
}

/// `~~取消線~~` が EmphKind::Strike として構造化される。
#[test]
fn strikethrough_is_structured() {
    let out = fmt_build("~~struck~~ text.\n");
    assert!(paras(&out)[0]
        .inline
        .iter()
        .any(|i| matches!(i, Inline::Emph { kind: EmphKind::Strike, .. })));
}

// ---- blockquote / 見出し閉じ装飾 / 代替マーカー ----------------------------------------

/// blockquote が Quote ノードになり、子ブロックを contains する。
#[test]
fn blockquote_becomes_quote_node_with_children() {
    let out = fmt_build("> quoted text\n> more\n");
    let quote = out
        .graph
        .nodes
        .values()
        .find(|n| matches!(&n.payload, NodePayload::Quote(_)))
        .expect("expected a quote node");
    let children = out.graph.children_of(quote.id);
    assert_eq!(children.len(), 1);
    let NodePayload::Para(p) = &out.graph.nodes[&children[0]].payload else {
        panic!("expected a para inside quote")
    };
    assert_eq!(flat_text(&p.inline), "quoted text\nmore");
}

/// 見出し閉じ装飾 `# H #####` の末尾 `#` が除去される。
#[test]
fn atx_closing_hashes_are_stripped() {
    let out = fmt_build("# Heading ###\n");
    let NodePayload::Section(s) = &out
        .graph
        .nodes
        .values()
        .find(|n| matches!(&n.payload, NodePayload::Section(_)))
        .expect("expected section")
        .payload
    else {
        unreachable!()
    };
    assert_eq!(flat_text(&s.heading), "Heading");
}

/// 代替マーカー(`*`・`+` 箇条書き、`1)` 順序)がリストとして解釈される。
#[test]
fn alternative_list_markers_are_recognized() {
    for src in ["* star\n* star2\n", "+ plus\n+ plus2\n", "1) one\n2) two\n"] {
        let out = fmt_build(src);
        assert!(
            out.graph.nodes.values().any(|n| matches!(n.payload, NodePayload::List(_))),
            "no list for {src:?}"
        );
    }
}

// ---- Tier 3: HTML への Warning ---------------------------------------------------------

/// HTML ブロックらしき行には Warning(HtmlNotSupported)を出しつつリテラル維持で成功する。
#[test]
fn html_like_line_warns_but_succeeds() {
    let src = "<div class=\"x\">\nhello\n</div>\n";
    let fmt = format(src).expect("fmt should succeed with warnings only");
    assert!(
        fmt.warnings.iter().any(|d| d.kind == strata_sml::DiagKind::HtmlNotSupported),
        "{:?}",
        fmt.warnings
    );
    let out = strata_build::build(&fmt.text).expect("build should succeed with warnings only");
    assert!(out.warnings.iter().any(|d| d.kind == strata_sml::DiagKind::HtmlNotSupported));
}

// ---- CommonMark サンプル文書(WP-C4 受け入れ) ------------------------------------------

/// 見出し・リスト・リンク・引用・表・コードを含む素の CommonMark 文書が
/// fmt → build を無診断で通り、主要構造が全てグラフに現れる。
#[test]
fn plain_commonmark_document_passes_fmt_and_build_without_diags() {
    let src = "\
Project Notes
=============

Introduction paragraph with **bold**, _em_, ~~old~~, `code`,
an [external link](https://example.com), and a [reference][docs].

## Tasks

- [x] write spec
- [ ] implement parser

1. first step
2. second step

> A quotation with *emphasis* inside.
> Second quoted line.

| Item | Cost |
| ---- | ---- |
| Apple | 3 |
| Pen | 5 ms |

```python
total = 3 * 5
```

---

Final paragraph\\* with an escape.

[docs]: https://docs.example.com \"Docs\"
";
    let out = fmt_build(src);

    let has = |pred: &dyn Fn(&NodePayload) -> bool| out.graph.nodes.values().any(|n| pred(&n.payload));
    assert!(has(&|p| matches!(p, NodePayload::Section(_))), "section missing");
    assert!(has(&|p| matches!(p, NodePayload::List(l) if !l.ordered)), "bullet list missing");
    assert!(has(&|p| matches!(p, NodePayload::List(l) if l.ordered)), "ordered list missing");
    assert!(has(&|p| matches!(p, NodePayload::Quote(_))), "quote missing");
    assert!(has(&|p| matches!(p, NodePayload::Table(_))), "table missing");
    assert!(has(&|p| matches!(p, NodePayload::Code(_))), "code missing");
    assert!(has(&|p| matches!(p, NodePayload::ThematicBreak(_))), "thematic break missing");

    // インライン語彙(リンク・参照リンク・取消線・エスケープ)。
    let all_paras = paras(&out);
    let all_inline: Vec<&Inline> = all_paras.iter().flat_map(|p| p.inline.iter()).collect();
    assert!(all_inline.iter().any(
        |i| matches!(i, Inline::Link { url, .. } if url == "https://example.com")
    ));
    assert!(all_inline.iter().any(
        |i| matches!(i, Inline::Link { url, .. } if url == "https://docs.example.com")
    ));
    assert!(all_inline
        .iter()
        .any(|i| matches!(i, Inline::Emph { kind: EmphKind::Strike, .. })));
    assert!(all_paras.iter().any(|p| flat_text(&p.inline).contains("Final paragraph*")));

    // タスク状態。
    assert!(all_paras.iter().any(|p| p.checked == Some(true)));
    assert!(all_paras.iter().any(|p| p.checked == Some(false)));

    // 意味エッジは張られない(外部リンクはナビゲーションですらない)が、contains の
    // 背骨は繋がっている。
    assert!(out.graph.edges.iter().all(|e| e.rel == Rel::Contains));
}

/// fmt の冪等性が CommonMark サンプルでも成り立つ(fmt 契約の非退行)。
#[test]
fn plain_commonmark_document_fmt_is_idempotent() {
    let src = "Title\n=====\n\n> quote\n\n| A |\n| - |\n| 1 |\n\n- [ ] task\n";
    let once = format(src).expect("first fmt");
    let twice = format(&once.text).expect("second fmt");
    assert!(twice.patches.is_empty(), "fmt not idempotent: {:?}", twice.patches);
    assert_eq!(twice.text, once.text);
}
