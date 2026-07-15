//! `strata fmt` コア — ID逆注入フォーマッタ(sml-spec.md §8、docs/sml-build-m3-handoff.md D-B4)。
//!
//! **スパンパッチ方式**(sml-spec D6): 元テキストは一切再シリアライズしない。
//! パース結果(`crate::parse`)を読み取り専用で走査して `Patch` の列を計画し
//! (`plan_patches`)、それをオフセット降順に元バイト列へ適用する(`apply_patches`)。
//! 計画と適用を分離しているのは、契約テスト(D-F5)がパッチ列そのものを検証する
//! 必要があるため。
//!
//! fmt が行う編集は6パターン(sml-spec §8.1 の4分類をケース漏れなく落とし込んだもの。
//! D10〜D12 でコードフェンス・リスト全体・フロントマターへ対象拡張、2026-07-14):
//!
//! 1. 行型ブロック(見出し・リスト項目・フェンスマーカー・コードフェンス開始行)に
//!    `{#...}` が無い → 行内容の末尾(行末空白の手前)に ` {#ULID}` を挿入
//! 2. 段落・リスト全体に前置属性行が無い → 先頭行の直前に `[id=ULID]\n` を挿入
//! 3. 段落・リスト全体の前置属性行に `id` キーが無い → `[` の直後に `id=ULID, ` を挿入
//! 4. `{#label}` のように id が非ULIDラベル → `inner_span` を `ULID alias=label` に置換
//! 5. `[id=label, ...]` のように id が非ULIDラベル → 値トークンを `ULID, alias=label` に置換
//! 6. フロントマターが無ければオフセット0に `---\nid: ULID\n---\n\n` を生成・挿入。
//!    あって `id` が無ければ開き `---` 行の直後に `id: ULID\n` を挿入
//!
//! 既に ULID を持つ(alias 有無問わず)場合は何もしない。参照(`(cell:...)` 等)は
//! 絶対に書き換えない — そもそもこのモジュールは参照(インライン)を走査すらしない。
//!
//! **ID 発行順**(D-B4): フロントマターが常に最初、以降は文書順(ブロック出現順、
//! リストは「リスト全体」→「各項目」の順)。

use ulid::Ulid;

use crate::ast::{AttrLine, AttrValue, BlockKind, Frontmatter, IdTag, ListItem, RefTarget, SmlBlock, SmlDocument};
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
    /// パース診断のうち `Warning` のもの(D17)。「全か無か」は `Error` にのみ適用する
    /// ため、`Warning` だけの入力でも fmt は成功し、ここに詰めて呼び出し側へ返す。
    pub warnings: Vec<Diag>,
}

/// `src` をフォーマットする。`idgen` が返す ID の発行順は呼び出し側の責任
/// (`format` は文書順に単調な `ulid::Generator` を使う。D-F2)。
///
/// `src` のパースに1件でも `Error` 診断があれば、ファイル内容には一切触れず `Err` で
/// 全診断(`Error`・`Warning` 双方)を返す(全か無か、sml-spec §8.2。D17で対象を
/// `Error` のみに絞った)。`Warning` のみなら成功し、`FmtOutput::warnings` に詰める。
pub fn format_with(src: &str, idgen: &mut dyn FnMut() -> Ulid) -> Result<FmtOutput, Vec<Diag>> {
    let parsed = crate::parse(src);
    if parsed.diags.iter().any(Diag::is_error) {
        return Err(parsed.diags);
    }
    let warnings = parsed.diags;
    let planned = plan_patches(src, &parsed.doc, idgen);
    let (text, patches) = apply_patches(src, planned);
    Ok(FmtOutput { text, patches, warnings })
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

/// ドキュメントを文書順(フロントマターが最初、以降はブロック出現順・リスト内は
/// 「リスト全体」→ 項目順)に走査し、`Patch` の列を計画する。ULID の発行順もこの
/// 走査順に一致する(D-F2、D-B4)。
fn plan_patches(src: &str, doc: &SmlDocument, idgen: &mut dyn FnMut() -> Ulid) -> Vec<Patch> {
    let mut patches = Vec::new();
    // ID はフロントマターが最初に発行される(D-B4)が、そのパッチ自体は末尾に
    // push する。フロントマター無し文書では先頭ブロックの挿入位置もオフセット0に
    // なりうる(ケース2/3)ため、`apply_patches` の同オフセット挿入の重なりで
    // フロントマターが常に手前(バッファ上でより先)に来るようにする必要がある
    // (`apply_patches` は同オフセットの挿入を後から処理した方が先頭に来る仕組み
    // なので、フロントマターは最後に処理されなければならない)。
    let frontmatter_patch = plan_frontmatter_id(src, &doc.frontmatter, idgen);
    for block in &doc.blocks {
        plan_block(src, block, idgen, &mut patches);
    }
    if let Some(p) = frontmatter_patch {
        patches.push(p);
    }
    patches
}

/// ケース6: フロントマターの ID を計画する(sml-spec §2.1・§8.1、D-B4)。ID の発行
/// (`idgen` 呼び出し)はここで文書順の最初に行われるが、返した `Patch` を
/// `patches` 列のどこに置くか(=適用順)は呼び出し側(`plan_patches`)の責任。
///
/// - フロントマターが無ければオフセット0に生成一式を挿入する
/// - あって `id` が無ければ開き `---` 行の直後に `id: ULID\n` を挿入する
/// - `id` があれば何もしない(`format_with` は診断ゼロの入力にしか到達しないため、
///   ここに来る `id` は必ず ULID — フロントマターの非ULID id は `BadIdValue` で
///   全か無かにより弾かれている)
fn plan_frontmatter_id(
    src: &str,
    frontmatter: &Option<Frontmatter>,
    idgen: &mut dyn FnMut() -> Ulid,
) -> Option<Patch> {
    match frontmatter {
        None => {
            let ulid = idgen();
            Some(Patch { at: 0, delete: 0, insert: format!("---\nid: {ulid}\n---\n\n") })
        }
        Some(fm) if fm.id.is_none() => {
            let ulid = idgen();
            let at = line_end_including_newline(src, fm.open_span.end);
            Some(Patch { at, delete: 0, insert: format!("id: {ulid}\n") })
        }
        Some(_) => None,
    }
}

fn plan_block(src: &str, block: &SmlBlock, idgen: &mut dyn FnMut() -> Ulid, patches: &mut Vec<Patch>) {
    plan_block_at(src, block, idgen, patches, 0);
}

/// `quote_depth` は blockquote のネスト深さ(M6 D40)。深さ d のブロックの直前に
/// 挿入する `[id=ULID]` 行には `"> "` × d のマーカー前置が必要(でないと挿入行が
/// 引用の外に出て、引用が分断される)。
fn plan_block_at(
    src: &str,
    block: &SmlBlock,
    idgen: &mut dyn FnMut() -> Ulid,
    patches: &mut Vec<Patch>,
    quote_depth: usize,
) {
    match &block.kind {
        BlockKind::Heading { id_tag, .. } => plan_heading_id(src, block, id_tag, idgen, patches),
        BlockKind::Fence(fb) => plan_line_type_id(src, block, &fb.id_tag, idgen, patches),
        // コードフェンス開始行も行型ブロック(D10、2026-07-14 改定)。
        BlockKind::CodeFence { id_tag, .. } => plan_line_type_id(src, block, id_tag, idgen, patches),
        BlockKind::Paragraph { .. } => plan_prose_id(src, block, idgen, patches, quote_depth),
        BlockKind::List { items, .. } => {
            // リスト全体の ID は前置属性行(プローズ扱い、D11)→ 各項目の順(D-B4)。
            // D24(2026-07-14 裁定): ネスト項目にも同じ規則で ID を注入する。子リストは
            // 親項目の行の直後に現れるため、文書順は「親項目 → その子リストの各項目
            // (再帰) → 次の兄弟項目」になる。
            plan_prose_id(src, block, idgen, patches, quote_depth);
            plan_list_items_ids(src, items, idgen, patches);
        }
        // M6(D40): blockquote・GFM 表もプローズ扱い(前置属性行、裁量・最終報告参照)。
        // blockquote は中身のブロックにも再帰的に ID を注入する。
        //
        // パッチ順序の注意: 引用自身の `[id=]` 挿入と、引用先頭行の内側ブロックの
        // `> [id=]` 挿入は**同じオフセット**(引用の1行目の行頭)になる。
        // `apply_patches` は同一オフセットでは「後に適用されたパッチほどバッファ上で
        // 先(上)に来る」ため、引用自身のパッチを内側のパッチより**後ろ**に置く
        // (plan_patches のフロントマターと同じ理屈)。ID の発行順自体は文書順
        // (引用 → 内側)を維持する。
        BlockKind::Quote { blocks } => {
            let mut own = Vec::new();
            plan_prose_id(src, block, idgen, &mut own, quote_depth);
            for inner in blocks {
                plan_block_at(src, inner, idgen, patches, quote_depth + 1);
            }
            patches.extend(own);
        }
        BlockKind::GfmTable(_) => plan_prose_id(src, block, idgen, patches, quote_depth),
        // M6(D40): 参照リンク定義行は非可視メタ(ノードを作らない)なので ID を持たない。
        // 水平線は build が決定的に ID を自動導出するため、fmt は関与しない(裁量、
        // 最終報告参照)。
        BlockKind::LinkRefDef { .. } | BlockKind::ThematicBreak => {}
    }
}

/// 見出しの ID タグを計画する(M6 D40: Setext 見出し対応)。
///
/// ATX(`#` 始まり)は従来どおり自身の行末へ。**Setext 見出し**(own line が `#` で
/// 始まらない)は、パーサが ID タグを「下線行の直前のテキスト行末尾」から抽出する
/// (block.rs の `RawKind::SetextHeading` 処理)ため、挿入位置もそこに合わせる —
/// でないと fmt が冪等でなくなる(裁量: fmt の ID 注入はテキスト最終行の行末
/// ` {#ULID}`。最終報告参照)。
fn plan_heading_id(
    src: &str,
    block: &SmlBlock,
    id_tag: &Option<IdTag>,
    idgen: &mut dyn FnMut() -> Ulid,
    patches: &mut Vec<Patch>,
) {
    match id_tag {
        None => {
            let (line_start, content_end) = own_line_bounds(src, block);
            // blockquote 内の見出し(M6 D40)は行頭に `> ` マーカーが付くため、
            // マーカーと空白を読み飛ばしてから ATX(`#`)かどうかを判定する。
            let stripped = src[line_start..content_end].trim_start_matches([' ', '>']);
            let is_setext = !stripped.starts_with('#');
            let at = if is_setext {
                setext_last_text_line_end(src, block, line_start)
            } else {
                trim_trailing_ws(src, line_start, content_end)
            };
            let ulid = idgen();
            patches.push(Patch { at, delete: 0, insert: format!(" {{#{ulid}}}") });
        }
        Some(tag) => plan_id_tag_relabel(tag, idgen, patches),
    }
}

/// Setext 見出しブロックの「テキスト最終行」(下線行の直前の行)の内容末尾
/// (行末空白の手前)を求める。
fn setext_last_text_line_end(src: &str, block: &SmlBlock, line_start: usize) -> usize {
    let lines = crate::scan::split_lines_range(src, Span::new(line_start, block.span.end));
    debug_assert!(lines.len() >= 2, "Setext 見出しはテキスト行+下線行の最低2行を持つ");
    // 末尾の空行(通常は無い)を除いた最後の行が下線行、その1つ前がテキスト最終行。
    let text_line = &lines[lines.len().saturating_sub(2)];
    trim_trailing_ws(src, text_line.content.start, text_line.content.end)
}

/// 行型ブロック(フェンスマーカー等)の ID タグを計画する。
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

/// D24: 項目列を文書順に走査し、各項目 → その子リスト(あれば、再帰)の順で ID を
/// 計画する。子リストは親項目の行の直後に現れるため、これが文書順と一致する。
fn plan_list_items_ids(src: &str, items: &[ListItem], idgen: &mut dyn FnMut() -> Ulid, patches: &mut Vec<Patch>) {
    for item in items {
        plan_list_item_id(src, item, idgen, patches);
        if let Some(child) = &item.child {
            plan_list_items_ids(src, &child.items, idgen, patches);
        }
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

/// 段落の ID を計画する(ケース2・3・5・6)。`quote_depth` は M6(D40)の blockquote
/// ネスト深さ(挿入行のマーカー前置に使う。トップレベルは 0 = 前置なし)。
fn plan_prose_id(
    src: &str,
    block: &SmlBlock,
    idgen: &mut dyn FnMut() -> Ulid,
    patches: &mut Vec<Patch>,
    quote_depth: usize,
) {
    match &block.attrs {
        None => {
            // ケース2: 前置属性行が無い → 先頭行の直前に `[id=ULID]\n` を挿入。
            // 引用内(M6 D40)では挿入行自体も `> ` で始める必要がある。
            let ulid = idgen();
            let prefix = "> ".repeat(quote_depth);
            patches.push(Patch { at: block.span.start, delete: 0, insert: format!("{prefix}[id={ulid}]\n") });
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

/// 見出し/フェンスマーカー/コードフェンス開始行の「自身の行」の境界を求める。
///
/// 戻り値は `(行の開始オフセット, 行内容の終端オフセット(改行・CRを含まない))`。
/// `block.attrs` が束縛されている場合、ブロックの前置属性行の次の行が自身の行になる
/// (`SmlBlock.span` はフェンス/コードフェンスの場合ブロック全体〈閉じ行まで〉を
/// 指すため、ここでは `block.span.end` に頼らず、行頭から自前で改行を探す)。
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

/// `from` から改行(`\n`)を探し、その直後のオフセットを返す(無ければファイル末尾)。
/// フロントマターの開き `---` 行の直後に `id: ULID\n` を挿入する位置の計算に使う。
fn line_end_including_newline(src: &str, from: usize) -> usize {
    let bytes = src.as_bytes();
    let mut i = from;
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    if i < bytes.len() { i + 1 } else { bytes.len() }
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

    /// `fmt_ok` に渡す入力は本テストファイル内ではすべてフロントマター無しなので、
    /// 出力は必ずケース6で生成された `---\nid: <ULID>\n---\n\n` で始まる。それを
    /// 取り除いた本体だけを見たいテストのための補助(D-B4でフロントマターが常に
    /// 先頭に付くようになったため、既存のブロック単位アサーションを本体に対して
    /// 行うのに使う)。
    fn strip_generated_frontmatter(text: &str) -> &str {
        text.splitn(5, '\n').nth(4).unwrap_or("")
    }

    // ---- ケース1: 行型ブロックに ID タグ無し(見出し・リスト項目・フェンス・コードフェンス) --

    #[test]
    fn heading_without_id_gets_trailing_tag() {
        let out = fmt_ok("# Title\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("# Title {#"), "{}", body);
        assert!(body.ends_with("}\n"));
        assert_eq!(out.patches.len(), 2); // フロントマター生成 + 見出しタグ
        assert!(out.patches.iter().all(|p| p.delete == 0));
    }

    #[test]
    fn heading_without_id_and_trailing_whitespace_inserts_before_whitespace() {
        let src = "# Title  \n";
        let out = fmt_ok(src);
        // 挿入位置は「行内容の末尾」= 行末空白の手前。
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("# Title {#"), "{}", body);
        assert!(body.contains("}  \n"), "{}", body);
    }

    #[test]
    fn list_item_without_id_gets_trailing_tag() {
        let out = fmt_ok("- one\n- two\n");
        // リスト全体にも前置属性行が無いので `[id=...]` 行が先頭に挿入される(D11)。
        let body = strip_generated_frontmatter(&out.text);
        let lines: Vec<&str> = body.lines().collect();
        assert!(lines[0].starts_with("[id="), "{}", body);
        assert!(lines[1].starts_with("- one {#") && lines[1].ends_with('}'), "{}", body);
        assert!(lines[2].starts_with("- two {#") && lines[2].ends_with('}'), "{}", body);
        assert_eq!(out.patches.len(), 4); // フロントマター + リストID + 項目×2
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
        // 上記は id_tag あり(フェンス自体は no-op)。フロントマター生成の1件だけ発生する。
        assert_eq!(out.patches.len(), 1);
        let body = strip_generated_frontmatter(&out.text);
        assert_eq!(body, "::math {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\nx\n::\n");

        // 無しのケースを別途確認する。
        let out2 = fmt_ok("::math\nx\n::\n");
        let body2 = strip_generated_frontmatter(&out2.text);
        assert!(body2.starts_with("::math {#"), "{}", body2);
        assert_eq!(out2.patches.len(), 2); // フロントマター + フェンスタグ
    }

    #[test]
    fn table_marker_without_id_gets_trailing_tag() {
        let out = fmt_ok("::table\n@rows:\n  - a: [b]\n::\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("::table {#"), "{}", body);
    }

    #[test]
    fn figure_marker_without_id_gets_trailing_tag() {
        let out = fmt_ok("::figure\n[kind=chart]\n::\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("::figure {#"), "{}", body);
    }

    #[test]
    fn code_fence_without_id_gets_trailing_tag() {
        let out = fmt_ok("```rust\nfn main() {}\n```\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("```rust {#"), "{}", body);
    }

    #[test]
    fn code_fence_with_ulid_tag_is_untouched() {
        let src = "```rust {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\nfn main() {}\n```\n";
        let out = fmt_ok(src);
        assert_eq!(out.patches.len(), 1); // フロントマター生成のみ
        let body = strip_generated_frontmatter(&out.text);
        assert_eq!(body, src);
    }

    // ---- ケース2: 段落・リスト全体に前置属性行が無い -----------------------------

    #[test]
    fn paragraph_without_attrs_gets_id_line_inserted() {
        let out = fmt_ok("Hello world.\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("[id="), "{}", body);
        assert!(body.contains("]\nHello world.\n"), "{}", body);
    }

    #[test]
    fn list_without_attrs_gets_id_line_inserted_before_list() {
        let out = fmt_ok("- one\n- two\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("[id="), "{}", body);
        assert!(body.contains("]\n- one {#"), "{}", body);
    }

    // ---- ケース3: 段落・リスト全体の前置属性行に id キーが無い --------------------

    #[test]
    fn paragraph_attrs_without_id_key_gets_id_prepended() {
        let out = fmt_ok("[supports=x]\nParagraph.\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("[id="), "{}", body);
        assert!(body.contains(", supports=x]"), "{}", body);
    }

    #[test]
    fn list_with_attrs_without_id_key_gets_id_prepended() {
        let out = fmt_ok("[ordered=false]\n- one\n- two\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("[id="), "{}", body);
        assert!(body.contains(", ordered=false]"), "{}", body);
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

    #[test]
    fn code_fence_label_id_tag_gets_relabeled_with_alias() {
        let out = fmt_ok("```rust {#my-snippet}\nfn main() {}\n```\n");
        assert!(out.text.contains("alias=my-snippet"), "{}", out.text);
        assert!(!out.text.contains("{#my-snippet}"), "{}", out.text);
    }

    // ---- ケース5: 属性行の id が非ULIDラベル -------------------------------------

    #[test]
    fn attr_line_label_id_gets_relabeled_with_alias() {
        let out = fmt_ok("[id=key-finding, supports=x]\nParagraph.\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("[id="), "{}", body);
        assert!(body.contains(", alias=key-finding, supports=x]"), "{}", body);
        assert!(!body.contains("id=key-finding"), "{}", body);
    }

    #[test]
    fn list_attr_line_label_id_gets_relabeled_with_alias() {
        let out = fmt_ok("[id=list-label]\n- one\n- two\n");
        let body = strip_generated_frontmatter(&out.text);
        assert!(body.starts_with("[id="), "{}", body);
        assert!(body.contains(", alias=list-label]"), "{}", body);
        assert!(!body.contains("id=list-label"), "{}", body);
    }

    // ---- ケース6: フロントマター生成/注入 ------------------------------------------

    #[test]
    fn frontmatter_missing_gets_generated_at_offset_zero() {
        let out = fmt_ok("# Title\n");
        assert!(out.text.starts_with("---\nid: "), "{}", out.text);
        assert!(out.text.contains("\n---\n\n# Title"), "{}", out.text);
        let fm_patch = out.patches.iter().find(|p| p.at == 0).expect("expected a patch at offset 0");
        assert_eq!(fm_patch.delete, 0);
        assert!(fm_patch.insert.starts_with("---\nid: "), "{}", fm_patch.insert);
        assert!(fm_patch.insert.ends_with("---\n\n"), "{}", fm_patch.insert);
    }

    #[test]
    fn frontmatter_without_id_gets_id_line_inserted_after_open() {
        let src = "---\ntitle: T\n---\n\n# Title\n";
        let out = fmt_ok(src);
        assert!(out.text.starts_with("---\nid: "), "{}", out.text);
        // id 行は開き `---` 行の直後(既存の `title:` 行より前)に挿入される。
        let ulid_len = ulid::Ulid::nil().to_string().len();
        let after_prefix = &out.text["---\nid: ".len()..];
        assert!(after_prefix[ulid_len..].starts_with("\ntitle: T\n---\n\n# Title"), "{}", out.text);
    }

    #[test]
    fn frontmatter_with_id_is_untouched() {
        let src =
            "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n---\n\n# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAW}\n";
        let out = fmt_ok(src);
        assert_eq!(out.text, src);
        assert!(out.patches.is_empty());
    }

    #[test]
    fn frontmatter_with_id_and_title_is_untouched() {
        let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: T\n---\n\n# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAW}\n";
        let out = fmt_ok(src);
        assert_eq!(out.text, src);
        assert!(out.patches.is_empty());
    }

    // ---- 既に ULID を持つ → 無変更(フロントマター生成は別途発生する) ----------------

    #[test]
    fn heading_with_ulid_tag_is_untouched() {
        let src = "# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n";
        let out = fmt_ok(src);
        assert_eq!(out.patches.len(), 1); // フロントマター生成のみ
        let body = strip_generated_frontmatter(&out.text);
        assert_eq!(body, src);
    }

    #[test]
    fn heading_with_ulid_and_alias_is_untouched() {
        let src = "# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=foo}\n";
        let out = fmt_ok(src);
        assert_eq!(out.patches.len(), 1); // フロントマター生成のみ
        let body = strip_generated_frontmatter(&out.text);
        assert_eq!(body, src);
    }

    #[test]
    fn attr_line_with_ulid_id_is_untouched() {
        let src = "[id=01ARZ3NDEKTSV4RRFFQ69G5FAV, supports=x]\nParagraph.\n";
        let out = fmt_ok(src);
        assert_eq!(out.patches.len(), 1); // フロントマター生成のみ
        let body = strip_generated_frontmatter(&out.text);
        assert_eq!(body, src);
    }

    #[test]
    fn list_with_ulid_id_is_untouched() {
        let src = "[id=01ARZ3NDEKTSV4RRFFQ69G5FAV]\n- one {#01ARZ3NDEKTSV4RRFFQ69G5FAW}\n- two {#01ARZ3NDEKTSV4RRFFQ69G5FAX}\n";
        let out = fmt_ok(src);
        assert_eq!(out.patches.len(), 1); // フロントマター生成のみ
        let body = strip_generated_frontmatter(&out.text);
        assert_eq!(body, src);
    }

    #[test]
    fn fully_formatted_document_is_a_no_op() {
        let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n---\n\n\
                    # Title {#01ARZ3NDEKTSV4RRFFQ69G5FAW}\n\n\
                    [id=01ARZ3NDEKTSV4RRFFQ69G5FAX]\nA paragraph.\n";
        let out = fmt_ok(src);
        assert!(out.patches.is_empty());
        assert_eq!(out.text, src);
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

    // ---- 回帰: フロントマター無し・先頭ブロックが offset 0 の純挿入パッチを ----------
    // ---- 発行するケース(段落/リスト)でも、フロントマターが必ず先頭に来ること ---------
    //
    // フロントマター生成(ケース6)と、先頭ブロックの ID 挿入(段落のケース2 /
    // リストのケース2)は、フロントマター無し文書では両方とも offset 0 の挿入
    // パッチになる。`plan_patches` の実装コメント(83-90行目)が説明するとおり、
    // `apply_patches` は同一 offset の挿入を「後から適用したパッチほどバッファ先頭に
    // 来る」ため、フロントマターのパッチは `patches` 列の末尾に置かれなければならない。
    // ここではその順序衝突を明示的に固定する(strip_generated_frontmatter に頼る他の
    // テストはこれを暗黙に前提しているだけだった)。

    #[test]
    fn frontmatter_ends_up_first_when_leading_block_is_a_paragraph_at_offset_zero() {
        let out = fmt_ok("Hello world.\n");
        // フロントマターが段落の `[id=...]` 挿入より前(バッファ上でより先)に来る。
        assert!(out.text.starts_with("---\nid: "), "{}", out.text);
        // 閉じ `---` の直後に空行を挟んで `[id=` が続く(段落の ID 行はフロントマターの後)。
        let (_, after_close) = out.text.split_once("---\n\n").expect("closing --- present");
        assert!(after_close.starts_with("[id="), "{}", after_close);
        assert!(after_close.contains("]\nHello world.\n"), "{}", after_close);
    }

    #[test]
    fn frontmatter_ends_up_first_when_leading_block_is_a_list_at_offset_zero() {
        let out = fmt_ok("- one\n- two\n");
        assert!(out.text.starts_with("---\nid: "), "{}", out.text);
        let (_, after_close) = out.text.split_once("---\n\n").expect("closing --- present");
        // リスト全体の `[id=...]` 行がフロントマターの直後に続く(リスト項目より前)。
        assert!(after_close.starts_with("[id="), "{}", after_close);
        assert!(after_close.contains("]\n- one {#"), "{}", after_close);
    }

    // ---- 決定的 idgen による固定出力 ----------------------------------------------

    #[test]
    fn deterministic_idgen_produces_stable_ulid_sequence() {
        // 発行順(D-B4): フロントマター → 見出し → リスト全体 → リスト項目。
        let fixed: Vec<Ulid> = [
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            "01ARZ3NDEKTSV4RRFFQ69G5FAW",
            "01ARZ3NDEKTSV4RRFFQ69G5FAX",
            "01ARZ3NDEKTSV4RRFFQ69G5FAY",
        ]
        .iter()
        .map(|s| s.parse().unwrap())
        .collect();
        let mut it = fixed.clone().into_iter();
        let mut idgen = move || it.next().expect("test provides exactly enough ulids");
        let out = format_with("# A\n\n- b\n", &mut idgen).unwrap();
        assert_eq!(
            out.text,
            format!(
                "---\nid: {}\n---\n\n# A {{#{}}}\n\n[id={}]\n- b {{#{}}}\n",
                fixed[0], fixed[1], fixed[2], fixed[3]
            )
        );
    }
}
