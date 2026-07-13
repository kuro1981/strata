//! 属性行のテスト(単一値/リスト値/引用符値/孤立/DuplicateId/BadKeyCharset)
//! (sml-parser-m1-handoff.md WP2 受け入れ条件)。

use strata_sml::{parse, AttrValue, DiagKind};

#[test]
fn single_value() {
    let out = parse("[supports=eval-table]\nParagraph text.\n");
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    let attrs = out.doc.blocks[0].attrs.as_ref().unwrap();
    assert_eq!(attrs.entries[0].0, "supports");
    assert_eq!(attrs.entries[0].1, AttrValue::Single("eval-table".to_string()));
}

#[test]
fn list_value() {
    let out = parse("[supports=[claim-1, claim-2]]\nParagraph text.\n");
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    let attrs = out.doc.blocks[0].attrs.as_ref().unwrap();
    assert_eq!(
        attrs.entries[0].1,
        AttrValue::List(vec!["claim-1".to_string(), "claim-2".to_string()])
    );
}

#[test]
fn quoted_value() {
    let out = parse("[caption=\"モデル別・データセット別の性能比較\"]\nParagraph text.\n");
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    let attrs = out.doc.blocks[0].attrs.as_ref().unwrap();
    assert_eq!(
        attrs.entries[0].1,
        AttrValue::Quoted("モデル別・データセット別の性能比較".to_string())
    );
}

#[test]
fn orphan_attr_line_before_blank() {
    let out = parse("[id=foo]\n\nParagraph.\n");
    assert_eq!(out.diags.len(), 1);
    assert_eq!(out.diags[0].kind, DiagKind::OrphanAttrLine);
}

#[test]
fn orphan_attr_line_at_eof() {
    let out = parse("[id=foo]\n");
    assert_eq!(out.diags.len(), 1);
    assert_eq!(out.diags[0].kind, DiagKind::OrphanAttrLine);
}

#[test]
fn duplicate_id_heading_and_attr_line() {
    let out = parse("[id=foo]\n# Title {#bar}\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

#[test]
fn duplicate_id_fence_and_attr_line() {
    let out = parse("[id=foo]\n::math {#bar}\nx = 1\n::\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

#[test]
fn no_duplicate_id_for_plain_paragraph() {
    let out = parse("[id=foo]\nA normal paragraph.\n");
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

// ---- IdNotAllowedHere(sml-spec §4: id はプローズブロックの属性行専用) -----------
//
// `{#...}` タグの併記が無く、行型ブロックの前置属性行に id= だけがある場合。

#[test]
fn id_not_allowed_on_heading_without_own_id_tag() {
    let out = parse("[id=foo]\n# Title\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

#[test]
fn id_not_allowed_on_fence_without_own_id_tag() {
    let out = parse("[id=foo]\n::math\nx = 1\n::\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

#[test]
fn id_not_allowed_on_list_even_when_item_has_own_id_tag() {
    // リスト全体を束縛する属性行の id= は、個々の項目の {#...} の有無に関わらず
    // 常に IdNotAllowedHere(項目とブロック単位の attrs は直接対応しないため)。
    let out = parse("[id=foo]\n- one {#item-1}\n- two\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

#[test]
fn id_not_allowed_on_list_without_any_item_id_tag() {
    let out = parse("[id=foo]\n- one\n- two\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
}

#[test]
fn duplicate_id_still_wins_over_id_not_allowed_when_both_tags_present() {
    // 既存の併記ケース(見出し自身に {#...} があり、かつ属性行にも id=)は
    // 引き続き DuplicateId のみで、IdNotAllowedHere は発火しない。
    let out = parse("[id=foo]\n# Title {#bar}\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
}

#[test]
fn bad_key_charset_attr_key() {
    let out = parse("[bad key=1]\nParagraph.\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{:?}", out.diags);
}

#[test]
fn bad_key_charset_alias() {
    let out = parse("# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=bad.alias}\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{:?}", out.diags);
}

#[test]
fn bad_key_charset_label() {
    let out = parse("# Title {#bad.label}\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{:?}", out.diags);
}
