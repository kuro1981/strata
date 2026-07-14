//! WP-F1: fmt コアの外形テスト(docs/sml-fmt-m2-handoff.md、docs/sml-build-m3-handoff.md WP-B2)。
//!
//! 最重要ケース: `docs/sml_example_draft.sml` に固定 ULID 列を注入した結果が
//! `docs/sml_example_formatted.sml` とバイト完全一致すること(簡易ゴールデン)。
//! fixture の読み込みは `tests/golden_isomorphism.rs` の `read_doc` を踏襲する。

use strata_sml::{format_with, DiagKind};

fn read_doc(rel: &str) -> String {
    let path = format!("{}/../../docs/{}", env!("CARGO_MANIFEST_DIR"), rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

/// draft の18ブロック(フロントマター生成込み)に文書順で発行される固定 ULID 列
/// (`01J2T8Z0000000000000000000` 〜 `01J2T8ZH000000000000000000`、
/// 末尾17文字目が Crockford Base32 の `0-9,A-H`。WP-B2 fixture 改版参照)。
fn golden_idgen() -> impl FnMut() -> ulid::Ulid {
    let ulids: Vec<ulid::Ulid> = "0123456789ABCDEFGH"
        .chars()
        .map(|c| format!("01J2T8Z{c}000000000000000000").parse().expect("valid crockford ulid"))
        .collect();
    let mut it = ulids.into_iter();
    move || it.next().expect("golden fixture needs exactly 18 ulids")
}

#[test]
fn golden_draft_formats_to_golden_formatted_byte_exact() {
    let draft = read_doc("sml_example_draft.sml");
    let expected = read_doc("sml_example_formatted.sml");

    let mut idgen = golden_idgen();
    let out = format_with(&draft, &mut idgen).unwrap_or_else(|d| panic!("expected Ok, got diags: {d:?}"));

    assert_eq!(out.text, expected, "formatted output does not byte-match the golden fixture");
}

#[test]
fn golden_formatted_is_idempotent_zero_patches() {
    let formatted = read_doc("sml_example_formatted.sml");
    let mut generator = ulid::Generator::new();
    let mut idgen = move || generator.generate().unwrap();
    let out = format_with(&formatted, &mut idgen).unwrap_or_else(|d| panic!("expected Ok, got diags: {d:?}"));
    assert!(out.patches.is_empty(), "formatted fixture should already be fully ID-tagged: {:?}", out.patches);
    assert_eq!(out.text, formatted);
}

#[test]
fn parse_error_input_yields_err_with_matching_diags() {
    // DuplicateId: 行型ブロックの {#...} と属性行の id= が併記されている。
    let src = "[id=foo]\n# Title {#bar}\n";
    let expected_diags = strata_sml::parse(src).diags;
    assert!(!expected_diags.is_empty());

    let mut idgen = golden_idgen();
    match format_with(src, &mut idgen) {
        Err(diags) => assert_eq!(diags, expected_diags),
        Ok(out) => panic!("expected Err on parse error, got Ok: {out:?}"),
    }
}

#[test]
fn parse_error_input_is_not_mutated_conceptually() {
    // OrphanAttrLine. 「全か無か」契約: Err のときファイルには一切触れない
    // (format_with は parse 直後に早期 return し、パッチ計画すら行わない)。
    let src = "[id=foo]\n\nOrphan paragraph.\n";
    let mut idgen = golden_idgen();
    let result = format_with(src, &mut idgen);
    assert!(result.is_err());
    if let Err(diags) = result {
        assert!(diags.iter().any(|d| d.kind == DiagKind::OrphanAttrLine));
    }
}

#[test]
fn all_five_case_kinds_appear_across_block_kinds_in_golden_draft() {
    // ゴールデン draft は D-F3 のケース1〜5(+ D-B4 のケース6: フロントマター生成)を
    // すべて少なくとも一度含む(見出し/リスト項目/フェンスマーカーの無ID、段落・
    // リスト全体の属性行無し・id無し、非ULIDラベルの {#...} と [id=...] の両方、
    // フロントマター無し)。ここでは patch 件数が期待通り18件であることだけを
    // 機械的に確認する(内容の正しさは golden_draft_formats_to_golden_formatted_byte_exact
    // がバイト単位で保証する)。
    let draft = read_doc("sml_example_draft.sml");
    let mut idgen = golden_idgen();
    let out = format_with(&draft, &mut idgen).unwrap();
    assert_eq!(out.patches.len(), 18, "{:#?}", out.patches);
}

#[test]
fn patches_are_insert_only_or_bracket_confined_replacements() {
    // 契約3(挿入のみ): 全パッチが delete==0 であるか、`{...}`/`[...]` の内側に
    // 収まる置換であること。ここでは「delete>0 の削除範囲そのものに区切り文字
    // `{`/`}`/`[`/`]` が含まれない(=区切りは消さず内側だけを書き換えている)」
    // ことを機械的に確認する。
    let draft = read_doc("sml_example_draft.sml");
    let mut idgen = golden_idgen();
    let out = format_with(&draft, &mut idgen).unwrap();
    let src_bytes = draft.as_bytes();
    for p in &out.patches {
        if p.delete > 0 {
            let deleted = &src_bytes[p.at..p.at + p.delete];
            assert!(
                !deleted.iter().any(|b| matches!(b, b'{' | b'}' | b'[' | b']')),
                "replacement at {} deletes a delimiter byte: {:?}",
                p.at,
                std::str::from_utf8(deleted)
            );
        }
    }
}
