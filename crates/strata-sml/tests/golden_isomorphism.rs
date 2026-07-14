//! ゴールデン同型テスト(WP5, sml-parser-design.md §7-1・§8)。
//!
//! 検証すること:
//! 1. `docs/sml_example_draft.sml` と `sml_example_formatted.sml` が両方とも
//!    diags **ゼロ**でパースできること
//! 2. 両ファイルの AST が「ID情報を無視すれば同型」であること(fmt 契約
//!    「意味保存」— sml-spec §8.1 — をパーサ側から挟む検証)
//! 3. 非対応 Markdown(blockquote / GFM表 / setext見出し)がエラーにならず
//!    段落として解釈されること(design.md §5・§7-4 のフォールバック方針)
//!
//! 同型比較は「ID情報/Span を消した正規化構造」への変換(`tests/common/mod.rs` の
//! `norm_doc` 以下、WP-F2 で共通化)を経由して行う。無視するのは: 各ブロックの
//! id_tag 全体(alias 含む)、属性行中の `id`・`alias` エントリ、すべての Span の数値
//! (sml-spec §8.1、2026-07-13 裁定)。それ以外(ブロック種別・見出しレベル・
//! リスト項目数・属性行のその他エントリ・インラインの構造とテキスト内容・参照の
//! scheme/target/coord・表の次元木とセル)は厳密に比較する。

mod common;

use common::{norm_doc, read_doc};
use strata_sml::BlockKind;

// ---- 1. ゴールデンペアが diags ゼロでパースできること ---------------------------

#[test]
fn golden_draft_parses_with_zero_diags() {
    let src = read_doc("sml_example_draft.sml");
    let out = strata_sml::parse(&src);
    assert!(out.diags.is_empty(), "draft: expected zero diags, got {:?}", out.diags);
}

#[test]
fn golden_formatted_parses_with_zero_diags() {
    let src = read_doc("sml_example_formatted.sml");
    let out = strata_sml::parse(&src);
    assert!(out.diags.is_empty(), "formatted: expected zero diags, got {:?}", out.diags);
}

// ---- 2. draft と formatted の AST が ID 無視で同型であること -------------------

#[test]
fn draft_and_formatted_are_isomorphic_ignoring_id_information() {
    let draft_src = read_doc("sml_example_draft.sml");
    let formatted_src = read_doc("sml_example_formatted.sml");

    let draft_out = strata_sml::parse(&draft_src);
    let formatted_out = strata_sml::parse(&formatted_src);

    assert!(draft_out.diags.is_empty(), "draft: expected zero diags, got {:?}", draft_out.diags);
    assert!(formatted_out.diags.is_empty(), "formatted: expected zero diags, got {:?}", formatted_out.diags);

    let draft_norm = norm_doc(&draft_src, &draft_out.doc);
    let formatted_norm = norm_doc(&formatted_src, &formatted_out.doc);

    assert_eq!(
        draft_norm.len(),
        formatted_norm.len(),
        "block count differs: draft has {}, formatted has {}",
        draft_norm.len(),
        formatted_norm.len()
    );

    for (i, (d, f)) in draft_norm.iter().zip(formatted_norm.iter()).enumerate() {
        assert_eq!(d, f, "block #{i} differs between draft and formatted (ID-agnostic comparison)");
    }
}

// ---- 3. 非対応 Markdown のフォールバック ---------------------------------------
//
// design.md §5: blockquote / GFM表 / setext見出しは v0 が解釈しないサブセット外の
// 構文であり、エラーにはせず段落(プレーンテキスト)として読む。

#[test]
fn blockquote_falls_back_to_paragraph_without_diags() {
    let src = "> quoted text\n";
    let out = strata_sml::parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 1);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Paragraph { .. }));
}

#[test]
fn gfm_table_falls_back_to_paragraph_without_diags() {
    let src = "| a | b |\n| - | - |\n| 1 | 2 |\n";
    let out = strata_sml::parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 1);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Paragraph { .. }));
}

#[test]
fn setext_heading_falls_back_to_paragraph_without_diags() {
    // 下線式見出し。v0 は ATX(`#`)のみ対応(design.md §5)。
    let src = "Title\n---\n";
    let out = strata_sml::parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 1);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Paragraph { .. }));
}
