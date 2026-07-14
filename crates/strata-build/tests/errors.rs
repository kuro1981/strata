//! WP-B4 完了チェックリスト: エラー全種の発火を確認する(全か無か、D13)。
//!
//! `MissingId` は draft fixture(`docs/sml_example_draft.sml`、ULID 未付与)で発火する
//! ことを確認する。他のエラー種(`Parse` / `DuplicateAlias` / `UnresolvedAlias` /
//! `BadFigure` / `Math` / `RefTypeMismatch`)は最小の合成入力で個別に確認する
//! (`Invariant` は tests/misc.rs の dangling テストで確認)。

use strata_build::{build, BuildError};
use ulid::Ulid;

const DRAFT: &str = include_str!("../../../docs/sml_example_draft.sml");

#[test]
fn draft_fixture_triggers_missing_id_and_is_all_or_nothing() {
    let errors = build(DRAFT).expect_err("draft fixture has no ULIDs yet (fmt not run)");
    assert!(!errors.is_empty());
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::MissingId { .. })),
        "expected at least one MissingId, got: {errors:#?}"
    );
    // draft の全ブロックが ID 未付与なので、多数の MissingId が全件収集されているはず
    // (最初の1件で止まらない、sml-spec §8.2 / D13 と同じ方針)。
    let missing_id_count = errors.iter().filter(|e| matches!(e, BuildError::MissingId { .. })).count();
    assert!(missing_id_count > 5, "expected many MissingId errors, got {missing_id_count}: {errors:#?}");
}

#[test]
fn duplicate_alias_is_reported_with_all_definition_spans() {
    let a = Ulid::new();
    let b = Ulid::new();
    let src = format!("# A {{#{a} alias=dup}}\n\n# B {{#{b} alias=dup}}\n");
    let errors = build(&src).expect_err("duplicate alias must fail the whole build");
    let dup = errors.iter().find_map(|e| match e {
        BuildError::DuplicateAlias { alias, spans } if alias == "dup" => Some(spans.clone()),
        _ => None,
    });
    let spans = dup.unwrap_or_else(|| panic!("expected DuplicateAlias(\"dup\"), got: {errors:#?}"));
    assert_eq!(spans.len(), 2, "both definition sites must be reported");
}

#[test]
fn unresolved_alias_target_is_reported() {
    let a = Ulid::new();
    let src = format!("[id={a}]\nSee [it](ref:does-not-exist).\n");
    let errors = build(&src).expect_err("unresolved alias must fail the whole build");
    assert!(
        errors.iter().any(|e| matches!(e, BuildError::UnresolvedAlias { alias, .. } if alias == "does-not-exist")),
        "got: {errors:#?}"
    );
}

#[test]
fn figure_missing_kind_is_bad_figure() {
    let a = Ulid::new();
    let src = format!("::figure {{#{a}}}\n[caption=\"no kind here\"]\n::\n");
    let errors = build(&src).expect_err("figure without kind must fail");
    assert!(errors.iter().any(|e| matches!(e, BuildError::BadFigure { .. })), "got: {errors:#?}");
}

#[test]
fn chart_figure_missing_required_attrs_is_bad_figure() {
    let a = Ulid::new();
    let src = format!("::figure {{#{a}}}\n[kind=chart]\n::\n");
    let errors = build(&src).expect_err("chart figure missing data-ref/mark/encode must fail");
    assert!(errors.iter().any(|e| matches!(e, BuildError::BadFigure { .. })), "got: {errors:#?}");
}

#[test]
fn ref_scheme_type_mismatch_is_reported() {
    // `table:` は Table ノードを指すことを要求する。ここでは見出し(Section)を
    // `table:` で参照するので型不一致になるはず(sml-build-m3-handoff.md D-B5、裁量事項)。
    let heading_id = Ulid::new();
    let para_id = Ulid::new();
    let src = format!(
        "# Heading {{#{heading_id}}}\n\n[id={para_id}]\nSee [it](table:{heading_id}).\n"
    );
    let errors = build(&src).expect_err("table: pointing at a Section must fail");
    assert!(errors.iter().any(|e| matches!(e, BuildError::RefTypeMismatch { .. })), "got: {errors:#?}");
}

#[test]
fn parser_diags_are_wrapped_as_parse_errors() {
    // 閉じられていないフェンス → パーサが UnclosedFence を積む → BuildError::Parse。
    let a = Ulid::new();
    let src = format!("::math {{#{a}}}\nx^2\n");
    let errors = build(&src).expect_err("unclosed fence must fail the whole build");
    assert!(
        errors.iter().any(|e| matches!(
            e,
            BuildError::Parse(d) if d.kind == strata_sml::DiagKind::UnclosedFence
        )),
        "got: {errors:#?}"
    );
}

#[test]
fn math_unknown_command_is_reported() {
    let a = Ulid::new();
    let src = format!("::math {{#{a}}}\n\\notarealcommand\n::\n");
    let errors = build(&src).expect_err("unknown TeX command must fail");
    assert!(errors.iter().any(|e| matches!(e, BuildError::Math { .. })), "got: {errors:#?}");
}
