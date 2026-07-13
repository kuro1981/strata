//! WP-F2: fmt の契約テスト(docs/sml-fmt-m2-handoff.md D-F5、sml-spec.md §8.1)。
//!
//! D-F5 の4契約を機械化する:
//!
//! 1. **ゴールデン完全一致**: 固定 ULID 16個を文書順に注入した `format_with(draft)` が
//!    formatted fixture とバイト完全一致
//! 2. **冪等性**: `format(formatted)` のパッチが0件。任意入力でも
//!    `format(format(x).text).patches.is_empty()`
//! 3. **挿入のみ**: 全パッチが「delete == 0」または「削除範囲が元テキストの
//!    `{...}` / `[...]` の内側に収まる置換」。判定は fmt の内部実装を一切使わず、
//!    **元テキストを独立にパースし直した結果のスパン**(IdTag.inner_span /
//!    AttrLine の括弧内側)と Patch 列だけから行う(トートロジー回避)
//! 4. **意味保存**: `parse(format(x).text)` と `parse(x)` が ID 無視で同型
//!    (`tests/common/mod.rs` の正規化関数で比較)
//!
//! 追加プロパティ(ハンドオフ WP-F2 指定):
//! - fmt 後のテキストを再パースして diags がゼロ
//! - fmt 後の全ブロックが ULID の ID を持つ(コードフェンスを除く。リストは全項目、
//!   段落は属性行の id、見出し・フェンスは IdTag)

mod common;

use common::{norm_doc, read_doc};
use strata_sml::{format, format_with, AttrValue, BlockKind, RefTarget, SmlDocument};
use ulid::Ulid;

// ---- ヘルパ --------------------------------------------------------------------

/// draft fixture の16ブロックに文書順で発行される固定 ULID 列
/// (`01J2T8Z0000000000000000000` 〜 `01J2T8ZF000000000000000000`。
/// 末尾16文字目が Crockford Base32 の 0-9,A-F。handoff D-F5 参照)。
fn golden_idgen() -> impl FnMut() -> Ulid {
    let ulids: Vec<Ulid> = "0123456789ABCDEF"
        .chars()
        .map(|c| format!("01J2T8Z{c}000000000000000000").parse().expect("valid crockford ulid"))
        .collect();
    let mut it = ulids.into_iter();
    move || it.next().expect("golden fixture needs exactly 16 ulids")
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

/// ブロック前置属性行の `alias` エントリを正規化結果から取り除く。
///
/// `tests/common/mod.rs` の正規化(WP5 由来)は属性行の `id` エントリだけを無視するが、
/// fmt はケース5(`[id=label, ...]` → `[id=ULID, alias=label, ...]`)で属性行に
/// `alias` エントリを**新規追加**する。alias は ID 情報の一部(sml-spec §3、D3)なので、
/// 契約4の「ID 無視で同型」の比較ではここでローカルに取り除く(共通の正規化関数は
/// golden_isomorphism.rs の検証を弱めないため変更しない)。
fn strip_attr_alias(blocks: &mut [common::NBlock]) {
    for b in blocks {
        b.attrs.retain(|e| e.key != "alias");
    }
}

/// `parse(format(x).text)` と `parse(x)` が ID 無視で同型であることを検証する。
fn assert_semantics_preserved(src: &str) {
    let before = strata_sml::parse(src);
    assert!(before.diags.is_empty(), "precondition: source must parse cleanly: {:?}", before.diags);

    let out = fmt_prod(src);
    let after = strata_sml::parse(&out.text);
    assert!(after.diags.is_empty(), "formatted text must parse cleanly: {:?}", after.diags);

    let mut before_norm = norm_doc(src, &before.doc);
    let mut after_norm = norm_doc(&out.text, &after.doc);
    strip_attr_alias(&mut before_norm);
    strip_attr_alias(&mut after_norm);

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

// ---- 追加プロパティ: fmt 後は diags ゼロ & 全ブロックが ULID を持つ -----------------

/// fmt 後のドキュメントで、コードフェンスを除く全ブロックが ULID の ID を持つことを
/// 検証する。リストは全項目、段落は属性行の `id` エントリ、見出し・フェンスは IdTag。
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
            BlockKind::List { items, .. } => {
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
            BlockKind::Paragraph { .. } => {
                let attrs = block
                    .attrs
                    .as_ref()
                    .unwrap_or_else(|| panic!("paragraph #{i} has no attr line after fmt"));
                let (_, value, _) = attrs
                    .entries
                    .iter()
                    .find(|(k, _, _)| k == "id")
                    .unwrap_or_else(|| panic!("paragraph #{i} attr line has no id entry after fmt"));
                match value {
                    AttrValue::Single(v) => assert!(
                        v.parse::<Ulid>().is_ok(),
                        "paragraph #{i} id is not a ULID after fmt: {v:?}"
                    ),
                    other => panic!("paragraph #{i} id is not a bare token after fmt: {other:?}"),
                }
            }
            // コードフェンスは ID を持たない(sml-spec §10 保留)。
            BlockKind::CodeFence { .. } => {}
        }
    }
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
