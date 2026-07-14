//! D24(2026-07-14 裁定、WP-X2): ネストリストのパース・fmt・診断のテスト。
//!
//! - 2〜3段のネスト構造が `ListItem.child` に正しく入ること(`-` と `1.` の混在可)
//! - fmt がネスト項目にも行末 `{#ULID}` を注入し、冪等・純挿入の契約を守ること
//! - リストとして解釈できないインデント行(奇数スペース・対応親の無い深いインデント)が
//!   `InconsistentIndent`(Error)で診断されること — 従来の「無警告で `- ` 混じりの
//!   別段落に化ける」挙動の根絶(回帰テスト)

use strata_sml::{format_with, parse, BlockKind, DiagKind, ListItem};
use ulid::Ulid;

fn items_of(src: &str) -> Vec<ListItem> {
    let out = parse(src);
    assert!(out.diags.is_empty(), "unexpected diags: {:?}", out.diags);
    match &out.doc.blocks[0].kind {
        BlockKind::List { items, .. } => items.clone(),
        other => panic!("expected list, got {other:?}"),
    }
}

#[test]
fn two_level_nest_parses_into_child_list() {
    let src = "- top1\n  - sub1\n  - sub2\n- top2\n";
    let items = items_of(src);
    assert_eq!(items.len(), 2);
    let child = items[0].child.as_ref().expect("top1 must have a child list");
    assert!(!child.ordered);
    assert_eq!(child.items.len(), 2);
    assert!(child.items[0].child.is_none());
    assert!(items[1].child.is_none());
}

#[test]
fn three_level_nest_parses_recursively() {
    let src = "- a\n  - b\n    - c\n";
    let items = items_of(src);
    let b = &items[0].child.as_ref().unwrap().items[0];
    let c = &b.child.as_ref().unwrap().items[0];
    assert!(c.child.is_none());
}

/// `-` と `1.` の混在(D24): 子リストの ordered は子の先頭項目のマーカーで決まる。
#[test]
fn ordered_child_under_unordered_parent() {
    let src = "- top\n  1. first\n  2. second\n";
    let items = items_of(src);
    let child = items[0].child.as_ref().unwrap();
    assert!(child.ordered);
    assert_eq!(child.items.len(), 2);
}

/// ネスト項目の行末 `{#id}` タグも通常項目と同じ規則で抽出される。
#[test]
fn nested_item_id_tags_are_extracted() {
    let src = "- top {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n  - sub {#01ARZ3NDEKTSV4RRFFQ69G5FAW}\n";
    let items = items_of(src);
    assert!(items[0].id_tag.is_some());
    let sub = &items[0].child.as_ref().unwrap().items[0];
    assert!(sub.id_tag.is_some());
}

// ---- fmt: ネスト項目への ID 注入と冪等性 -------------------------------------------

fn seq_idgen(mut next: u128) -> impl FnMut() -> Ulid {
    move || {
        let ulid = Ulid(next);
        next += 1;
        ulid
    }
}

#[test]
fn fmt_injects_ids_into_nested_items_and_is_idempotent() {
    let src = "- top\n  - sub1\n  - sub2\n- top2\n";
    let mut idgen = seq_idgen(0x0001_8000_0000_0000_0000_0000_0000_0000);
    let out = format_with(src, &mut idgen).expect("nested list must format cleanly");

    // 純挿入(削除ゼロ)であること。
    assert!(out.patches.iter().all(|p| p.delete == 0), "{:?}", out.patches);

    // ネスト項目を含む全項目行の行末に {#ULID} が付くこと(インデントは保存)。
    let lines: Vec<&str> = out.text.lines().collect();
    let item_lines: Vec<&&str> = lines.iter().filter(|l| l.trim_start().starts_with("- ")).collect();
    assert_eq!(item_lines.len(), 4);
    for l in &item_lines {
        assert!(l.ends_with('}'), "item line must end with an id tag: {l}");
    }
    assert!(out.text.contains("  - sub1 {#"), "{}", out.text);

    // 冪等: 2回目の fmt はパッチゼロ。
    let mut idgen2 = seq_idgen(0x0002_8000_0000_0000_0000_0000_0000_0000);
    let out2 = format_with(&out.text, &mut idgen2).expect("second fmt must succeed");
    assert!(out2.patches.is_empty(), "fmt must be idempotent: {:?}", out2.patches);
    assert_eq!(out2.text, out.text);
}

/// ID 発行順(D-B4 + D24): リスト全体 → 親項目 → その子リストの項目(再帰)→ 次の
/// 兄弟項目、の文書順。
#[test]
fn fmt_id_issue_order_is_document_order_including_nested_items() {
    let src = "- a\n  - b\n- c\n";
    let mut idgen = seq_idgen(0x0001_8000_0000_0000_0000_0000_0000_0000);
    let out = format_with(src, &mut idgen).expect("must format");
    // フロントマター(1個目)→ リスト全体(2個目)→ a(3個目)→ b(4個目)→ c(5個目)。
    let body: Vec<&str> = out.text.lines().collect();
    let a_line = body.iter().find(|l| l.starts_with("- a")).unwrap();
    let b_line = body.iter().find(|l| l.trim_start().starts_with("- b")).unwrap();
    let c_line = body.iter().find(|l| l.starts_with("- c")).unwrap();
    let id_of = |line: &str| line.rsplit("{#").next().unwrap().trim_end_matches('}').to_string();
    let (ia, ib, ic) = (id_of(a_line), id_of(b_line), id_of(c_line));
    assert!(ia < ib && ib < ic, "document order violated: a={ia} b={ib} c={ic}");
}

// ---- 診断: 従来の無警告誤パースの根絶(D24 回帰テスト) ------------------------------

/// 奇数スペースのインデント → InconsistentIndent(Error)。
#[test]
fn odd_indent_list_line_is_diagnosed() {
    let out = parse("- top\n   - sub\n");
    assert!(
        out.diags.iter().any(|d| d.kind == DiagKind::InconsistentIndent),
        "{:?}",
        out.diags
    );
    assert!(out.diags.iter().all(|d| d.is_error()), "must be Error severity: {:?}", out.diags);
}

/// 対応する親の無い深すぎるインデント(0段の直後に2段)→ InconsistentIndent。
#[test]
fn too_deep_indent_jump_is_diagnosed() {
    let out = parse("- top\n    - too-deep\n");
    assert!(
        out.diags.iter().any(|d| d.kind == DiagKind::InconsistentIndent),
        "{:?}",
        out.diags
    );
}

/// 回帰: 旧実装では「段落の続きにインデントされた `- ` 行」が無警告で段落の一部に
/// 化けていた。D24 以降はリスト項目行として切り出され、インデントが不正なら診断が出る
/// (無警告で通ることは決してない)。
#[test]
fn old_silent_misparse_paragraph_with_indented_marker_now_diagnosed() {
    let src = "本文の段落です。\n  - 旧実装では段落に化けていた行\n";
    let out = parse(src);
    // 段落 + リストの2ブロックに分かれ、インデント不正(対応親なし)の診断が出る。
    assert!(
        out.diags.iter().any(|d| d.kind == DiagKind::InconsistentIndent),
        "{:?}",
        out.diags
    );
    assert_eq!(out.doc.blocks.len(), 2, "{:#?}", out.doc.blocks);
}

/// 正しい平坦リスト・正しいネストは引き続き無診断(全か無かで既存文書を壊さない)。
#[test]
fn flat_and_properly_nested_lists_have_no_diags() {
    for src in ["- a\n- b\n", "- a\n  - b\n    - c\n- d\n"] {
        let out = parse(src);
        assert!(out.diags.is_empty(), "{src:?}: {:?}", out.diags);
    }
}
