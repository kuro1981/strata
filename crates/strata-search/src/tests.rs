use super::*;

/// ドラフト SML(ラベルのみ、ID 未付与)を `strata fmt` → `strata build` の順で通した
/// `BuildOutput` を返す。他クレートの CLI 統合テストと同じ「fmt してから使う」流儀
/// (ULID を手で捏造しない)。パイプラインのどちらかが診断を出したら即座に panic する
/// (テストフィクスチャ自体が壊れていることの早期検出)。
fn built(draft: &str) -> BuildOutput {
    let formatted = strata_sml::format(draft).expect("fmt must succeed on draft fixture").text;
    strata_build::build(&formatted).expect("build must succeed on formatted fixture")
}

const TEST_DOC: &str = r#"---
title: 検索テスト文書
alias: searchdoc
---

# 検索テスト文書

[class=note]
## 【補足】将来の自分へ {#note-section}

- 最初のメモです {#note-item-1}
- 二番目のメモです {#note-item-2}

## 本編 {#main-section}

[id=proj-alpha-status]
プロジェクトαの進捗は順調である。[アジャイル](term:アジャイル)開発を採用した。

[id=proj-beta-status]
プロジェクトβはウォーターフォール型で進めている。

## プロジェクト概要 {#project-overview}

このセクションでは全体の方針を説明する。

::table {#summary-table}
[caption="プロジェクト進捗一覧"]
@rows:
  - project: [alpha, beta]
@cols:
  - status: [ratio]
@cells:
  alpha | ratio : 80
  beta  | ratio : 40
::
"#;

fn test_index() -> InMemoryIndex {
    let build = built(TEST_DOC);
    InMemoryIndex::from_build(&build, DocRef { path: "search-test.sml".to_string(), alias: Some("searchdoc".to_string()) })
}

fn aliases(hits: &[Hit]) -> Vec<Option<String>> {
    hits.iter().map(|h| h.alias.clone()).collect()
}

// --- 素のテキスト条件(CJK 部分一致) ------------------------------------------------

#[test]
fn plain_text_term_matches_cjk_substring_in_body() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("ウォーターフォール"), 10);
    assert_eq!(aliases(&hits), vec![Some("proj-beta-status".to_string())]);
}

#[test]
fn plain_text_term_matches_partial_word_not_just_whole_token() {
    let idx = test_index();
    // 「プロジェクト」は複合語の一部にも部分文字列として現れる(CJK は分かち書きが無い)。
    let hits = idx.search(&Query::parse("プロジェクト"), 20);
    let found_aliases: Vec<Option<String>> = aliases(&hits);
    assert!(found_aliases.contains(&Some("proj-alpha-status".to_string())));
    assert!(found_aliases.contains(&Some("proj-beta-status".to_string())));
    assert!(found_aliases.contains(&Some("project-overview".to_string())));
}

// --- ランキング: 見出し一致 > 本文一致(裁量) ----------------------------------------

#[test]
fn heading_match_ranks_above_body_only_match() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("プロジェクト"), 20);
    assert_eq!(hits[0].node_type, "section", "見出し一致の project-overview が先頭に来るはず: {hits:?}");
    assert_eq!(hits[0].alias.as_deref(), Some("project-overview"));
}

// --- 構造述語: class:(D46 実効 class = 自身+祖先) ------------------------------------

#[test]
fn class_predicate_matches_via_effective_class_inheritance() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("class:note"), 20);
    let found = aliases(&hits);
    assert!(found.contains(&Some("note-section".to_string())), "{found:?}");
    assert!(found.contains(&Some("note-item-1".to_string())), "継承: {found:?}");
    assert!(found.contains(&Some("note-item-2".to_string())), "継承: {found:?}");
    // note class を持たないブロックは含まれない。
    assert!(!found.contains(&Some("proj-alpha-status".to_string())));
}

#[test]
fn class_predicate_is_exact_tag_match_not_substring() {
    let idx = test_index();
    // "not" は "note" の部分文字列だが、class は完全一致のはず。
    let hits = idx.search(&Query::parse("class:not"), 20);
    assert!(hits.is_empty(), "{hits:?}");
}

// --- 構造述語: alias:<接頭辞> ------------------------------------------------------

#[test]
fn alias_prefix_predicate_matches_prefix_only() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("alias:proj-"), 20);
    let found = aliases(&hits);
    assert!(found.contains(&Some("proj-alpha-status".to_string())));
    assert!(found.contains(&Some("proj-beta-status".to_string())));
    assert!(!found.contains(&Some("project-overview".to_string())), "接頭辞は 'proj-' であり 'project-' は含まないはず: {found:?}");
}

// --- 構造述語: term:<用語> (Term ノード自体 + term-ref 使用ブロックの両方) ------------

#[test]
fn term_predicate_matches_both_the_term_node_and_blocks_using_it() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("term:アジャイル"), 20);
    let types: Vec<&str> = hits.iter().map(|h| h.node_type.as_str()).collect();
    assert!(types.contains(&"term"), "用語ノード自身がヒットするはず: {types:?}");
    assert!(hits.iter().any(|h| h.alias.as_deref() == Some("proj-alpha-status")), "用語を使用したブロックもヒットするはず: {hits:?}");
}

// --- 述語+テキストの AND 混在 -------------------------------------------------------

#[test]
fn mixed_predicate_and_text_terms_are_anded() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("alias:proj- ウォーターフォール"), 20);
    assert_eq!(aliases(&hits), vec![Some("proj-beta-status".to_string())]);
}

// --- スニペット ---------------------------------------------------------------------

#[test]
fn snippet_marks_the_matched_span_and_keeps_surrounding_context() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("ウォーターフォール"), 10);
    let snip = &hits[0].snippet;
    assert_eq!(snip.matched, "ウォーターフォール");
    assert!(snip.before.contains("プロジェクトβは"), "{snip:?}");
    assert!(snip.after.contains("型で進めている"), "{snip:?}");
}

// --- 空クエリ ------------------------------------------------------------------------

#[test]
fn empty_query_returns_no_hits() {
    let idx = test_index();
    assert!(idx.search(&Query::parse(""), 10).is_empty());
    assert!(idx.search(&Query::parse("   "), 10).is_empty());
}

// --- クイックスイッチャー(文書・見出し・alias に限定) --------------------------------

#[test]
fn switcher_matches_section_heading_by_prefix() {
    let idx = test_index();
    let hits = idx.switcher("本編", 10);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node_type, "section");
    assert_eq!(hits[0].alias.as_deref(), Some("main-section"));
}

#[test]
fn switcher_matches_any_node_type_by_alias() {
    let idx = test_index();
    // note-item-1 は para だが alias を持つ(fmt がラベルを alias に昇格させた)。
    let hits = idx.switcher("note-item-1", 10);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].node_type, "para");
    assert_eq!(hits[0].alias.as_deref(), Some("note-item-1"));
}

#[test]
fn switcher_excludes_alias_less_body_paragraphs() {
    let idx = test_index();
    // 「方針」は project-overview 配下の alias 無し段落の本文にのみ出現する。
    // 全文検索なら拾うが、スイッチャーは文書・見出し・alias 限定なので拾わない。
    let full_text_hits = idx.search(&Query::parse("方針"), 10);
    assert!(!full_text_hits.is_empty(), "前提: 全文検索では見つかるはず");
    let switcher_hits = idx.switcher("方針", 10);
    assert!(switcher_hits.is_empty(), "{switcher_hits:?}");
}

#[test]
fn switcher_ranks_exact_alias_and_prefix_above_plain_substring() {
    let idx = test_index();
    let hits = idx.switcher("proj", 10);
    // 両方とも alias 部分一致(前方一致でもある): "proj-alpha-status"/"proj-beta-status"。
    assert!(hits.len() >= 2);
    assert!(hits[0].score >= hits[1].score);
}

#[test]
fn switcher_empty_query_returns_nothing() {
    let idx = test_index();
    assert!(idx.switcher("", 10).is_empty());
}

// --- ワークスペース: 所属文書の解決 --------------------------------------------------

#[test]
fn workspace_index_resolves_doc_ref_alias_and_path_per_member() {
    let a_formatted = strata_sml::format(
        "---\ntitle: 文書A\nalias: doca\n---\n\n# 文書A\n\n内容Aの本文テキスト。\n",
    )
    .unwrap()
    .text;
    let b_formatted = strata_sml::format(
        "---\ntitle: 文書B\n---\n\n# 文書B\n\n内容Bの本文テキスト。\n",
    )
    .unwrap()
    .text;

    let members = vec![
        strata_build::Member { path: "a.sml".to_string(), src: a_formatted },
        strata_build::Member { path: "b.sml".to_string(), src: b_formatted },
    ];
    let ws = strata_build::build_workspace(&members).expect("workspace build must succeed");
    let idx = InMemoryIndex::from_workspace(&ws);

    let hits_a = idx.search(&Query::parse("内容Aの本文"), 10);
    assert_eq!(hits_a.len(), 1);
    let doc_a = hits_a[0].doc.as_ref().expect("doc must be resolved");
    assert_eq!(doc_a.path, "a.sml");
    assert_eq!(doc_a.alias.as_deref(), Some("doca"), "文書 alias が無ければ path のみ");
    assert_eq!(doc_a.display(), "doca");

    let hits_b = idx.search(&Query::parse("内容Bの本文"), 10);
    assert_eq!(hits_b.len(), 1);
    let doc_b = hits_b[0].doc.as_ref().expect("doc must be resolved");
    assert_eq!(doc_b.path, "b.sml");
    assert_eq!(doc_b.alias, None, "文書 alias 未宣言のメンバーは path 表示にフォールバック");
    assert_eq!(doc_b.display(), "b.sml");
}

// --- JSON ラウンドトリップ(--json / エディタ・AI 向け構造化出力) ---------------------

#[test]
fn hit_roundtrips_through_json() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("ウォーターフォール"), 1);
    assert_eq!(hits.len(), 1);
    let json = serde_json::to_string_pretty(&hits[0]).unwrap();
    // エディタ/AI が拾うべき主要フィールドがフラットな JSON に出ていること。
    assert!(json.contains("\"node_id\""));
    assert!(json.contains("\"label\""));
    assert!(json.contains("\"snippet\""));
    assert!(json.contains("\"doc\""));
    let back: Hit = serde_json::from_str(&json).unwrap();
    assert_eq!(back, hits[0]);
}

#[test]
fn switcher_hit_roundtrips_through_json() {
    let idx = test_index();
    let hits = idx.switcher("本編", 1);
    assert_eq!(hits.len(), 1);
    let json = serde_json::to_string(&hits[0]).unwrap();
    let back: SwitcherHit = serde_json::from_str(&json).unwrap();
    assert_eq!(back, hits[0]);
}

// --- 表(Table)の caption・セル値も検索対象になること ---------------------------------

#[test]
fn table_caption_and_cell_text_are_searchable() {
    let idx = test_index();
    let hits = idx.search(&Query::parse("進捗一覧"), 10);
    assert!(hits.iter().any(|h| h.alias.as_deref() == Some("summary-table")), "{hits:?}");
}
