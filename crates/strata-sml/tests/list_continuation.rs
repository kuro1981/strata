//! D52(2026-07-16 裁定、sml-spec.md §1.14): リスト項目の継続行(lazy continuation)。
//!
//! CommonMark 準拠で、空行を挟まずマーカー行の後に続く非マーカー行は項目本文へ
//! 併合される(「項目=段落1つ」の制約は維持——その段落が複数ソース行にまたがれる
//! ようになるだけ)。notes ドッグフーディング(M8)で踏んだ「折り返し付きリストが
//! 無警告で別段落ブロックに分断される」バグの回帰テスト。

use strata_sml::{format_with, parse, BlockKind, SmlInline};
use ulid::Ulid;

fn seq_idgen(mut next: u128) -> impl FnMut() -> Ulid {
    move || {
        let ulid = Ulid(next);
        next += 1;
        ulid
    }
}

fn plain_text(src: &str, inline: &[SmlInline]) -> String {
    inline
        .iter()
        .map(|i| match i {
            SmlInline::Text(sp) => sp.slice(src).to_string(),
            other => panic!("expected plain text only in this fixture: {other:?}"),
        })
        .collect()
}

/// 再現ケース(M8 で踏んだ形): `- 長い文…\n  続き` が1項目1ブロックになる。
#[test]
fn continuation_line_merges_into_single_item_block() {
    let src = "- 長い文…\n  続き\n";
    let out = parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 1, "継続行が独立ブロックへ分断されてはいけない: {:#?}", out.doc.blocks);
    match &out.doc.blocks[0].kind {
        BlockKind::List { items, .. } => {
            assert_eq!(items.len(), 1);
            let text = plain_text(src, &items[0].inline);
            assert_eq!(text, "長い文…\n  続き");
        }
        other => panic!("expected list, got {other:?}"),
    }
}

/// 複数の継続行、かつ複数項目にまたがる場合も正しく各項目へ振り分けられる。
#[test]
fn multiple_continuation_lines_attach_to_correct_item() {
    let src = "- one\n  cont1\n  cont2\n- two\n  cont3\n";
    let out = parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    match &out.doc.blocks[0].kind {
        BlockKind::List { items, .. } => {
            assert_eq!(items.len(), 2);
            assert_eq!(plain_text(src, &items[0].inline), "one\n  cont1\n  cont2");
            assert_eq!(plain_text(src, &items[1].inline), "two\n  cont3");
        }
        other => panic!("expected list, got {other:?}"),
    }
}

/// 空行はブロック境界のまま: 継続行にはならず、リストはそこで終わる。
#[test]
fn blank_line_still_ends_the_item_and_the_list() {
    let src = "- one\n\ncontinuation-like paragraph\n";
    let out = parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert_eq!(out.doc.blocks.len(), 2, "{:#?}", out.doc.blocks);
    match &out.doc.blocks[0].kind {
        BlockKind::List { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_eq!(plain_text(src, &items[0].inline), "one");
        }
        other => panic!("expected list, got {other:?}"),
    }
    assert!(matches!(&out.doc.blocks[1].kind, BlockKind::Paragraph { .. }));
}

/// ネストリストとの判定順序: マーカー行(2スペース `- `)は継続行に飲まれず、
/// 子項目として認識される。
#[test]
fn marker_line_after_continuation_still_becomes_child_item() {
    let src = "- top\n  wrap\n  - child\n";
    let out = parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    match &out.doc.blocks[0].kind {
        BlockKind::List { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_eq!(plain_text(src, &items[0].inline), "top\n  wrap");
            let child = items[0].child.as_ref().expect("marker line must become a child list, not continuation text");
            assert_eq!(child.items.len(), 1);
            assert_eq!(plain_text(src, &child.items[0].inline), "child");
        }
        other => panic!("expected list, got {other:?}"),
    }
}

/// タスクリストのチェック状態はマーカー行からのみ読む(継続行には無い)。
#[test]
fn task_list_checkbox_with_continuation() {
    let src = "- [x] done\n  more detail\n";
    let out = parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    match &out.doc.blocks[0].kind {
        BlockKind::List { items, .. } => {
            assert_eq!(items[0].checked, Some(true));
            assert_eq!(plain_text(src, &items[0].inline), "done\n  more detail");
        }
        other => panic!("expected list, got {other:?}"),
    }
}

// ---- fmt: {#id} タグは項目の最終行末尾に注入(Setext・D40 と同型) -------------------

#[test]
fn fmt_injects_id_tag_at_last_continuation_line_and_is_idempotent() {
    let src = "- 長い文…\n  続き\n";
    let mut idgen = seq_idgen(0x0001_8000_0000_0000_0000_0000_0000_0000);
    let out = format_with(src, &mut idgen).expect("must format cleanly");

    // 純挿入(削除ゼロ)。
    assert!(out.patches.iter().all(|p| p.delete == 0), "{:?}", out.patches);

    let lines: Vec<&str> = out.text.lines().collect();
    // マーカー行自体には id タグが付かず、最終行(継続行)の末尾に付く。
    let marker_line = lines.iter().find(|l| l.trim_start().starts_with("- 長い文")).unwrap();
    assert!(!marker_line.contains("{#"), "marker line must not carry the id tag: {marker_line}");
    let cont_line = lines.iter().find(|l| l.contains("続き")).unwrap();
    assert!(cont_line.trim_end().ends_with('}'), "continuation line must carry the trailing id tag: {cont_line}");

    // 冪等: 2回目の fmt はパッチゼロ。
    let mut idgen2 = seq_idgen(0x0002_8000_0000_0000_0000_0000_0000_0000);
    let out2 = format_with(&out.text, &mut idgen2).expect("second fmt must succeed");
    assert!(out2.patches.is_empty(), "fmt must be idempotent: {:?}", out2.patches);
    assert_eq!(out2.text, out.text);
}

/// 継続行が無い従来どおりの単一行項目は、これまでどおりマーカー行自身の末尾に付く。
#[test]
fn fmt_single_line_item_id_tag_unaffected() {
    let src = "- one\n";
    let mut idgen = seq_idgen(0x0001_8000_0000_0000_0000_0000_0000_0000);
    let out = format_with(src, &mut idgen).expect("must format cleanly");
    let line = out.text.lines().find(|l| l.starts_with("- one")).expect("item line must survive fmt");
    assert!(line.starts_with("- one {#") && line.trim_end().ends_with('}'), "{line}");
}
