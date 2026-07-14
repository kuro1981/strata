//! WP-F2: fmt の契約テスト(docs/sml-fmt-m2-handoff.md D-F5、sml-spec.md §8.1、
//! docs/sml-build-m3-handoff.md WP-B2)。
//!
//! D-F5 の4契約を機械化する:
//!
//! 1. **ゴールデン完全一致**: 固定 ULID 18個(フロントマター生成込み、D-B4)を
//!    文書順に注入した `format_with(draft)` が formatted fixture とバイト完全一致
//! 2. **冪等性**: `format(formatted)` のパッチが0件。任意入力でも
//!    `format(format(x).text).patches.is_empty()`
//! 3. **挿入のみ**: 全パッチが「delete == 0」または「削除範囲が元テキストの
//!    `{...}` / `[...]` の内側に収まる置換」。判定は fmt の内部実装を一切使わず、
//!    **元テキストを独立にパースし直した結果のスパン**(IdTag.inner_span /
//!    AttrLine の括弧内側)と Patch 列だけから行う(トートロジー回避)
//! 4. **意味保存**: `parse(format(x).text)` と `parse(x)` が ID 無視で同型
//!    (`tests/common/mod.rs` の正規化関数で比較)
//!
//! 追加プロパティ(ハンドオフ WP-F2 指定、WP-B2 で拡張):
//! - fmt 後のテキストを再パースして diags がゼロ
//! - fmt 後の**全ブロック**(コードフェンスも含む。D10/D-B4 で除外を廃止)が ULID の
//!   ID を持つ(リストは全項目 + リスト全体の前置属性行〈D11〉、段落は属性行の id、
//!   見出し・フェンス・コードフェンスは IdTag)
//! - fmt 後の文書は必ずフロントマターを持ち、その id が ULID である(D12/D-B4)

mod common;

use common::{norm_doc, read_doc};
use strata_sml::{format, format_with, AttrValue, BlockKind, RefTarget, SmlBlock, SmlDocument};
use ulid::Ulid;

// ---- ヘルパ --------------------------------------------------------------------

/// draft fixture の18ブロック(フロントマター生成込み)に文書順で発行される
/// 固定 ULID 列(`01J2T8Z0000000000000000000` 〜 `01J2T8ZH000000000000000000`。
/// 末尾17文字目が Crockford Base32 の 0-9,A-H。sml-build-m3-handoff.md WP-B2 参照)。
fn golden_idgen() -> impl FnMut() -> Ulid {
    let ulids: Vec<Ulid> = "0123456789ABCDEFGH"
        .chars()
        .map(|c| format!("01J2T8Z{c}000000000000000000").parse().expect("valid crockford ulid"))
        .collect();
    let mut it = ulids.into_iter();
    move || it.next().expect("golden fixture needs exactly 18 ulids")
}

/// 見出し / リスト / 段落 / フェンス / コードフェンス / 属性行を横断する自作サンプル。
/// 契約2(冪等性)・契約3(挿入のみ)・契約4(意味保存)・追加プロパティで共用する。
fn samples() -> Vec<&'static str> {
    vec![
        // 見出し+段落+リスト(ラベル付き項目を含む)
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- item one\n- item two {#label-two}\n",
        // ラベル付き見出し+属性行付き段落+mathフェンス+コードフェンス
        "## Sec {#sec-label}\n\n[supports=sec-label]\nA paragraph.\n\n::math {#loss}\nE = mc^2\n::\n\n```rust\nfn main() {}\n```\n",
        // tableフェンス(フェンス内属性行)+番号付きリスト+ラベルidの属性行付き段落
        "::table {#tbl}\n[caption=\"c\"]\n\n@rows:\n  - model: [A, B]\n@cols:\n  - metric: [X]\n@cells:\n  A | X : 1\n::\n\n1. first\n2. second\n\n[id=para-label, cites=tbl]\nCites the table.\n",
        // 複数行段落+非ASCII(全角ダッシュ)を含む見出し+figureフェンス
        "段落一行目\n二行目が続く。\n\n### 見出し 日本語 — テスト\n\n::figure\n[kind=chart, data-ref=tbl]\n[caption=\"図\"]\n::\n",
        // 既に ULID 済みの部分と未付与の部分が混在
        "# T {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n\n[id=01ARZ3NDEKTSV4RRFFQ69G5FAX]\nDone.\n\n- untagged item\n",
    ]
}

fn fmt_prod(src: &str) -> strata_sml::FmtOutput {
    format(src).unwrap_or_else(|d| panic!("format should succeed, got diags: {d:?}\ninput: {src:?}"))
}

// ---- 契約1: ゴールデン完全一致 ---------------------------------------------------

#[test]
fn contract1_golden_draft_formats_to_formatted_byte_exact() {
    let draft = read_doc("sml_example_draft.sml");
    let expected = read_doc("sml_example_formatted.sml");

    let mut idgen = golden_idgen();
    let out = format_with(&draft, &mut idgen).unwrap_or_else(|d| panic!("expected Ok, got diags: {d:?}"));

    assert_eq!(out.text, expected, "format_with(draft) does not byte-match the golden formatted fixture");
}

// ---- 契約2: 冪等性 ----------------------------------------------------------------

#[test]
fn contract2_formatted_fixture_yields_zero_patches() {
    let formatted = read_doc("sml_example_formatted.sml");
    let out = fmt_prod(&formatted);
    assert!(
        out.patches.is_empty(),
        "formatted fixture should be a fixpoint of fmt, got patches: {:?}",
        out.patches
    );
    assert_eq!(out.text, formatted);
}

#[test]
fn contract2_format_is_idempotent_on_arbitrary_inputs() {
    for src in samples() {
        let once = fmt_prod(src);
        let twice = fmt_prod(&once.text);
        assert!(
            twice.patches.is_empty(),
            "fmt(fmt(x)) produced patches for input {src:?}: {:?}",
            twice.patches
        );
        assert_eq!(twice.text, once.text, "second fmt changed text for input {src:?}");
    }
}

// ---- 契約3: 挿入のみ ---------------------------------------------------------------
//
// 検証ロジックは fmt の内部実装(plan_patches 等)を一切使わない。元テキストを
// strata_sml::parse で独立にパースし直し、「置換が許される範囲」= IdTag.inner_span
// (= `{#` と `}` の内側)と、ブロック前置属性行の `[` と `]` の内側、を列挙する。
// その上で Patch 列だけを見て、delete>0 のパッチの削除範囲が必ずどれかの許容範囲に
// 包含されることを確認する。さらに削除バイト自体に区切り文字が含まれないことも
// 元テキストのバイト列から直接検証する。

/// 元テキストのパース結果から「置換が許される範囲」(半開区間)を列挙する。
/// - 行型ブロック(見出し・フェンス・リスト項目)の `IdTag.inner_span`
/// - ブロック前置属性行の `[` と `]` の内側
///
/// フェンス内属性行・インライン参照は fmt の置換対象ではないため意図的に含めない
/// (含めると検証が緩くなる)。
fn allowed_replace_ranges(src: &str, doc: &SmlDocument) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    for block in &doc.blocks {
        if let Some(attrs) = &block.attrs {
            // AttrLine.span は行全体(前後の空白を含みうる)。`[` と `]` の内側を求める。
            let text = attrs.span.slice(src);
            let leading_ws = text.len() - text.trim_start().len();
            let trimmed = text.trim();
            assert!(
                trimmed.starts_with('[') && trimmed.ends_with(']'),
                "attr line span does not bracket: {text:?}"
            );
            let open = attrs.span.start + leading_ws;
            let close = open + trimmed.len() - 1;
            ranges.push((open + 1, close));
        }
        match &block.kind {
            BlockKind::Heading { id_tag: Some(tag), .. } => {
                ranges.push((tag.inner_span.start, tag.inner_span.end));
            }
            BlockKind::Fence(fb) => {
                if let Some(tag) = &fb.id_tag {
                    ranges.push((tag.inner_span.start, tag.inner_span.end));
                }
            }
            BlockKind::List { items, .. } => {
                for item in items {
                    if let Some(tag) = &item.id_tag {
                        ranges.push((tag.inner_span.start, tag.inner_span.end));
                    }
                }
            }
            _ => {}
        }
    }
    ranges
}

/// `src` に fmt を掛け、全パッチが「挿入のみ」契約を満たすことを検証する。
fn assert_patches_insert_only_or_bracket_confined(src: &str) {
    let parsed = strata_sml::parse(src);
    assert!(parsed.diags.is_empty(), "precondition: source must parse cleanly: {:?}", parsed.diags);
    let allowed = allowed_replace_ranges(src, &parsed.doc);

    let out = fmt_prod(src);
    let bytes = src.as_bytes();
    for p in &out.patches {
        if p.delete == 0 {
            continue; // 純粋な挿入。無条件に契約を満たす。
        }
        let (start, end) = (p.at, p.at + p.delete);
        assert!(end <= bytes.len(), "patch deletes past end of source: {p:?}");
        // (a) 削除範囲が `{...}` / `[...]` の内側スパンのどれかに包含されること。
        assert!(
            allowed.iter().any(|&(s, e)| s <= start && end <= e),
            "replacement at {start}..{end} is not confined to any {{...}}/[...] interior \
             (allowed: {allowed:?}, patch: {p:?}, input: {src:?})"
        );
        // (b) 削除バイト自体に区切り文字が含まれないこと(元テキストの直接検証)。
        let deleted = &bytes[start..end];
        assert!(
            !deleted.iter().any(|b| matches!(b, b'{' | b'}' | b'[' | b']')),
            "replacement deletes a delimiter byte at {start}..{end}: {:?}",
            std::str::from_utf8(deleted)
        );
    }
}

#[test]
fn contract3_golden_draft_patches_are_insert_only_or_bracket_confined() {
    let draft = read_doc("sml_example_draft.sml");
    assert_patches_insert_only_or_bracket_confined(&draft);
}

#[test]
fn contract3_arbitrary_input_patches_are_insert_only_or_bracket_confined() {
    for src in samples() {
        assert_patches_insert_only_or_bracket_confined(src);
    }
}

// ---- 契約4: 意味保存 ---------------------------------------------------------------

/// `parse(format(x).text)` と `parse(x)` が ID 無視で同型であることを検証する。
/// 「ID無視」の定義(属性行の `id`・`alias` エントリと id_tag 全体を無視)は
/// `tests/common/mod.rs` の正規化関数が持つ(sml-spec §8.1、2026-07-13 裁定)。
fn assert_semantics_preserved(src: &str) {
    let before = strata_sml::parse(src);
    assert!(before.diags.is_empty(), "precondition: source must parse cleanly: {:?}", before.diags);

    let out = fmt_prod(src);
    let after = strata_sml::parse(&out.text);
    assert!(after.diags.is_empty(), "formatted text must parse cleanly: {:?}", after.diags);

    let before_norm = norm_doc(src, &before.doc);
    let after_norm = norm_doc(&out.text, &after.doc);

    assert_eq!(
        before_norm.len(),
        after_norm.len(),
        "block count changed by fmt for input {src:?}"
    );
    for (i, (b, a)) in before_norm.iter().zip(after_norm.iter()).enumerate() {
        assert_eq!(b, a, "block #{i} differs before/after fmt (ID-agnostic comparison), input: {src:?}");
    }
}

#[test]
fn contract4_golden_draft_semantics_preserved_by_fmt() {
    let draft = read_doc("sml_example_draft.sml");
    assert_semantics_preserved(&draft);
}

#[test]
fn contract4_arbitrary_input_semantics_preserved_by_fmt() {
    for src in samples() {
        assert_semantics_preserved(src);
    }
}

// ---- 追加プロパティ: fmt 後は diags ゼロ & 全ブロック(+フロントマター)が ULID を持つ --

/// ブロック前置属性行(段落・リスト全体)の `id` エントリが ULID であることを検証する
/// (D11: リストも段落と同じ規則)。
fn assert_attr_line_id_is_ulid(block: &SmlBlock, i: usize, kind_label: &str) {
    let attrs =
        block.attrs.as_ref().unwrap_or_else(|| panic!("{kind_label} #{i} has no attr line after fmt"));
    let (_, value, _) = attrs
        .entries
        .iter()
        .find(|(k, _, _)| k == "id")
        .unwrap_or_else(|| panic!("{kind_label} #{i} attr line has no id entry after fmt"));
    match value {
        AttrValue::Single(v) => {
            assert!(v.parse::<Ulid>().is_ok(), "{kind_label} #{i} id is not a ULID after fmt: {v:?}")
        }
        other => panic!("{kind_label} #{i} id is not a bare token after fmt: {other:?}"),
    }
}

/// fmt 後のドキュメントで、**全ブロック**(コードフェンスも含む。D10/D-B4で除外を廃止)が
/// ULID の ID を持つことを検証する。リストは全項目に加えリスト全体の前置属性行 `id`
/// (D11)、段落は属性行の `id` エントリ、見出し・フェンス・コードフェンスは IdTag。
fn assert_all_blocks_have_ulid_ids(doc: &SmlDocument) {
    for (i, block) in doc.blocks.iter().enumerate() {
        match &block.kind {
            BlockKind::Heading { id_tag, .. } => {
                let tag = id_tag.as_ref().unwrap_or_else(|| panic!("heading #{i} has no id tag after fmt"));
                assert!(
                    matches!(tag.id, RefTarget::Ulid(_)),
                    "heading #{i} id is not a ULID after fmt: {:?}",
                    tag.id
                );
            }
            BlockKind::Fence(fb) => {
                let tag =
                    fb.id_tag.as_ref().unwrap_or_else(|| panic!("fence #{i} has no id tag after fmt"));
                assert!(
                    matches!(tag.id, RefTarget::Ulid(_)),
                    "fence #{i} id is not a ULID after fmt: {:?}",
                    tag.id
                );
            }
            // コードフェンス開始行も行型ブロック(D10、2026-07-14 改定)。
            BlockKind::CodeFence { id_tag, .. } => {
                let tag = id_tag
                    .as_ref()
                    .unwrap_or_else(|| panic!("code fence #{i} has no id tag after fmt"));
                assert!(
                    matches!(tag.id, RefTarget::Ulid(_)),
                    "code fence #{i} id is not a ULID after fmt: {:?}",
                    tag.id
                );
            }
            BlockKind::List { items, .. } => {
                // リスト全体の前置属性行(D11)。
                assert_attr_line_id_is_ulid(block, i, "list");
                for (j, item) in items.iter().enumerate() {
                    let tag = item
                        .id_tag
                        .as_ref()
                        .unwrap_or_else(|| panic!("list #{i} item #{j} has no id tag after fmt"));
                    assert!(
                        matches!(tag.id, RefTarget::Ulid(_)),
                        "list #{i} item #{j} id is not a ULID after fmt: {:?}",
                        tag.id
                    );
                }
            }
            BlockKind::Paragraph { .. } => assert_attr_line_id_is_ulid(block, i, "paragraph"),
        }
    }
}

/// fmt 後のドキュメントが必ずフロントマターを持ち、その `id` が ULID であることを
/// 検証する(D12/D-B4: fmt 済みファイルは常にフロントマターを持つ)。
fn assert_frontmatter_has_ulid_id(doc: &SmlDocument) {
    let fm = doc.frontmatter.as_ref().expect("document has no frontmatter after fmt");
    let (target, _) = fm.id.as_ref().expect("frontmatter has no id after fmt");
    assert!(matches!(target, RefTarget::Ulid(_)), "frontmatter id is not a ULID after fmt: {target:?}");
}

#[test]
fn property_formatted_output_reparses_with_zero_diags() {
    let mut inputs: Vec<String> = samples().iter().map(|s| s.to_string()).collect();
    inputs.push(read_doc("sml_example_draft.sml"));
    for src in &inputs {
        let out = fmt_prod(src);
        let reparsed = strata_sml::parse(&out.text);
        assert!(
            reparsed.diags.is_empty(),
            "formatted output has diags: {:?}\ninput: {src:?}\noutput: {:?}",
            reparsed.diags,
            out.text
        );
    }
}

#[test]
fn property_all_blocks_have_ulid_ids_after_fmt() {
    let mut inputs: Vec<String> = samples().iter().map(|s| s.to_string()).collect();
    inputs.push(read_doc("sml_example_draft.sml"));
    for src in &inputs {
        let out = fmt_prod(src);
        let reparsed = strata_sml::parse(&out.text);
        assert!(reparsed.diags.is_empty(), "{:?}", reparsed.diags);
        assert_all_blocks_have_ulid_ids(&reparsed.doc);
    }
}

#[test]
fn property_document_has_frontmatter_with_ulid_id_after_fmt() {
    let mut inputs: Vec<String> = samples().iter().map(|s| s.to_string()).collect();
    inputs.push(read_doc("sml_example_draft.sml"));
    for src in &inputs {
        let out = fmt_prod(src);
        let reparsed = strata_sml::parse(&out.text);
        assert!(reparsed.diags.is_empty(), "{:?}", reparsed.diags);
        assert_frontmatter_has_ulid_id(&reparsed.doc);
    }
}
