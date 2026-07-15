//! WP-W1/WP-W2 のセレクタ・コンビネータ・profile 分岐・決定性・--check 単体テスト。
//! 小さな合成 SML を `strata_build::build` で直接グラフ化し、view 定義(YAML)を
//! 適用する(実データの resume.sml / work_history.sml は WP-W3 の別リポジトリで
//! 検証する)。
use strata_build::build;
use strata_view::{apply, check, parse_manifest, parse_view_def};

fn yaml_of(src: &str, view_yaml: &str) -> String {
    let out = build(src).expect("sml must build");
    let view = parse_view_def(view_yaml).expect("view def must parse");
    let (files, warnings) = apply(&out, &view, None).expect("apply must succeed");
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
    assert_eq!(files.len(), 1);
    files[0].yaml.clone()
}

// --------------------------------------------------------------------------
// セレクタ: alias / record-field / pick
// --------------------------------------------------------------------------

#[test]
fn alias_and_record_field_selector_with_pick() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n姓: 山田\n名: 太郎\n::\n";
    let view = r#"
version: 1
profiles: [submit]
files:
  profile.yaml:
    content:
      fields:
        性:
          pick:
            of:
              record-field:
                of: { alias: basic-info }
                key: 姓
        名:
          pick:
            of:
              record-field:
                of: { alias: basic-info }
                key: 名
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "性: 山田\n名: 太郎\n");
}

#[test]
fn unresolved_alias_is_reported_as_an_error() {
    let out = build("").unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      pick:
        of: { alias: does-not-exist }
"#,
    )
    .unwrap();
    let err = apply(&out, &view, None).unwrap_err();
    assert!(err.contains("does-not-exist"), "{err}");
}

// --------------------------------------------------------------------------
// 糖衣構文: 裸文字列 `alias.キー` = pick(record-field) の略記(D35)
// --------------------------------------------------------------------------

/// 糖衣構文の出力は完全形と(直前のテストと)バイト同一でなければならない
/// (機械的な脱糖であって振る舞いを変えないことの検証)。
#[test]
fn bare_alias_dot_key_is_sugar_for_pick_record_field() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n姓: 山田\n名: 太郎\n::\n";
    let view = r#"
version: 1
profiles: [submit]
files:
  profile.yaml:
    content:
      fields:
        性: basic-info.姓
        名: basic-info.名
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "性: 山田\n名: 太郎\n");
}

/// 糖衣は `rows: table` の `item` の中でも使える(表モードの列参照ではなく、
/// あくまで record-field 抽出の略記であることの確認: ここでは行の外にある
/// record を alias で直接引く)。
#[test]
fn sugar_works_inside_rows_item_fields() {
    let src = "\
::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}
姓: 山田
::

::table {#01ARZ3NDEKTSV4RRFFQ69G5FA1 alias=education}
@rows:
  - entry: [a]
@cols:
  - field:
    - text
@cells:
  a | text : \"卒業\"
::
";
    let view = r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      rows:
        table: { alias: education }
        item:
          fields:
            姓: basic-info.姓
            学歴: { pick: { of: { cell: { col: text } } } }
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "- 姓: 山田\n  学歴: 卒業\n");
}

/// ドットの無い裸文字列(例えば `self` を糖衣位置に書き間違えた場合)は
/// record-field 抽出として解釈できないため、明示的なパースエラーになる
/// (セレクタの `self` キーワードとは型が別で、暗黙に読み替えられたりしない)。
#[test]
fn bare_string_without_dot_is_a_parse_error_not_self() {
    let result = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      fields:
        title: self
"#,
    );
    let err = match result {
        Ok(_) => panic!("expected a parse error for bare 'self' in combinator position"),
        Err(e) => e,
    };
    assert!(err.contains("alias.キー"), "{err}");
}

// --------------------------------------------------------------------------
// rows(table) + date コンビネータ(v0 extract_dated_table 相当)
// --------------------------------------------------------------------------

#[test]
fn rows_over_table_with_date_tokens() {
    let src = "\
::table {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=education}
@rows:
  - entry: [a, b]
@cols:
  - field:
    - date
    - text
@cells:
  a | date : 1997-03
  a | text : \"卒業\"
  b | date : 2000-04
  b | text : \"入学\"
::
";
    let view = r#"
version: 1
profiles: [submit]
files:
  education_history.yaml:
    content:
      rows:
        table: { alias: education }
        item:
          fields:
            年: { date: { of: { cell: { col: date } }, format: "YYYY" } }
            月: { date: { of: { cell: { col: date } }, format: "M" } }
            学歴: { pick: { of: { cell: { col: text } } } }
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(
        yaml,
        "- 年: '1997'\n  月: '3'\n  学歴: 卒業\n- 年: '2000'\n  月: '4'\n  学歴: 入学\n"
    );
}

#[test]
fn date_as_int_casts_numeric_string() {
    let src = "\
::table {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=licenses}
@rows:
  - entry: [a]
@cols:
  - field:
    - date
    - text
@cells:
  a | date : 2001-04
  a | text : \"普通自動車免許取得\"
::
";
    let view = r#"
version: 1
profiles: [submit]
files:
  licenses.yaml:
    content:
      rows:
        table: { alias: licenses }
        item:
          fields:
            year: { date: { of: { cell: { col: date } }, format: "YYYY", as: int } }
            month: { date: { of: { cell: { col: date } }, format: "M", as: int } }
            name: { pick: { of: { cell: { col: text } } } }
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "- year: 2001\n  month: 4\n  name: 普通自動車免許取得\n");
}

// --------------------------------------------------------------------------
// period の date コンビネータ(jp_period / iso_period 相当)
// --------------------------------------------------------------------------

#[test]
fn period_with_open_end_uses_period_open() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n在籍期間: 2020-01 〜 現在\n::\n";
    let view = r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      date:
        of: { record-field: { of: { class: dummy }, key: 在籍期間 } }
        format: "YYYY年M月"
        period-separator: "～"
        period-open: "現在"
"#;
    // class セレクタを使うため、record に class を付けたバージョンで組み直す。
    let src = format!("[class=dummy]\n{src}");
    let out = build(&src).expect("must build");
    let view = parse_view_def(view).unwrap();
    let (files, warnings) = apply(&out, &view, None).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(files[0].yaml, "2020年1月～現在\n");
}

// --------------------------------------------------------------------------
// age コンビネータ
// --------------------------------------------------------------------------

#[test]
fn age_computes_from_birth_and_as_of() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n生年月日: 1981-05-18\n作成日: 2026-06-23\n::\n";
    let view = r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      age:
        birth: { record-field: { of: { alias: basic-info }, key: 生年月日 } }
        as-of: { record-field: { of: { alias: basic-info }, key: 作成日 } }
        as: text
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "'45'\n");
}

#[test]
fn age_before_birthday_in_year_is_one_less() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n生年月日: 1981-05-18\n作成日: 2026-04-01\n::\n";
    let view = r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      age:
        birth: { record-field: { of: { alias: basic-info }, key: 生年月日 } }
        as-of: { record-field: { of: { alias: basic-info }, key: 作成日 } }
        as: int
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "44\n");
}

// --------------------------------------------------------------------------
// literal コンビネータ
// --------------------------------------------------------------------------

#[test]
fn literal_combinator_passes_through_fixed_value() {
    let out = build("").unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      fields:
        environments: { literal: "Linux, Windows, macOS" }
        tech_skills: { literal: [] }
"#,
    )
    .unwrap();
    let (files, _) = apply(&out, &view, None).unwrap();
    assert_eq!(files[0].yaml, "environments: Linux, Windows, macOS\ntech_skills: []\n");
}

// --------------------------------------------------------------------------
// join: ネストリストのアウトライン化(v0 flatten_summary_list 相当)
// --------------------------------------------------------------------------

#[test]
fn join_flattens_nested_list_with_prefix_and_skips_note_class() {
    let src = "\
# proj {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=proj}

[id=01ARZ3NDEKTSV4RRFFQ69G5FA1]
概要の段落。

[id=01ARZ3NDEKTSV4RRFFQ69G5FA2]
- 第1項目 {#01ARZ3NDEKTSV4RRFFQ69G5FA3}
  - 内訳A {#01ARZ3NDEKTSV4RRFFQ69G5FA4}
  - 内訳B {#01ARZ3NDEKTSV4RRFFQ69G5FA5}

[id=01ARZ3NDEKTSV4RRFFQ69G5FA6, class=note]
補足メモ。
";
    let view = r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      join:
        of: { alias: proj }
        separator: "\n"
        nested-prefix: "・"
        exclude-class: note
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "|-\n  概要の段落。\n  第1項目\n  ・内訳A\n  ・内訳B\n");
}

#[test]
fn join_include_only_class_extracts_notes() {
    let src = "\
# proj {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=proj}

[id=01ARZ3NDEKTSV4RRFFQ69G5FA1]
概要の段落。

[id=01ARZ3NDEKTSV4RRFFQ69G5FA6, class=note]
補足メモ。
";
    let view = r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      join:
        of: { alias: proj }
        separator: "\n\n"
        include-only-class: note
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "補足メモ。\n");
}

#[test]
fn join_over_record_keys_skips_empty_values() {
    let src =
        "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=other-info}\n健康状態: 良好\n趣味: \n特技: 特になし\n::\n";
    let view = r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      join:
        of: { alias: other-info }
        separator: "\n\n"
        keys: [健康状態, 趣味, 特技]
"#;
    let yaml = yaml_of(src, view);
    assert_eq!(yaml, "|-\n  健康状態: 良好\n\n  特技: 特になし\n");
}

// --------------------------------------------------------------------------
// rows(contains) + extend-path + 別表への cell cross-reference
// (v0 extract_companies の3方向 join 相当。会社セクション→H4子セクション→
//  project-index の行を alias 接尾辞で突き合わせる)
// --------------------------------------------------------------------------

#[test]
fn rows_contains_extend_path_joins_into_another_table_by_alias_suffix() {
    // v0 の extract_companies() 相当: 会社の rows(table) が row_path=[company_key]
    // を確立し、その中の rows(contains) が H4 子セクション自身の alias から
    // "proj-" を剥がした接尾辞を row_path に継ぎ足して、別表(project-index)の
    // セルを cross-reference する。
    let src = "\
::table {#01ARZ3NDEKTSV4RRFFQ69G5FA0 alias=company-overview}
@rows:
  - company:
    - acme
@cols:
  - info:
    - dummy
@cells:
  acme | dummy : \"x\"
::

::table {#01ARZ3NDEKTSV4RRFFQ69G5FA3 alias=project-index}
@rows:
  - company:
    - acme:
      - project:
        - widget \"widget\"
@cols:
  - attr:
    - period
    - role
@cells:
  acme.widget | period : 2020-01 ~ 2020-04
  acme.widget | role   : \"開発\"
::

# 会社 {#01ARZ3NDEKTSV4RRFFQ69G5FA1 alias=co-acme}

## Widget projekt {#01ARZ3NDEKTSV4RRFFQ69G5FA2 alias=proj-widget}
";
    let view = r#"
version: 1
profiles: [submit]
files:
  companies.yaml:
    content:
      rows:
        table: { alias: company-overview }
        item:
          rows:
            contains: { alias-from-row: { prefix: "co-", segment: 0 } }
            type: section
            extend-path: { alias-suffix: { prefix: "proj-" } }
            item:
              fields:
                name: { pick: { of: self } }
                role: { pick: { of: { cell: { of: { alias: project-index }, col: role } } } }
                period:
                  date:
                    of: { cell: { of: { alias: project-index }, col: period } }
                    format: "YYYY-MM"
                    period-separator: " ~ "
                    period-open: "現在"
"#;
    let out = build(src).expect("must build");
    let view = parse_view_def(view).unwrap();
    let (files, warnings) = apply(&out, &view, None).unwrap();
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(files[0].yaml, "- - name: Widget projekt\n    role: 開発\n    period: 2020-01 ~ 2020-04\n");
}

// --------------------------------------------------------------------------
// class セレクタ / heading-text セレクタ(Warning 付き)
// --------------------------------------------------------------------------

#[test]
fn class_selector_finds_unique_node() {
    let src = "[id=01ARZ3NDEKTSV4RRFFQ69G5FAV, class=marker]\n本文\n";
    let out = build(src).unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      pick:
        of: { class: marker }
"#,
    )
    .unwrap();
    let (files, warnings) = apply(&out, &view, None).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(files[0].yaml, "本文\n");
}

#[test]
fn heading_text_selector_matches_and_emits_warning() {
    let src = "# タイトル {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n";
    let out = build(src).unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  x.yaml:
    content:
      pick:
        of: { heading-text: "タイトル" }
"#,
    )
    .unwrap();
    let (files, warnings) = apply(&out, &view, None).unwrap();
    assert_eq!(files[0].yaml, "タイトル\n");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("heading-text"));
}

// --------------------------------------------------------------------------
// profile 分岐(D34)
// --------------------------------------------------------------------------

#[test]
fn profile_filters_files_and_default_emits_all() {
    let out = build("").unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit, check]
files:
  common.yaml:
    content: { literal: 1 }
  submit-only.yaml:
    profiles: [submit]
    content: { literal: 2 }
  check-only.yaml:
    profiles: [check]
    content: { literal: 3 }
"#,
    )
    .unwrap();

    let (all, _) = apply(&out, &view, None).unwrap();
    assert_eq!(all.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), vec!["check-only.yaml", "common.yaml", "submit-only.yaml"]);

    let (submit, _) = apply(&out, &view, Some("submit")).unwrap();
    assert_eq!(submit.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), vec!["common.yaml", "submit-only.yaml"]);

    let (chk, _) = apply(&out, &view, Some("check")).unwrap();
    assert_eq!(chk.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), vec!["check-only.yaml", "common.yaml"]);
}

#[test]
fn unknown_profile_is_rejected() {
    let out = build("").unwrap();
    let view = parse_view_def("version: 1\nprofiles: [submit]\nfiles: {}\n").unwrap();
    let err = apply(&out, &view, Some("nope")).unwrap_err();
    assert!(err.contains("nope"));
}

// --------------------------------------------------------------------------
// 決定性(WP-W1 要件: 同一入力・同一定義 → バイト同一出力)
// --------------------------------------------------------------------------

#[test]
fn apply_is_deterministic_across_runs() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n姓: 山田\n名: 太郎\n::\n";
    let view_src = r#"
version: 1
profiles: [submit]
files:
  profile.yaml:
    content:
      fields:
        姓: { pick: { of: { record-field: { of: { alias: basic-info }, key: 姓 } } } }
        名: { pick: { of: { record-field: { of: { alias: basic-info }, key: 名 } } } }
"#;
    let out = build(src).unwrap();
    let view = parse_view_def(view_src).unwrap();
    let (a, _) = apply(&out, &view, None).unwrap();
    let (b, _) = apply(&out, &view, None).unwrap();
    assert_eq!(a, b);
}

// --------------------------------------------------------------------------
// --check(D33): 未充足スロット・未使用ノード
// --------------------------------------------------------------------------

#[test]
fn check_reports_zero_diagnostics_when_manifest_and_view_agree() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n姓: 山田\n::\n";
    let out = build(src).unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  profile.yaml:
    content:
      fields:
        姓: { pick: { of: { record-field: { of: { alias: basic-info }, key: 姓 } } } }
"#,
    )
    .unwrap();
    let manifest = parse_manifest(
        "version: 1\nfiles:\n  profile.yaml:\n    shape: fields\n    fields: [姓]\n",
    )
    .unwrap();
    let report = check(&out, &view, &manifest);
    assert!(report.is_clean(), "{report:#?}");
}

/// negative テスト: マニフェストが要求するフィールドをビュー定義側でわざと
/// 外すと、「未充足スロット」診断が出ること。
#[test]
fn check_reports_missing_slot_when_view_omits_a_required_field() {
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n姓: 山田\n名: 太郎\n::\n";
    let out = build(src).unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  profile.yaml:
    content:
      fields:
        姓: { pick: { of: { record-field: { of: { alias: basic-info }, key: 姓 } } } }
"#,
    )
    .unwrap();
    // マニフェストは「名」も要求するが、ビュー定義には無い(わざと外した)。
    let manifest = parse_manifest(
        "version: 1\nfiles:\n  profile.yaml:\n    shape: fields\n    fields: [姓, 名]\n",
    )
    .unwrap();
    let report = check(&out, &view, &manifest);
    assert!(!report.is_clean());
    assert!(report.missing_slots.iter().any(|m| m.contains('名')), "{report:#?}");
}

#[test]
fn check_reports_unused_node_never_touched_by_any_selector() {
    // 「未使用ノード」は alias 付きの Table/Record/List だけを対象にする(粗い
    // 定義、裁量。check.rs 参照)。ここでは alias 付き record を2つ用意し、
    // 片方だけをビュー定義から参照する。
    let src = "::record {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=basic-info}\n姓: 山田\n::\n\n::record {#01ARZ3NDEKTSV4RRFFQ69G5FA1 alias=other-info}\n趣味: 読書\n::\n";
    let out = build(src).unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  profile.yaml:
    content:
      fields:
        姓: { pick: { of: { record-field: { of: { alias: basic-info }, key: 姓 } } } }
"#,
    )
    .unwrap();
    let manifest = parse_manifest(
        "version: 1\nfiles:\n  profile.yaml:\n    shape: fields\n    fields: [姓]\n",
    )
    .unwrap();
    let report = check(&out, &view, &manifest);
    assert!(!report.unused_nodes.is_empty(), "{report:#?}");
    assert!(report.missing_slots.is_empty(), "{report:#?}");
}
