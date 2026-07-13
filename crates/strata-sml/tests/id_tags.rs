//! IDタグ4形(なし / `{#ULID}` / `{#label}` / `{#ULID alias=x}`)× 行型ブロック位置
//! (見出し / リスト項目 / フェンスマーカー)の組み合わせテスト
//! (sml-parser-m1-handoff.md WP2 受け入れ条件)。

use strata_sml::{parse, BlockKind, RefTarget};
use ulid::Ulid;

fn heading_id_tag(src: &str) -> Option<strata_sml::IdTag> {
    let out = parse(src);
    match &out.doc.blocks[0].kind {
        BlockKind::Heading { id_tag, .. } => id_tag.clone(),
        other => panic!("expected heading, got {other:?}"),
    }
}

fn first_list_item_id_tag(src: &str) -> Option<strata_sml::IdTag> {
    let out = parse(src);
    match &out.doc.blocks[0].kind {
        BlockKind::List { items, .. } => items[0].id_tag.clone(),
        other => panic!("expected list, got {other:?}"),
    }
}

fn fence_id_tag(src: &str) -> Option<strata_sml::IdTag> {
    let out = parse(src);
    match &out.doc.blocks[0].kind {
        BlockKind::Fence(fb) => fb.id_tag.clone(),
        other => panic!("expected fence, got {other:?}"),
    }
}

// ---- 見出し -------------------------------------------------------------

#[test]
fn heading_none() {
    assert!(heading_id_tag("# Title\n").is_none());
}

#[test]
fn heading_ulid() {
    let u = Ulid::new().to_string();
    let tag = heading_id_tag(&format!("# Title {{#{u}}}\n")).unwrap();
    assert!(matches!(tag.id, RefTarget::Ulid(_)));
    assert!(tag.alias.is_none());
}

#[test]
fn heading_label() {
    let tag = heading_id_tag("# Title {#analysis}\n").unwrap();
    assert_eq!(tag.id, RefTarget::Label("analysis".into()));
    assert!(tag.alias.is_none());
}

#[test]
fn heading_ulid_alias() {
    let u = Ulid::new().to_string();
    let tag = heading_id_tag(&format!("# Title {{#{u} alias=analysis}}\n")).unwrap();
    assert!(matches!(tag.id, RefTarget::Ulid(_)));
    assert_eq!(tag.alias.as_deref(), Some("analysis"));
}

// ---- リスト項目 ----------------------------------------------------------

#[test]
fn list_item_none() {
    assert!(first_list_item_id_tag("- item one\n").is_none());
}

#[test]
fn list_item_ulid() {
    let u = Ulid::new().to_string();
    let tag = first_list_item_id_tag(&format!("- item one {{#{u}}}\n")).unwrap();
    assert!(matches!(tag.id, RefTarget::Ulid(_)));
}

#[test]
fn list_item_label() {
    let tag = first_list_item_id_tag("- item one {#item-1}\n").unwrap();
    assert_eq!(tag.id, RefTarget::Label("item-1".into()));
}

#[test]
fn list_item_ulid_alias() {
    let u = Ulid::new().to_string();
    let tag = first_list_item_id_tag(&format!("- item one {{#{u} alias=item-1}}\n")).unwrap();
    assert!(matches!(tag.id, RefTarget::Ulid(_)));
    assert_eq!(tag.alias.as_deref(), Some("item-1"));
}

// ---- フェンスマーカー -----------------------------------------------------

#[test]
fn fence_marker_none() {
    let src = "::math\nx = 1\n::\n";
    assert!(fence_id_tag(src).is_none());
}

#[test]
fn fence_marker_ulid() {
    let u = Ulid::new().to_string();
    let src = format!("::math {{#{u}}}\nx = 1\n::\n");
    let tag = fence_id_tag(&src).unwrap();
    assert!(matches!(tag.id, RefTarget::Ulid(_)));
}

#[test]
fn fence_marker_label() {
    let src = "::math {#loss-formula}\nx = 1\n::\n";
    let tag = fence_id_tag(src).unwrap();
    assert_eq!(tag.id, RefTarget::Label("loss-formula".into()));
}

#[test]
fn fence_marker_ulid_alias() {
    let u = Ulid::new().to_string();
    let src = format!("::math {{#{u} alias=loss-formula}}\nx = 1\n::\n");
    let tag = fence_id_tag(&src).unwrap();
    assert!(matches!(tag.id, RefTarget::Ulid(_)));
    assert_eq!(tag.alias.as_deref(), Some("loss-formula"));
}
