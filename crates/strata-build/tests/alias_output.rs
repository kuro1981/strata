//! WP-Y1(D26、sml-spec.md §1.5): build が解決済みエイリアスを graph JSON に出力する。
//!
//! - alias 付きノードは `Node.alias` にそのエイリアス文字列が入ること
//! - alias 無しノードは `Node.alias` が `None`(JSON に "alias" キーが出ない)こと
//! - JSON 往復でも保持されること

use strata_build::build;
use strata_core::NodeId;
use ulid::Ulid;

#[test]
fn aliased_heading_carries_its_alias_on_the_node() {
    let ulid = Ulid::new();
    let src = format!("# Title {{#{ulid} alias=my-title}}\n");
    let out = build(&src).expect("must build");
    let node = &out.graph.nodes[&NodeId(ulid)];
    assert_eq!(node.alias.as_deref(), Some("my-title"));
}

#[test]
fn heading_without_alias_has_none() {
    let ulid = Ulid::new();
    let src = format!("# Title {{#{ulid}}}\n");
    let out = build(&src).expect("must build");
    let node = &out.graph.nodes[&NodeId(ulid)];
    assert_eq!(node.alias, None);
}

/// プローズブロック(段落・リスト全体)の前置属性行 `[id=ULID, alias=x]` からも
/// alias が写ること。
#[test]
fn aliased_paragraph_via_attr_line_carries_its_alias() {
    let ulid = Ulid::new();
    let src = format!("[id={ulid}, alias=key-finding]\nA paragraph.\n");
    let out = build(&src).expect("must build");
    let node = &out.graph.nodes[&NodeId(ulid)];
    assert_eq!(node.alias.as_deref(), Some("key-finding"));
}

/// alias 付きノードの JSON に "alias" キーが現れ、無しノードには現れないこと
/// (後方互換フィールド、skip_serializing_if)。往復でも一致すること。
#[test]
fn alias_roundtrips_through_build_output_json() {
    let aliased = Ulid::new();
    let plain = Ulid::new();
    let src = format!("# Aliased {{#{aliased} alias=a}}\n\n## Plain {{#{plain}}}\n");
    let out = build(&src).expect("must build");

    let json = serde_json::to_string(&out).unwrap();
    assert!(json.contains(r#""alias":"a""#), "{json}");

    let back: strata_build::BuildOutput = serde_json::from_str(&json).unwrap();
    assert_eq!(out, back);
    assert_eq!(back.graph.nodes[&NodeId(plain)].alias, None);
}

/// エイリアスで参照されたノード(alias 経由で解決された Ref のターゲット)も
/// 自身の alias を保持していること(D26 の主眼: ビューが alias から直接引ける)。
#[test]
fn alias_is_present_even_when_only_referenced_by_alias() {
    let table_ulid = Ulid::new();
    let para_ulid = Ulid::new();
    let src = format!(
        "::table {{#{table_ulid} alias=eval-table}}\n@rows:\n  - a: [x]\n::\n\n[id={para_ulid}]\n[ref](table:eval-table) here.\n"
    );
    let out = build(&src).expect("must build");
    let node = &out.graph.nodes[&NodeId(table_ulid)];
    assert_eq!(node.alias.as_deref(), Some("eval-table"));
}
