//! WP4(インラインパース)の受け入れテスト(sml-parser-m1-handoff.md WP4)。
//!
//! - ゴールデンペア2ファイルが `strata_sml::parse` で diags ゼロになること、および
//!   代表的なインライン構文が期待どおりの AST ノードとして出現すること
//! - 各スキーム / ネスト強調 / 未閉じ各種のフォールバック / UnknownScheme /
//!   BadKeyCharset / BadCellCoord / `$x^2$` のスパン正確性の単体テスト
//! - スパン規律: 返す `Text`/`MathTex` スパンが入力 span の範囲内に収まること

use strata_sml::inline::parse_inlines;
use strata_sml::{
    BlockKind, CellCoord, Diag, DiagKind, EmphKind, RefScheme, RefTarget, SmlBlock, SmlInline, Span,
};

fn read_doc(rel: &str) -> String {
    let path = format!("{}/../../docs/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

fn parse(src: &str) -> (Vec<SmlInline>, Vec<Diag>) {
    let mut diags = Vec::new();
    let span = Span::new(0, src.len());
    let out = parse_inlines(src, span, &mut diags);
    (out, diags)
}

/// ドキュメント中の全インラインノードを平坦化して集める(見出し・段落・リスト項目の
/// 中身、および `Emph` の子を再帰的に)。フェンス本体(表/数式/図)はインラインを
/// 持たないのでここでは対象外。
fn collect_inlines(blocks: &[SmlBlock]) -> Vec<SmlInline> {
    let mut out = Vec::new();
    for b in blocks {
        match &b.kind {
            BlockKind::Heading { inline, .. } => collect_from(inline, &mut out),
            BlockKind::Paragraph { inline } => collect_from(inline, &mut out),
            BlockKind::List { items, .. } => {
                for item in items {
                    collect_from(&item.inline, &mut out);
                }
            }
            BlockKind::Fence(_) | BlockKind::CodeFence { .. } => {}
        }
    }
    out
}

fn collect_from(list: &[SmlInline], out: &mut Vec<SmlInline>) {
    for node in list {
        out.push(node.clone());
        if let SmlInline::Emph { children, .. } = node {
            collect_from(children, out);
        }
    }
}

/// 任意のインライン列(再帰的に)に対して、Text/MathTex のスパンが全て `span` の
/// 範囲内に収まっていることを検証する(スパン規律)。
fn assert_spans_within(nodes: &[SmlInline], span: Span) {
    for node in nodes {
        match node {
            SmlInline::Text(sp) | SmlInline::MathTex(sp) => {
                assert!(
                    sp.start >= span.start && sp.end <= span.end,
                    "span {sp:?} escapes bounds {span:?}"
                );
            }
            SmlInline::Emph { children, .. } => assert_spans_within(children, span),
            SmlInline::Ref { .. } | SmlInline::TermRef { .. } => {}
        }
    }
}

// ---- ゴールデンペア -----------------------------------------------------------

#[test]
fn golden_pair_parses_with_zero_diags() {
    for path in ["sml_example_draft.sml", "sml_example_formatted.sml"] {
        let src = read_doc(path);
        let out = strata_sml::parse(&src);
        assert!(out.diags.is_empty(), "{path}: expected zero diags, got {:?}", out.diags);
    }
}

#[test]
fn golden_pair_contains_expected_inline_nodes() {
    for path in ["sml_example_draft.sml", "sml_example_formatted.sml"] {
        let src = read_doc(path);
        let out = strata_sml::parse(&src);
        let nodes = collect_inlines(&out.doc.blocks);

        // term: の日本語ターゲット。
        assert!(
            nodes.iter().any(|n| matches!(
                n,
                SmlInline::TermRef { name_or_id: RefTarget::Label(l), .. } if l == "予測モデル"
            )),
            "{path}: missing term ref 予測モデル in {nodes:?}"
        );

        // table: 参照。
        assert!(
            nodes.iter().any(|n| matches!(
                n,
                SmlInline::Ref { scheme: RefScheme::Table, target: RefTarget::Label(l), .. }
                    if l == "eval-table"
            )),
            "{path}: missing table ref eval-table"
        );

        // cell: 参照の座標。
        assert!(
            nodes.iter().any(|n| matches!(
                n,
                SmlInline::Ref { scheme: RefScheme::Cell, coord: Some(c), .. }
                    if c.row_path == vec!["Opt-v2".to_string()]
                        && c.col_path == vec!["Dataset-A".to_string(), "Latency".to_string()]
            )),
            "{path}: missing cell ref with expected coord"
        );

        // math: 参照。
        assert!(
            nodes.iter().any(|n| matches!(
                n,
                SmlInline::Ref { scheme: RefScheme::Math, target: RefTarget::Label(l), .. }
                    if l == "loss-formula"
            )),
            "{path}: missing math ref loss-formula"
        );

        // **12 ms** (strong)。
        assert!(
            nodes.iter().any(|n| matches!(
                n,
                SmlInline::Emph { kind: EmphKind::Strong, children }
                    if children.iter().any(|c| matches!(c, SmlInline::Text(sp) if sp.slice(&src) == "12 ms"))
            )),
            "{path}: missing strong 12 ms"
        );

        // `Opt-v2` (code)。
        assert!(
            nodes.iter().any(|n| matches!(
                n,
                SmlInline::Emph { kind: EmphKind::Code, children }
                    if children.iter().any(|c| matches!(c, SmlInline::Text(sp) if sp.slice(&src) == "Opt-v2"))
            )),
            "{path}: missing code Opt-v2"
        );

        // スパン規律: 全ての Text/MathTex スパンがドキュメント全体の範囲内に収まる。
        assert_spans_within(&nodes, Span::new(0, src.len()));
    }
}

// ---- 各スキーム ---------------------------------------------------------------

#[test]
fn ref_scheme_label_target() {
    let src = "[x](ref:some-label)";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(out.len(), 1);
    match &out[0] {
        SmlInline::Ref { scheme: RefScheme::Ref, target: RefTarget::Label(l), coord: None, text } => {
            assert_eq!(l, "some-label");
            assert_eq!(text.slice(src), "x");
        }
        other => panic!("expected ref, got {other:?}"),
    }
}

#[test]
fn table_scheme_label_target() {
    let (out, diags) = parse("[表](table:eval-table)");
    assert!(diags.is_empty(), "{diags:?}");
    assert!(matches!(
        &out[0],
        SmlInline::Ref { scheme: RefScheme::Table, target: RefTarget::Label(l), .. } if l == "eval-table"
    ));
}

#[test]
fn fig_scheme_label_target() {
    let (out, diags) = parse("[図](fig:perf-chart)");
    assert!(diags.is_empty(), "{diags:?}");
    assert!(matches!(
        &out[0],
        SmlInline::Ref { scheme: RefScheme::Fig, target: RefTarget::Label(l), .. } if l == "perf-chart"
    ));
}

#[test]
fn math_scheme_label_target() {
    let (out, diags) = parse("[式](math:loss-formula)");
    assert!(diags.is_empty(), "{diags:?}");
    assert!(matches!(
        &out[0],
        SmlInline::Ref { scheme: RefScheme::Math, target: RefTarget::Label(l), .. } if l == "loss-formula"
    ));
}

#[test]
fn term_scheme_allows_japanese_target_without_charset_diag() {
    let (out, diags) = parse("[予測モデル](term:予測モデル)");
    assert!(diags.is_empty(), "{diags:?}");
    match &out[0] {
        SmlInline::TermRef { name_or_id: RefTarget::Label(l), text } => {
            assert_eq!(l, "予測モデル");
            assert_eq!(text.slice("[予測モデル](term:予測モデル)"), "予測モデル");
        }
        other => panic!("expected term ref, got {other:?}"),
    }
}

#[test]
fn cell_scheme_parses_coord() {
    let src = "[12 ms](cell:eval-table#Opt-v2|Dataset-A.Latency)";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    match &out[0] {
        SmlInline::Ref {
            scheme: RefScheme::Cell,
            target: RefTarget::Label(l),
            coord: Some(CellCoord { row_path, col_path }),
            ..
        } => {
            assert_eq!(l, "eval-table");
            assert_eq!(row_path, &vec!["Opt-v2".to_string()]);
            assert_eq!(col_path, &vec!["Dataset-A".to_string(), "Latency".to_string()]);
        }
        other => panic!("expected cell ref, got {other:?}"),
    }
}

#[test]
fn ulid_target_is_recognized_as_ulid_variant() {
    // 26字 Crockford Base32 の有効な ULID(block.rs のテストでも使われている値)。
    let src = "[x](ref:01ARZ3NDEKTSV4RRFFQ69G5FAV)";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert!(matches!(&out[0], SmlInline::Ref { target: RefTarget::Ulid(_), .. }));
}

// ---- ネスト強調 -----------------------------------------------------------------

#[test]
fn strong_and_em_nest_recursively() {
    let src = "**a *b* c**";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    match &out[0] {
        SmlInline::Emph { kind: EmphKind::Strong, children } => {
            assert!(children.iter().any(|c| matches!(c, SmlInline::Emph { kind: EmphKind::Em, .. })));
        }
        other => panic!("expected strong, got {other:?}"),
    }
}

#[test]
fn code_span_does_not_nest() {
    let src = "`a *b* c`";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    match &out[0] {
        SmlInline::Emph { kind: EmphKind::Code, children } => {
            assert_eq!(children.len(), 1);
            assert!(matches!(&children[0], SmlInline::Text(sp) if sp.slice(src) == "a *b* c"));
        }
        other => panic!("expected code, got {other:?}"),
    }
}

// ---- フォールバック -------------------------------------------------------------

#[test]
fn unclosed_strong_falls_back_to_text() {
    let src = "**never closed";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
}

#[test]
fn unclosed_em_falls_back_to_text() {
    let src = "*never closed";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
}

#[test]
fn unclosed_code_falls_back_to_text() {
    let src = "`never closed";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
}

#[test]
fn unclosed_math_falls_back_to_text() {
    let src = "$never closed";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
}

#[test]
fn bracket_without_following_paren_falls_back_to_text() {
    let src = "[just a bracket] not a link";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
}

#[test]
fn bracket_with_empty_parens_falls_back_to_text() {
    let src = "[x]()";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
}

#[test]
fn external_link_falls_back_without_diag() {
    let src = "本文中の [外部サイト](https://example.com/foo) はv0仕様に存在しない。";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "external links must not raise diags: {diags:?}");
    // 曖昧点: リンク自体は解決されずプレーンテキストへ丸ごとフォールバックする。
    assert_eq!(out.len(), 1);
    assert!(matches!(&out[0], SmlInline::Text(_)));
}

// ---- UnknownScheme --------------------------------------------------------------

#[test]
fn unknown_scheme_raises_diag_and_falls_back() {
    let src = "[x](foo:bar)";
    let (out, diags) = parse(src);
    assert_eq!(diags.len(), 1);
    assert_eq!(diags[0].kind, DiagKind::UnknownScheme);
    assert_eq!(out, vec![SmlInline::Text(Span::new(0, src.len()))]);
}

// ---- BadKeyCharset ----------------------------------------------------------------

#[test]
fn bad_key_charset_on_ref_target_still_builds_node() {
    let src = "[x](ref:bad label)";
    let (out, diags) = parse(src);
    assert!(diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{diags:?}");
    match &out[0] {
        SmlInline::Ref { scheme: RefScheme::Ref, target: RefTarget::Label(l), .. } => {
            assert_eq!(l, "bad label");
        }
        other => panic!("expected ref node despite bad charset, got {other:?}"),
    }
}

#[test]
fn term_target_with_spaces_does_not_raise_bad_key_charset() {
    // term: は D5 の字句制限の対象外。
    let src = "[x](term:bad label with spaces)";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    assert!(matches!(&out[0], SmlInline::TermRef { .. }));
}

// ---- BadCellCoord -----------------------------------------------------------------

#[test]
fn bad_cell_coord_on_invalid_key_charset() {
    let src = "[x](cell:eval-table#Opt v2|Dataset-A.Latency)";
    let (out, diags) = parse(src);
    assert!(diags.iter().any(|d| d.kind == DiagKind::BadCellCoord), "{diags:?}");
    assert!(matches!(&out[0], SmlInline::Ref { scheme: RefScheme::Cell, .. }));
}

#[test]
fn bad_cell_coord_on_missing_pipe() {
    let src = "[x](cell:eval-table#OnlyRowNoPipe)";
    let (out, diags) = parse(src);
    assert!(diags.iter().any(|d| d.kind == DiagKind::BadCellCoord), "{diags:?}");
    match &out[0] {
        SmlInline::Ref { scheme: RefScheme::Cell, coord: Some(c), .. } => {
            assert_eq!(c.row_path, vec!["OnlyRowNoPipe".to_string()]);
            assert!(c.col_path.is_empty());
        }
        other => panic!("expected cell ref, got {other:?}"),
    }
}

// ---- $...$ のスパン正確性 -----------------------------------------------------------

#[test]
fn math_tex_span_covers_inner_content_only() {
    let src = "before $x^2$ after";
    let (out, diags) = parse(src);
    assert!(diags.is_empty(), "{diags:?}");
    let math = out.iter().find_map(|n| match n {
        SmlInline::MathTex(sp) => Some(*sp),
        _ => None,
    });
    let sp = math.expect("expected a MathTex node");
    assert_eq!(sp.slice(src), "x^2");
    // 前後の `$` は含まれない。
    assert_ne!(src.as_bytes()[sp.start - 1], b'x');
    assert_eq!(src.as_bytes()[sp.start - 1], b'$');
    assert_eq!(src.as_bytes()[sp.end], b'$');
}

// ---- スパン規律(単体) ------------------------------------------------------------

#[test]
fn spans_stay_within_input_span_for_a_substring_of_a_larger_document() {
    let src = "prefix text **bold `code` and $m$** [t](term:用語) suffix";
    let start = src.find("prefix").unwrap();
    let end = src.len();
    let span = Span::new(start, end);
    let mut diags = Vec::new();
    let out = parse_inlines(src, span, &mut diags);
    assert_spans_within(&out, span);
}
