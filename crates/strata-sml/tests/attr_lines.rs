//! 属性行のテスト(単一値/リスト値/引用符値/孤立/DuplicateId/BadKeyCharset)
//! (sml-parser-m1-handoff.md WP2 受け入れ条件)。

use strata_sml::{parse, AttrValue, BlockKind, DiagKind, RefTarget};

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

// ---- D11(2026-07-14 改定): リスト全体はプローズ扱い、前置属性行の id= を許す ------
//
// 項目の `{#id}`(行型)とリスト全体の `[id=...]`(プローズ属性行)は別エンティティ
// なので併記可(DuplicateId にも IdNotAllowedHere にもならない)。

#[test]
fn id_allowed_on_list_even_when_item_has_own_id_tag() {
    let out = parse("[id=foo]\n- one {#item-1}\n- two\n");
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

#[test]
fn id_allowed_on_list_without_any_item_id_tag() {
    let out = parse("[id=foo]\n- one\n- two\n");
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
}

/// リスト全体の `[id=...]` と項目の `{#id}` は別エンティティなので独立に共存する
/// (どちらも正しくパースされ、互いに干渉しない)。
#[test]
fn list_id_and_item_id_tag_coexist_as_independent_entities() {
    let out = parse("[id=list-label]\n- one {#item-label}\n- two\n");
    assert!(out.diags.is_empty(), "{:?}", out.diags);

    let block = &out.doc.blocks[0];
    let attrs = block.attrs.as_ref().expect("expected attr line on list block");
    assert_eq!(attrs.entries[0].0, "id");
    assert_eq!(attrs.entries[0].1, AttrValue::Single("list-label".to_string()));

    match &block.kind {
        BlockKind::List { items, .. } => {
            let tag = items[0].id_tag.as_ref().expect("expected item id tag");
            assert_eq!(tag.id, RefTarget::Label("item-label".to_string()));
            assert!(items[1].id_tag.is_none());
        }
        other => panic!("expected list, got {other:?}"),
    }
}

#[test]
fn duplicate_id_still_wins_over_id_not_allowed_when_both_tags_present() {
    // 既存の併記ケース(見出し自身に {#...} があり、かつ属性行にも id=)は
    // 引き続き DuplicateId のみで、IdNotAllowedHere は発火しない。
    let out = parse("[id=foo]\n# Title {#bar}\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
}

// ---- D10(2026-07-14 改定): コードフェンスは行型ブロック(前置属性行の id= は不可) --

#[test]
fn id_not_allowed_on_code_fence_without_own_id_tag() {
    let out = parse("[id=foo]\n```rust\nfn main() {}\n```\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
}

#[test]
fn duplicate_id_on_code_fence_with_own_id_tag() {
    let out = parse("[id=foo]\n```rust {#bar}\nfn main() {}\n```\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{:?}", out.diags);
    assert!(!out.diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere), "{:?}", out.diags);
}

/// D11(2026-07-14 改定): `check_id_value` の対象にリストが追加された。
/// リストの前置属性行 `id=` にも段落と同じ値検証(裸トークンのみ・字句制約)が働く。
#[test]
fn list_id_value_is_validated_like_paragraph() {
    let out = parse("[id=\"quoted\"]\n- one\n- two\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::BadIdValue), "{:?}", out.diags);

    let out = parse("[id=bad.label]\n- one\n- two\n");
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{:?}", out.diags);

    let out = parse("[id=good-label]\n- one\n- two\n");
    assert!(out.diags.is_empty(), "{:?}", out.diags);
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
