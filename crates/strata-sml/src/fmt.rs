//! `strata fmt` コア — ID逆注入フォーマッタ(sml-spec.md §8、docs/sml-fmt-m2-handoff.md D-F1〜D-F3)。
//!
//! **スパンパッチ方式**(sml-spec D6): 元テキストは一切再シリアライズしない。
//! パース結果(`crate::parse`)を読み取り専用で走査して `Patch` の列を計画し
//! (`plan_patches`)、それをオフセット降順に元バイト列へ適用する(`apply_patches`)。
//! 計画と適用を分離しているのは、契約テスト(D-F5)がパッチ列そのものを検証する
//! 必要があるため。
//!
//! fmt が行う編集は5パターン(D-F3。sml-spec §8.1 の3分類をケース漏れなく落とし込んだもの):
//!
//! 1. 行型ブロック(見出し・リスト項目・フェンスマーカー)に `{#...}` が無い →
//!    行内容の末尾(行末空白の手前)に ` {#ULID}` を挿入
//! 2. 段落に前置属性行が無い → 段落先頭行の直前に `[id=ULID]\n` を挿入
//! 3. 段落の前置属性行に `id` キーが無い → `[` の直後に `id=ULID, ` を挿入
//! 4. `{#label}` のように id が非ULIDラベル → `inner_span` を `ULID alias=label` に置換
//! 5. `[id=label, ...]` のように id が非ULIDラベル → 値トークンを `ULID, alias=label` に置換
//!
//! 既に ULID を持つ(alias 有無問わず)場合は何もしない。コードフェンスには一切
//! 注入しない(sml-spec §10 保留)。参照(`(cell:...)` 等)は絶対に書き換えない —
//! そもそもこのモジュールは参照(インライン)を走査すらしない。

use ulid::Ulid;

use crate::ast::{AttrLine, AttrValue, BlockKind, IdTag, ListItem, RefTarget, SmlBlock, SmlDocument};
use crate::error::Diag;
use crate::span::Span;

/// 1件の編集。`at` は元テキスト基準のバイトオフセット。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    /// 挿入/置換の開始バイトオフセット(元テキスト基準)。
    pub at: usize,
    /// 削除バイト数(挿入のみなら 0)。
    pub delete: usize,
    pub insert: String,
}

/// fmt の結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmtOutput {
    /// パッチ適用後の全文。
    pub text: String,
    /// 監査・テスト用。オフセット降順(実際に適用された順)。
    pub patches: Vec<Patch>,
}

/// `src` をフォーマットする。`idgen` が返す ID の発行順は呼び出し側の責任
/// (`format` は文書順に単調な `ulid::Generator` を使う。D-F2)。
///
/// `src` のパースに1件でも診断があれば、ファイル内容には一切触れず `Err` を返す
/// (全か無か、sml-spec §8.2)。
pub fn format_with(src: &str, idgen: &mut dyn FnMut() -> Ulid) -> Result<FmtOutput, Vec<Diag>> {
    let parsed = crate::parse(src);
    if !parsed.diags.is_empty() {
        return Err(parsed.diags);
    }
    let planned = plan_patches(src, &parsed.doc, idgen);
    let (text, patches) = apply_patches(src, planned);
    Ok(FmtOutput { text, patches })
}

/// 本番エントリポイント。`ulid::Generator`(単調増加)を使い、文書順に ID を発行する。
pub fn format(src: &str) -> Result<FmtOutput, Vec<Diag>> {
    let mut generator = ulid::Generator::new();
    let mut idgen = move || {
        generator.generate().expect(
            "ulid::Generator: 同一ミリ秒内に発行可能な乱数ビットを使い果たした(実運用では起こり得ない)",
        )
    };
    format_with(src, &mut idgen)
}

// ---- パッチ計画(パース結果の走査のみ。バイト列は変更しない) --------------------------

/// ドキュメントを文書順(ブロック出現順、リスト内は項目順)に走査し、`Patch` の列を
/// 計画する。ULID の発行順もこの走査順に一致する(D-F2)。
fn plan_patches(src: &str, doc: &SmlDocument, idgen: &mut dyn FnMut() -> Ulid) -> Vec<Patch> {
    let mut patches = Vec::new();
    for block in &doc.blocks {
        plan_block(src, block, idgen, &mut patches);
    }
    patches
}

fn plan_block(src: &str, block: &SmlBlock, idgen: &mut dyn FnMut() -> Ulid, patches: &mut Vec<Patch>) {
    match &block.kind {
        BlockKind::Heading { id_tag, .. } => plan_line_type_id(src, block, id_tag, idgen, patches),
        BlockKind::Fence(fb) => plan_line_type_id(src, block, &fb.id_tag, idgen, patches),
        BlockKind::Paragraph { .. } => plan_prose_id(src, block, idgen, patches),
        BlockKind::List { items, .. } => {
            for item in items {
                plan_list_item_id(src, item, idgen, patches);
            }
        }
        // コードフェンスには一切注入しない(sml-spec §10 保留、D-F3 ケース7)。
        BlockKind::CodeFence { .. } => {}
    }
}

/// 行型ブロック(見出し・フェンスマーカー)の ID タグを計画する。
/// リスト項目は自身の行スパンを直接持つため `plan_list_item_id` が別途扱う。
fn plan_line_type_id(
    src: &str,
    block: &SmlBlock,
    id_tag: &Option<IdTag>,
    idgen: &mut dyn FnMut() -> Ulid,
    patches: &mut Vec<Patch>,
) {
    match id_tag {
        None => {
            let (line_start, content_end) = own_line_bounds(src, block);
            let at = trim_trailing_ws(src, line_start, content_end);
            let ulid = idgen();
            patches.push(Patch { at, delete: 0, insert: format!(" {{#{ulid}}}") });
        }
        Some(tag) => plan_id_tag_relabel(tag, idgen, patches),
    }
}

fn plan_list_item_id(src: &str, item: &ListItem, idgen: &mut dyn FnMut() -> Ulid, patches: &mut Vec<Patch>) {
    match &item.id_tag {
        None => {
            let at = trim_trailing_ws(src, item.span.start, item.span.end);
            let ulid = idgen();
            patches.push(Patch { at, delete: 0, insert: format!(" {{#{ulid}}}") });
        }
        Some(tag) => plan_id_tag_relabel(tag, idgen, patches),
    }
}

/// ケース4: `{#label}` の `inner_span` を `ULID alias=label` に置換する。
/// 既に ULID なら(alias 有無問わず)何もしない。
///
/// `{#label alias=x}`(非 ULID + 既存 alias)はパーサが `AliasWithoutUlid` で弾く
/// (2026-07-13 裁定)ため、ここに来る Label は常に alias 無し。既存 alias を
/// 静かに破棄する経路は存在しない。
fn plan_id_tag_relabel(tag: &IdTag, idgen: &mut dyn FnMut() -> Ulid, patches: &mut Vec<Patch>) {
    if let RefTarget::Label(label) = &tag.id {
        let ulid = idgen();
        patches.push(Patch {
            at: tag.inner_span.start,
            delete: tag.inner_span.len(),
            insert: format!("{ulid} alias={label}"),
        });
    }
}

/// 段落の ID を計画する(ケース2・3・5・6)。
fn plan_prose_id(src: &str, block: &SmlBlock, idgen: &mut dyn FnMut() -> Ulid, patches: &mut Vec<Patch>) {
    match &block.attrs {
        None => {
            // ケース2: 前置属性行が無い → 先頭行の直前に `[id=ULID]\n` を挿入。
            let ulid = idgen();
            patches.push(Patch { at: block.span.start, delete: 0, insert: format!("[id={ulid}]\n") });
        }
        Some(attrs) => match find_id_entry(attrs) {
            None => {
                // ケース3: 属性行はあるが id キーが無い → `[` の直後に挿入。
                let bracket = open_bracket_pos(src, attrs.span.start);
                let ulid = idgen();
                let insert = if attrs.entries.is_empty() {
                    format!("id={ulid}")
                } else {
                    format!("id={ulid}, ")
                };
                patches.push(Patch { at: bracket + 1, delete: 0, insert });
            }
            Some((value, entry_span)) => {
                // ケース5/6: id キーはある。値が非ULIDラベルなら置換、ULIDなら何もしない。
                if let AttrValue::Single(label) = value
                    && label.parse::<Ulid>().is_err()
                {
                    let (value_start, value_len) = locate_single_value_token(src, *entry_span, label);
                    let ulid = idgen();
                    patches.push(Patch {
                        at: value_start,
                        delete: value_len,
                        insert: format!("{ulid}, alias={label}"),
                    });
                }
                // AttrValue::Quoted / List の id はパーサが BadIdValue で弾く
                // (2026-07-13 裁定)ため、ここには到達しない。防御的に何もしない。
            }
        },
    }
}

fn find_id_entry(attrs: &AttrLine) -> Option<(&AttrValue, &Span)> {
    attrs.entries.iter().find(|(k, _, _)| k == "id").map(|(_, v, s)| (v, s))
}

// ---- バイトオフセット計算ヘルパ ------------------------------------------------------
//
// 全てASCIIバイト(改行・空白・タブ・`[`)の探索に限定しているため、UTF-8の文字境界を
// 破壊する心配はない(非ASCIIバイトは 0x80 以上であり、これらの値と衝突しない)。

/// 見出し/フェンスマーカーの「自身の行」の境界を求める。
///
/// 戻り値は `(行の開始オフセット, 行内容の終端オフセット(改行・CRを含まない))`。
/// `block.attrs` が束縛されている場合、ブロックの前置属性行の次の行が自身の行になる
/// (`SmlBlock.span` はフェンスの場合ブロック全体〈閉じ `::` 行まで〉を指すため、
/// ここでは `block.span.end` に頼らず、行頭から自前で改行を探す)。
fn own_line_bounds(src: &str, block: &SmlBlock) -> (usize, usize) {
    let bytes = src.as_bytes();
    let start = match &block.attrs {
        Some(attrs) => {
            let mut i = attrs.span.end;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            if i < bytes.len() { i + 1 } else { bytes.len() }
        }
        None => block.span.start,
    };
    let mut end = start;
    while end < bytes.len() && bytes[end] != b'\n' {
        end += 1;
    }
    let mut content_end = end;
    if content_end > start && bytes[content_end - 1] == b'\r' {
        content_end -= 1;
    }
    (start, content_end)
}

/// `[lo, hi)` の末尾から半角空白/タブを取り除いた境界を返す(挿入位置の計算に使う)。
fn trim_trailing_ws(src: &str, lo: usize, hi: usize) -> usize {
    let bytes = src.as_bytes();
    let mut b = hi;
    while b > lo && matches!(bytes[b - 1], b' ' | b'\t') {
        b -= 1;
    }
    b
}

/// 属性行の先頭にある `[` の絶対位置を求める(`AttrLine.span` は行全体で、
/// 前後の空白を含みうる)。
fn open_bracket_pos(src: &str, from: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = from;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t') {
        i += 1;
    }
    i
}

/// 属性エントリ(`key=value` 全体のスパン)から値トークンの絶対開始位置を求める。
/// `value_str` は既に空白を trim 済みの値文字列(id は裸トークンなので長さがそのまま
/// バイト長に一致する)。
fn locate_single_value_token(src: &str, entry_span: Span, value_str: &str) -> (usize, usize) {
    let text = entry_span.slice(src);
    let eq_idx = text.find('=').expect("id エントリは key=value 形式のはず(パーサ不変条件)");
    let bytes = src.as_bytes();
    let mut vs = entry_span.start + eq_idx + 1;
    while vs < entry_span.end && matches!(bytes[vs], b' ' | b'\t') {
        vs += 1;
    }
    (vs, value_str.len())
}

// ---- パッチ適用(バイト列操作のみ。AST は見ない) ------------------------------------

/// パッチをオフセット降順に並べ替え、その順で元テキストへ適用する。
/// 戻り値は `(適用後の全文, 適用に使った順のパッチ列)`。
fn apply_patches(src: &str, mut patches: Vec<Patch>) -> (String, Vec<Patch>) {
    patches.sort_by_key(|p| std::cmp::Reverse(p.at));
    let mut buf = src.to_string();
    for p in &patches {
        let start = p.at;
        let end = p.at + p.delete;
        buf.replace_range(start..end, &p.insert);
    }
    (buf, patches)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 決定的な連番 ULID 生成器(テスト用)。`base` から1ずつインクリメントした
    /// タイムスタンプ・固定乱数部で Ulid を作る。生成される文字列は互いに文書順で
    /// 単調増加する。
    fn seq_idgen(mut next: u128) -> impl FnMut() -> Ulid {
        move || {
            let ulid = Ulid(next);
            next += 1;
            ulid
        }
    }

    fn fmt_ok(src: &str) -> FmtOutput {
        let mut idgen = seq_idgen(0x0001_8000_0000_0000_0000_0000_0000_0000);
        format_with(src, &mut idgen).unwrap_or_else(|d| panic!("expected Ok, got diags: {d:?}"))
    }

    // ---- ケース1: 行型ブロックに ID タグ無し -----------------------------------

    #[test]
    fn heading_without_id_gets_trailing_tag() {
        let out = fmt_ok("# Title\n");
        assert!(out.text.starts_with("# Title {#"));
        assert!(out.text.ends_with("}\n"));
        assert_eq!(out.patches.len(), 1);
        assert_eq!(out.patches[0].delete, 0);
    }

    #[test]
    fn heading_without_id_and_trailing_whitespace_inserts_before_whitespace() {
        let src = "# Title  \n";
        let out = fmt_ok(src);
        // 挿入位置は「行内容の末尾」= 行末空白の手前。
        assert!(out.text.starts_with("# Title {#"), "{}", out.text);
        assert!(out.text.contains("}  \n"), "{}", out.text);
    }

    #[test]
    fn list_item_without_id_gets_trailing_tag() {
        let out = fmt_ok("- one\n- two\n");
        let lines: Vec<&str> = out.text.lines().collect();
        assert!(lines[0].starts_with("- one {#") && lines[0].ends_with('}'));
        assert!(lines[1].starts_with("- two {#") && lines[1].ends_with('}'));
        assert_eq!(out.patches.len(), 2);
    }

    #[test]
    fn list_item_with_full_width_dash_content_char_boundary_safe() {
        // 全角ダッシュ(—)を含む行での挿入位置計算が char boundary を破壊しないこと。
        let src = "- 項目テキスト — 補足\n";
        let out = fmt_ok(src);
        assert!(out.text.contains("補足 {#"), "{}", out.text);
    }

    #[test]
    fn fence_marker_without_id_gets_trailing_tag() {
        let out = fmt_ok("::math {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\nx\n::\n");
        // 上記は id_tag あり(no-op)。無しのケースを別途確認する。
        assert!(out.patches.is_empty());

        let out2 = fmt_ok("::math\nx\n::\n");
        assert!(out2.text.starts_with("::math {#"));
        assert_eq!(out2.patches.len(), 1);
    }

    #[test]
    fn table_marker_without_id_gets_trailing_tag() {
        let out = fmt_ok("::table\n@rows:\n  - a: [b]\n::\n");
        assert!(out.text.starts_with("::table {#"));
    }

    #[test]
    fn figure_marker_without_id_gets_trailing_tag() {
        let out = fmt_ok("::figure\n[kind=chart]\n::\n");
        assert!(out.text.starts_with("::figure {#"));
    }

    // ---- ケース2: 段落に前置属性行が無い ----------------------------------------

    #[test]
    fn paragraph_without_attrs_gets_id_line_inserted() {
        let out = fmt_ok("Hello world.\n");
        assert!(out.text.starts_with("[id="), "{}", out.text);
        assert!(out.text.contains("]\nHello world.\n"), "{}", out.text);
    }

    // ---- ケース3: 段落の前置属性行に id キーが無い -------------------------------

    #[test]
    fn paragraph_attrs_without_id_key_gets_id_prepended() {
        let out = fmt_ok("[supports=x]\nParagraph.\n");
        assert!(out.text.starts_with("[id="), "{}", out.text);
        assert!(out.text.contains(", supports=x]"), "{}", out.text);
    }

    // ---- ケース4: IdTag の id が非ULIDラベル -------------------------------------

    #[test]
    fn heading_label_id_tag_gets_relabeled_with_alias() {
        let out = fmt_ok("# Title {#my-label}\n");
        assert!(out.text.contains("alias=my-label"), "{}", out.text);
        assert!(!out.text.contains("{#my-label}"), "{}", out.text);
    }

    #[test]
    fn fence_label_id_tag_gets_relabeled_with_alias() {
        let out = fmt_ok("::table {#eval-table}\n@rows:\n  - a: [b]\n::\n");
        assert!(out.text.contains("alias=eval-table"), "{}", out.text);
    }

    #[test]
    fn list_item_label_id_tag_gets_relabeled_with_alias() {
        let out = fmt_ok("- one {#item-one}\n");
        assert!(out.text.contains("alias=item-one"), "{}", out.text);
    }

    // ---- ケース5: 属性行の id が非ULIDラベル -------------------------------------

    #[test]
    fn attr_line_label_id_gets_relabeled_with_alias() {
        let out = fmt_ok("[id=key-finding, supports=x]\nParagraph.\n");
        assert!(out.text.starts_with("[id="), "{}", out.text);
        assert!(out.text.contains(", alias=key-finding, supports=x]"), "{}", out.text);
        assert!(!out.text.contains("id=key-finding"), "{}", out.text);
    }

    // ---- ケース6: 既に ULID を持つ → 無変更 --------------------------------------

    #[test]
    fn heading_with_ulid_tag_is_untouched() {
        let src = "# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n";
        let out = fmt_ok(src);
        assert_eq!(out.text, src);
        assert!(out.patches.is_empty());
    }

    #[test]
    fn heading_with_ulid_and_alias_is_untouched() {
        let src = "# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=foo}\n";
        let out = fmt_ok(src);
        assert_eq!(out.text, src);
        assert!(out.patches.is_empty());
    }

    #[test]
    fn attr_line_with_ulid_id_is_untouched() {
        let src = "[id=01ARZ3NDEKTSV4RRFFQ69G5FAV, supports=x]\nParagraph.\n";
        let out = fmt_ok(src);
        assert_eq!(out.text, src);
        assert!(out.patches.is_empty());
    }

    #[test]
    fn fully_formatted_document_is_a_no_op() {
        let out = fmt_ok(
            "# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n\n[id=01ARZ3NDEKTSV4RRFFQ69G5FAW]\nA paragraph.\n",
        );
        assert!(out.patches.is_empty());
    }

    // ---- ケース7: コードフェンスには一切注入しない -------------------------------

    #[test]
    fn code_fence_is_never_touched() {
        let src = "```rust\nfn main() {}\n```\n";
        let out = fmt_ok(src);
        assert_eq!(out.text, src);
        assert!(out.patches.is_empty());
    }

    // ---- パースエラー時は Err(diags と一致) --------------------------------------

    #[test]
    fn parse_error_yields_err_matching_parse_diags() {
        let src = "[id=foo]\n# Title {#bar}\n"; // DuplicateId
        let mut idgen = seq_idgen(0);
        let result = format_with(src, &mut idgen);
        let parsed_diags = crate::parse(src).diags;
        assert!(!parsed_diags.is_empty());
        match result {
            Err(diags) => assert_eq!(diags, parsed_diags),
            Ok(_) => panic!("expected Err"),
        }
    }

    #[test]
    fn parse_error_leaves_conceptually_untouched_since_no_patches_are_planned() {
        // 「全か無か」: Err の場合、そもそも patch 計画・適用は行われない
        // (format_with が parse 直後に早期 return する)。
        let src = "[id=foo]\n\nOrphan.\n"; // OrphanAttrLine
        let mut idgen = seq_idgen(0);
        assert!(format_with(src, &mut idgen).is_err());
    }

    // ---- パッチ降順適用の正しさ ---------------------------------------------------

    #[test]
    fn patches_apply_in_descending_offset_order_are_consistent_with_output() {
        let src = "# A\n\n- b\n- c\n\n[supports=x]\nD.\n";
        let out = fmt_ok(src);
        // 出力パッチ列がオフセット降順であること。
        for w in out.patches.windows(2) {
            assert!(w[0].at >= w[1].at, "patches not in descending order: {:?}", out.patches);
        }
        // 同じパッチ列を独立に元テキストへ再適用しても同じ結果になること。
        let mut buf = src.to_string();
        for p in &out.patches {
            let end = p.at + p.delete;
            buf.replace_range(p.at..end, &p.insert);
        }
        assert_eq!(buf, out.text);
    }

    // ---- 決定的 idgen による固定出力 ----------------------------------------------

    #[test]
    fn deterministic_idgen_produces_stable_ulid_sequence() {
        let fixed: Vec<Ulid> =
            ["01ARZ3NDEKTSV4RRFFQ69G5FAV", "01ARZ3NDEKTSV4RRFFQ69G5FAW", "01ARZ3NDEKTSV4RRFFQ69G5FAX"]
                .iter()
                .map(|s| s.parse().unwrap())
                .collect();
        let mut it = fixed.clone().into_iter();
        let mut idgen = move || it.next().expect("test provides exactly enough ulids");
        let out = format_with("# A\n\n- b\n", &mut idgen).unwrap();
        assert_eq!(out.text, format!("# A {{#{}}}\n\n- b {{#{}}}\n", fixed[0], fixed[1]));
    }
}
