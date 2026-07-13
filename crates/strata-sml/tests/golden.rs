//! ゴールデンペア + スパン被覆不変条件のテスト(sml-parser-design.md §7)。
//!
//! ここでは「パニックせず処理でき、スパン被覆不変条件を満たすこと」までを検証する。
//! ゴールデンペアの「diags ゼロ」および「draft/formatted の AST が ID 無視で同型」の
//! 検証は `tests/golden_isomorphism.rs`(WP5)に集約されている。

use strata_sml::{parse, ParseOutput};

fn read_doc(rel: &str) -> String {
    let path = format!("{}/../../docs/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// 任意の入力に対し、パース結果のブロック列(隙間を含む)が全バイトを昇順・非重複・
/// 全被覆でカバーし、隙間が空行のみであることを検証する。
fn assert_span_coverage(src: &str, out: &ParseOutput) {
    assert_eq!(out.doc.src_len, src.len());
    let mut cursor = 0usize;
    for block in &out.doc.blocks {
        assert!(
            block.span.start >= cursor,
            "block span {:?} starts before cursor {cursor}",
            block.span
        );
        if block.span.start > cursor {
            let gap = &src[cursor..block.span.start];
            for line in gap.split_inclusive('\n') {
                assert!(line.trim().is_empty(), "gap between blocks is not blank: {line:?}");
            }
        }
        assert!(block.span.end >= block.span.start);
        assert!(block.span.end <= src.len());
        cursor = block.span.end;
    }
    if cursor < src.len() {
        let gap = &src[cursor..];
        for line in gap.split_inclusive('\n') {
            assert!(line.trim().is_empty(), "trailing gap is not blank: {line:?}");
        }
    }
}

#[test]
fn golden_draft_parses_without_panicking_and_covers_all_bytes() {
    let src = read_doc("sml_example_draft.sml");
    let out = parse(&src);
    assert_span_coverage(&src, &out);
    assert!(!out.doc.blocks.is_empty());
}

#[test]
fn golden_formatted_parses_without_panicking_and_covers_all_bytes() {
    let src = read_doc("sml_example_formatted.sml");
    let out = parse(&src);
    assert_span_coverage(&src, &out);
    assert!(!out.doc.blocks.is_empty());
}

#[test]
fn empty_file_satisfies_coverage() {
    let out = parse("");
    assert_span_coverage("", &out);
    assert!(out.doc.blocks.is_empty());
}

#[test]
fn blank_lines_only_satisfies_coverage() {
    let src = "\n\n\n\n";
    let out = parse(src);
    assert_span_coverage(src, &out);
    assert!(out.doc.blocks.is_empty());
}

#[test]
fn unclosed_fence_still_satisfies_coverage() {
    let src = "# Title\n\n::table {#t}\n@rows:\n  - a: [b]\n";
    let out = parse(src);
    assert_span_coverage(src, &out);
    assert!(out.diags.iter().any(|d| d.kind == strata_sml::DiagKind::UnclosedFence));
}

#[test]
fn unclosed_code_fence_still_satisfies_coverage() {
    let src = "before\n\n```rust\nfn main() {}\n";
    let out = parse(src);
    assert_span_coverage(src, &out);
    assert!(out.diags.iter().any(|d| d.kind == strata_sml::DiagKind::UnclosedFence));
}
