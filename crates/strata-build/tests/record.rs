//! WP-Y3(D28/D29、sml-spec.md §1.5): `::record` フェンス + 日付・期間セル値型の
//! build 側テスト。
//!
//! - Record ノードがグラフに格納され、キー/値の順序が保存されること
//! - Date/Period セル値が検証・変換されること
//! - record の `alias`(D26)も他フェンス同様に写ること
//! - 診断(空キー・`:` 無し・重複キー)が「全か無か」の対象になる/ならないこと

use strata_build::build;
use strata_core::{CellValue, DateValue, NodeId, NodePayload};
use ulid::Ulid;

#[test]
fn record_node_is_stored_with_ordered_entries() {
    let ulid = Ulid::new();
    let src = format!("::record {{#{ulid} alias=basic-info}}\n姓: 山田\n名: 太郎\n生年月日: 1997-03-15\n::\n");
    let out = build(&src).expect("must build");

    let node = &out.graph.nodes[&NodeId(ulid)];
    assert_eq!(node.alias.as_deref(), Some("basic-info"));
    match &node.payload {
        NodePayload::Record(r) => {
            assert_eq!(r.entries.len(), 3);
            assert_eq!(r.entries[0].key, "姓");
            assert_eq!(r.entries[0].value, CellValue::Text { v: "山田".to_string() });
            assert_eq!(r.entries[1].key, "名");
            assert_eq!(r.entries[2].key, "生年月日");
            assert_eq!(r.entries[2].value, CellValue::Date(DateValue { y: 1997, m: 3, d: Some(15) }));
        }
        other => panic!("expected Record, got {other:?}"),
    }
}

#[test]
fn record_is_contained_by_parent_section() {
    let sec = Ulid::new();
    let rec = Ulid::new();
    let src = format!("# 基本情報 {{#{sec}}}\n\n::record {{#{rec}}}\n姓: 山田\n::\n");
    let out = build(&src).expect("must build");
    assert_eq!(out.graph.children_of(NodeId(sec)), vec![NodeId(rec)]);
}

#[test]
fn period_value_with_ongoing_to_is_none() {
    let ulid = Ulid::new();
    let src = format!("::record {{#{ulid}}}\n在籍期間: 2020-10 〜 現在\n::\n");
    let out = build(&src).expect("must build");
    match &out.graph.nodes[&NodeId(ulid)].payload {
        NodePayload::Record(r) => {
            assert_eq!(
                r.entries[0].value,
                CellValue::Period { from: DateValue { y: 2020, m: 10, d: None }, to: None }
            );
        }
        other => panic!("expected Record, got {other:?}"),
    }
}

#[test]
fn date_format_attr_enables_japanese_date_parsing() {
    let ulid = Ulid::new();
    let src = format!(
        "::record {{#{ulid}}}\n[date-format=\"YYYY年M月D日\"]\n生年月日: 1997年3月15日\n::\n"
    );
    let out = build(&src).expect("must build");
    match &out.graph.nodes[&NodeId(ulid)].payload {
        NodePayload::Record(r) => {
            assert_eq!(r.entries[0].value, CellValue::Date(DateValue { y: 1997, m: 3, d: Some(15) }));
        }
        other => panic!("expected Record, got {other:?}"),
    }
}

/// 空キー・`:` 無し行は `Error`(全か無かで build 失敗)。
#[test]
fn record_missing_colon_fails_the_build() {
    let ulid = Ulid::new();
    let src = format!("::record {{#{ulid}}}\nキーだけの行\n::\n");
    let errors = build(&src).expect_err("missing colon must fail the build");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            strata_build::BuildError::Parse(d) if d.kind == strata_sml::DiagKind::RecordMissingColon
        )),
        "{errors:#?}"
    );
}

/// 重複キーは `Warning`(全か無かの対象外。build は成功し warnings に積まれる)。
#[test]
fn record_duplicate_key_is_a_warning_and_build_succeeds() {
    let ulid = Ulid::new();
    let src = format!("::record {{#{ulid}}}\n姓: 山田\n姓: 田中\n::\n");
    let out = build(&src).expect("duplicate key is a Warning, build must succeed");
    assert!(
        out.warnings.iter().any(|w| w.kind == strata_sml::DiagKind::DuplicateRecordKey),
        "{:#?}",
        out.warnings
    );
    match &out.graph.nodes[&NodeId(ulid)].payload {
        NodePayload::Record(r) => assert_eq!(r.entries.len(), 2, "both values must be retained"),
        other => panic!("expected Record, got {other:?}"),
    }
}

/// ISO っぽいが不正な日付(13月)は診断されつつ Text へフォールバックする
/// (record 内の値パースは Error だが、パース自体は継続する = 全か無かで build 失敗)。
#[test]
fn invalid_iso_looking_date_fails_the_build_with_bad_date_value() {
    let ulid = Ulid::new();
    let src = format!("::record {{#{ulid}}}\n日付: 2026-13-01\n::\n");
    let errors = build(&src).expect_err("invalid date value must fail the build");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            strata_build::BuildError::Parse(d) if d.kind == strata_sml::DiagKind::BadDateValue
        )),
        "{errors:#?}"
    );
}
