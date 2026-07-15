//! フロントマター(sml-spec.md §2.1、D12)の単体テスト(WP-B1)。
//!
//! 検証すること:
//! - 正常形2種(id のみ / id+title)
//! - 未知キー → `UnknownFrontmatterKey`
//! - id が非 ULID → `BadIdValue`
//! - 閉じ `---` 欠落 → `UnclosedFrontmatter`
//! - ファイル途中の `---` は単なる段落(フロントマターとして解釈されない)

use strata_sml::{parse, BlockKind, DiagKind, RefTarget};
use ulid::Ulid;

// ---- 正常形 ---------------------------------------------------------------

#[test]
fn frontmatter_with_id_only() {
    let ulid = Ulid::new().to_string();
    let src = format!("---\nid: {ulid}\n---\n\n# Title\n");
    let out = parse(&src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);

    let fm = out.doc.frontmatter.expect("expected frontmatter");
    let (target, _) = fm.id.expect("expected id");
    assert_eq!(target, RefTarget::Ulid(ulid.parse().unwrap()));
    assert!(fm.title.is_none());

    // フロントマターの後ろの本文は通常どおりブロックとして続く。
    assert_eq!(out.doc.blocks.len(), 1);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Heading { .. }));
}

#[test]
fn frontmatter_with_id_and_title() {
    let ulid = Ulid::new().to_string();
    let src = format!("---\nid: {ulid}\ntitle: 機械学習モデルの評価レポート\n---\n\n# Title\n");
    let out = parse(&src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);

    let fm = out.doc.frontmatter.expect("expected frontmatter");
    let (target, _) = fm.id.expect("expected id");
    assert_eq!(target, RefTarget::Ulid(ulid.parse().unwrap()));
    assert_eq!(fm.title.as_deref(), Some("機械学習モデルの評価レポート"));
}

/// コロン後の空白は任意(sml-spec §2.1)。
#[test]
fn frontmatter_colon_without_following_space_is_accepted() {
    let ulid = Ulid::new().to_string();
    let src = format!("---\nid:{ulid}\n---\n");
    let out = parse(&src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    let fm = out.doc.frontmatter.expect("expected frontmatter");
    let (target, _) = fm.id.expect("expected id");
    assert_eq!(target, RefTarget::Ulid(ulid.parse().unwrap()));
}

// ---- 診断 -------------------------------------------------------------------

#[test]
fn unknown_frontmatter_key_is_diagnosed() {
    let src = "---\nauthor: someone\n---\n";
    let out = parse(src);
    assert!(
        out.diags.iter().any(|d| d.kind == DiagKind::UnknownFrontmatterKey),
        "{:?}",
        out.diags
    );
    // 未知キーでもフロントマター自体は構築される(全か無かの判定は呼び出し側の仕事)。
    assert!(out.doc.frontmatter.is_some());
}

#[test]
fn non_ulid_frontmatter_id_is_bad_id_value() {
    let src = "---\nid: my-label\n---\n";
    let out = parse(src);
    assert!(out.diags.iter().any(|d| d.kind == DiagKind::BadIdValue), "{:?}", out.diags);
    let fm = out.doc.frontmatter.expect("expected frontmatter");
    let (target, _) = fm.id.expect("expected id");
    assert_eq!(target, RefTarget::Label("my-label".to_string()));
}

#[test]
fn unclosed_frontmatter_is_diagnosed() {
    let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: no closing delimiter\n";
    let out = parse(src);
    assert!(
        out.diags.iter().any(|d| d.kind == DiagKind::UnclosedFrontmatter),
        "{:?}",
        out.diags
    );
    // 全か無かの判定はしない: best-effort で id/title は拾ったまま返す。
    let fm = out.doc.frontmatter.expect("expected frontmatter");
    assert!(fm.id.is_some());
    assert_eq!(fm.title.as_deref(), Some("no closing delimiter"));
    // 閉じが無いので本文ブロックは残らない(ファイル末尾まで飲み込む)。
    assert!(out.doc.blocks.is_empty());
}

// ---- ファイル途中の --- は段落扱い -------------------------------------------

#[test]
fn mid_file_triple_dash_is_a_paragraph_not_frontmatter() {
    // M6(D40): 文中の孤立した `---` はもはや段落フォールバックではなく
    // `ThematicBreak`(水平線)になる(監査④の解消)。フロントマターとの無衝突
    // (オフセット0限定判定)自体は変わらない。
    let src = "# Title\n\n---\n\nMore text.\n";
    let out = parse(src);
    assert!(out.diags.is_empty(), "{:?}", out.diags);
    assert!(out.doc.frontmatter.is_none());
    assert_eq!(out.doc.blocks.len(), 3);
    assert!(matches!(out.doc.blocks[0].kind, BlockKind::Heading { .. }));
    assert!(matches!(out.doc.blocks[1].kind, BlockKind::ThematicBreak));
    assert!(matches!(out.doc.blocks[2].kind, BlockKind::Paragraph { .. }));
}

#[test]
fn file_without_frontmatter_has_none() {
    let out = parse("# Title\n");
    assert!(out.doc.frontmatter.is_none());
}

/// `---` に前後の空白や余計な文字が付くと単独行と認めない(段落フォールバック)。
#[test]
fn triple_dash_with_extra_chars_is_not_frontmatter() {
    let src = "----\nid: x\n----\n";
    let out = parse(src);
    assert!(out.doc.frontmatter.is_none());
}
