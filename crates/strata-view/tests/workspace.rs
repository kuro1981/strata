//! M7(WP-W3、D43): view の複数文書入力・セレクタの doc スコープ。
//!
//! `strata_build::build_workspace` で作った統合グラフに対して、
//! `apply_workspace`/`check_workspace` と `{ alias, doc }` セレクタ・
//! `doc/alias.key` 糖衣が正しく解決できることを検証する。

use strata_build::{build_workspace, Member};
use strata_view::{apply_workspace, check_workspace, parse_manifest, parse_view_def};

fn member(path: &str, src: &str) -> Member {
    Member { path: path.to_string(), src: src.to_string() }
}

/// resume.sml(alias=resume, basic-info record)+ work_history.sml(alias=work-history)
/// の2文書ワークスペース。WP-W4 のドッグフーディング(実害2)を模した最小構成。
fn two_doc_workspace() -> strata_build::WorkspaceBuildOutput {
    let resume_src = "\
---
id: 01ARZ3NDEKTSV4RRFFQ69G5FA1
alias: resume
---

::record {#01ARZ3NDEKTSV4RRFFQ69G5FA2 alias=basic-info}
姓: 山田
名: 太郎
::
";
    let wh_src = "\
---
id: 01ARZ3NDEKTSV4RRFFQ69G5FA3
alias: work-history
---

# 概要 {#01ARZ3NDEKTSV4RRFFQ69G5FA4 alias=summary}
";
    build_workspace(&[member("resume.sml", resume_src), member("work_history.sml", wh_src)])
        .expect("workspace must build")
}

#[test]
fn doc_scoped_alias_selector_resolves_across_documents() {
    let ws = two_doc_workspace();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  out.yaml:
    content:
      fields:
        姓:
          pick:
            of:
              record-field:
                of: { alias: basic-info, doc: resume }
                key: 姓
"#,
    )
    .unwrap();
    let (files, warnings) = apply_workspace(&ws, &view, None).expect("apply_workspace must succeed");
    assert!(warnings.is_empty(), "{warnings:?}");
    assert_eq!(files[0].yaml, "姓: 山田\n");
}

#[test]
fn doc_scoped_sugar_resolves_across_documents() {
    let ws = two_doc_workspace();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  out.yaml:
    content:
      fields:
        姓: resume/basic-info.姓
"#,
    )
    .unwrap();
    let (files, _) = apply_workspace(&ws, &view, None).expect("apply_workspace must succeed");
    assert_eq!(files[0].yaml, "姓: 山田\n");
}

/// D42: 無修飾 alias はワークスペース全体で一意なら解決できる(単一文書モードと
/// 同じ「1文書内で一意」という前提が、他文書に同名が無い限り自然に拡張される)。
#[test]
fn unqualified_alias_resolves_when_unique_across_workspace() {
    let ws = two_doc_workspace();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  out.yaml:
    content:
      fields:
        姓: basic-info.姓
"#,
    )
    .unwrap();
    let (files, _) = apply_workspace(&ws, &view, None).expect("apply_workspace must succeed");
    assert_eq!(files[0].yaml, "姓: 山田\n");
}

/// 2文書が同じブロック alias を持つ場合、無修飾では曖昧エラーになる(黙って
/// どちらかを選ばない)。
#[test]
fn unqualified_alias_is_ambiguous_across_two_documents_with_same_alias() {
    let a_src = "\
---
id: 01ARZ3NDEKTSV4RRFFQ69G5FB1
alias: doc-a
---

::record {#01ARZ3NDEKTSV4RRFFQ69G5FB2 alias=shared}
k: a-value
::
";
    let b_src = "\
---
id: 01ARZ3NDEKTSV4RRFFQ69G5FB3
alias: doc-b
---

::record {#01ARZ3NDEKTSV4RRFFQ69G5FB4 alias=shared}
k: b-value
::
";
    let ws = build_workspace(&[member("a.sml", a_src), member("b.sml", b_src)]).expect("must build");
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  out.yaml:
    content:
      pick:
        of: { alias: shared }
"#,
    )
    .unwrap();
    let err = apply_workspace(&ws, &view, None).expect_err("ambiguous unqualified alias must error");
    assert!(err.contains("shared"), "{err}");
}

#[test]
fn unknown_doc_scope_reports_clear_error() {
    let ws = two_doc_workspace();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  out.yaml:
    content:
      pick:
        of: { alias: basic-info, doc: no-such-doc }
"#,
    )
    .unwrap();
    let err = apply_workspace(&ws, &view, None).expect_err("unknown doc must error");
    assert!(err.contains("no-such-doc"), "{err}");
}

/// 単一文書モードの `apply` に doc スコープ付きセレクタを渡すと(ワークスペース
/// モードではない旨の)明確なエラーになる。
#[test]
fn doc_scope_selector_in_single_file_mode_is_a_clear_error() {
    let out = strata_build::build("").unwrap();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
files:
  out.yaml:
    content:
      pick:
        of: { alias: x, doc: resume }
"#,
    )
    .unwrap();
    let err = strata_view::apply(&out, &view, None).expect_err("doc scope without workspace must error");
    assert!(err.contains("workspace") || err.contains("ワークスペース"), "{err}");
}

/// WP-W4 の実地バグ潰し: cv-jis.view.yaml が resume.sml から `doc: resume` で
/// 1フィールドだけ借りると、resume.sml 側の無関係な alias 付きノード(この
/// テストでは `other-record`)まで「未使用」判定に巻き込まれてはいけない
/// (`doc:` で明示的に借りたノードは、その文書「全体」を使ったことにしない)。
#[test]
fn check_workspace_does_not_flag_unrelated_nodes_in_a_doc_qualified_source_document() {
    let resume_src = "\
---
id: 01ARZ3NDEKTSV4RRFFQ69G5FC1
alias: resume
---

::record {#01ARZ3NDEKTSV4RRFFQ69G5FC2 alias=basic-info}
姓: 山田
::

::record {#01ARZ3NDEKTSV4RRFFQ69G5FC3 alias=other-record}
k: v
::
";
    let cv_src = "\
---
id: 01ARZ3NDEKTSV4RRFFQ69G5FC4
alias: work-history
---

# 概要 {#01ARZ3NDEKTSV4RRFFQ69G5FC5}
";
    let ws = build_workspace(&[member("resume.sml", resume_src), member("work_history.sml", cv_src)])
        .expect("must build");
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
manifest: manifest.yaml
files:
  out.yaml:
    content:
      fields:
        姓: resume/basic-info.姓
"#,
    )
    .unwrap();
    let manifest = parse_manifest(
        r#"
version: 1
files:
  out.yaml:
    shape: fields
    fields: [姓]
"#,
    )
    .unwrap();
    let report = check_workspace(&ws, &view, &manifest);
    assert!(report.is_clean(), "{:?}", report);
}

/// 上のテストと対になる確認: 「その文書は home document として実際に(無修飾で)
/// 使われている」場合は、従来どおり本当に未使用のノードを検出できること
/// (doc スコープ絞り込みが検出力そのものを失わせていないことの確認)。
#[test]
fn check_workspace_still_flags_genuinely_unused_node_in_an_unqualified_home_document() {
    let resume_src = "\
---
id: 01ARZ3NDEKTSV4RRFFQ69G5FD1
alias: resume
---

::record {#01ARZ3NDEKTSV4RRFFQ69G5FD2 alias=basic-info}
姓: 山田
::

::record {#01ARZ3NDEKTSV4RRFFQ69G5FD3 alias=unused-record}
k: v
::
";
    let ws = build_workspace(&[member("resume.sml", resume_src)]).expect("must build");
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
manifest: manifest.yaml
files:
  out.yaml:
    content:
      fields:
        姓: basic-info.姓
"#,
    )
    .unwrap();
    let manifest = parse_manifest(
        r#"
version: 1
files:
  out.yaml:
    shape: fields
    fields: [姓]
"#,
    )
    .unwrap();
    let report = check_workspace(&ws, &view, &manifest);
    assert!(
        report.unused_nodes.iter().any(|m| m.contains("unused-record")),
        "{:?}",
        report
    );
}

#[test]
fn check_workspace_reports_zero_diagnostics_when_manifest_and_view_agree() {
    let ws = two_doc_workspace();
    let view = parse_view_def(
        r#"
version: 1
profiles: [submit]
manifest: manifest.yaml
files:
  out.yaml:
    content:
      fields:
        姓: { pick: { of: { record-field: { of: { alias: basic-info, doc: resume }, key: 姓 } } } }
"#,
    )
    .unwrap();
    let manifest = parse_manifest(
        r#"
version: 1
files:
  out.yaml:
    shape: fields
    fields: [姓]
"#,
    )
    .unwrap();
    let report = check_workspace(&ws, &view, &manifest);
    assert!(report.is_clean(), "{:?}", report);
}
