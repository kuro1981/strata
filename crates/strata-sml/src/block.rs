//! 層B: ブロック内パース(sml-parser-design.md §3)。
//!
//! 層A(`scan.rs`)が確定させた `RawBlock` 列を受け取り、以下を解決して最終的な
//! `SmlBlock`(ast.rs)を組み立てる:
//!
//! - 行末 `{#id}` / `{#ULID alias=x}` タグの抽出(`inner_span` は fmt の置換対象)
//! - 属性行の `key=value` パース(リスト値 `[a, b]`、引用符付き値 `"..."`)
//! - ULID 判定(Crockford Base32)と `RefTarget::Ulid | Label` の振り分け
//! - `{#}` と `[id=]` の併記検出 → `DuplicateId`
//! - key/エイリアス字句 `[A-Za-z0-9_-]+` の検証 → `BadKeyCharset`(D5)
//!
//! インライン本体(`inline.rs`)・表本体(`table.rs`)は WP1/WP2 時点ではプレースホルダを
//! 呼ぶだけで、実装そのものはここでは行わない。

use ulid::Ulid;

use crate::ast::{
    AttrLine, AttrValue, BlockKind, FenceBlock, FenceBody, FenceKind, IdTag, ListBlock, ListItem, RefDefs,
    RefTarget, SmlBlock, SmlInline,
};
use crate::error::{Diag, DiagKind};
use crate::scan::{fence_kind_word, looks_like_attr_line, looks_like_html_line, split_lines_range, RawBlock, RawKind};
use crate::span::Span;
use crate::value::DateFormat;

/// 層Aの `RawBlock` 列を最終的な `SmlBlock` 列に変換する。参照スタイルリンクの
/// 定義行(M6 D40)を先に収集し、インライン解決に使う。
pub(crate) fn build_blocks(src: &str, raw_blocks: Vec<RawBlock>, diags: &mut Vec<Diag>) -> Vec<SmlBlock> {
    let refdefs = collect_link_ref_defs(src, &raw_blocks);
    build_blocks_with(src, raw_blocks, diags, &refdefs)
}

fn build_blocks_with(src: &str, raw_blocks: Vec<RawBlock>, diags: &mut Vec<Diag>, refdefs: &RefDefs) -> Vec<SmlBlock> {
    raw_blocks.into_iter().map(|rb| build_block(src, rb, diags, refdefs)).collect()
}

/// M6(D40、監査②4): 参照スタイルリンクの定義行(`[label]: url "title"`)を集める。
/// 最上位のブロック列だけを見る(blockquote 内の定義行は対象外 — 裁量、最終報告参照。
/// 実運用上の定義行はほぼ常に文書トップレベルに置かれる)。
fn collect_link_ref_defs(src: &str, raw_blocks: &[RawBlock]) -> RefDefs {
    let mut map = RefDefs::new();
    for rb in raw_blocks {
        if let RawKind::LinkRefDef { line_span } = &rb.kind
            && let Some((label, url_span, _title)) = parse_link_ref_def_line(src, *line_span)
        {
            map.entry(label).or_insert(url_span);
        }
    }
    map
}

/// `[label]: url ["title"]` 行を `(正規化ラベル, url span, title span)` へ分解する。
/// scan.rs の `looks_like_link_ref_def` が既に大まかな形を検証済みなので、ここでは
/// 失敗しても `None` を返すだけ(呼び出し側は寛容にスキップする)。
fn parse_link_ref_def_line(src: &str, line_span: Span) -> Option<(String, Span, Option<Span>)> {
    let bytes = src.as_bytes();
    let mut i = line_span.start;
    while i < line_span.end && bytes[i] == b' ' {
        i += 1;
    }
    if i >= line_span.end || bytes[i] != b'[' {
        return None;
    }
    let label_start = i + 1;
    let mut j = label_start;
    while j < line_span.end && bytes[j] != b']' {
        j += 1;
    }
    if j >= line_span.end || j == label_start {
        return None;
    }
    let label = src[label_start..j].to_string();
    let mut k = j + 1;
    if k >= line_span.end || bytes[k] != b':' {
        return None;
    }
    k += 1;
    while k < line_span.end && bytes[k] == b' ' {
        k += 1;
    }
    let url_start = k;
    let mut url_end = url_start;
    while url_end < line_span.end && bytes[url_end] != b' ' {
        url_end += 1;
    }
    if url_start == url_end {
        return None;
    }
    let url_span = Span::new(url_start, url_end);
    let mut m = url_end;
    while m < line_span.end && bytes[m] == b' ' {
        m += 1;
    }
    let title_span = if m < line_span.end && bytes[m] == b'"' && line_span.end > m + 1 && bytes[line_span.end - 1] == b'"' {
        Some(Span::new(m + 1, line_span.end - 1))
    } else {
        None
    };
    Some((crate::inline::normalize_label(&label), url_span, title_span))
}

/// 複数行段落のインラインパース。行スパンが連続(改行1バイトのみを挟む)していれば
/// 従来どおり1つの結合スパンとして渡す(強調等が行を跨げる)。連続していない場合
/// (M6 D40: blockquote 内の段落 — 行頭 `> ` マーカーがスパン間に挟まる)は行ごとに
/// パースし、行間には改行1バイトの Text スパンを差し込んで繋ぐ(マーカーの `> ` を
/// インライン本文へ混入させない。跨行の強調は引用内では非対応 — 裁量、最終報告参照)。
fn parse_paragraph_inlines(src: &str, line_spans: &[Span], diags: &mut Vec<Diag>, refdefs: &RefDefs) -> Vec<SmlInline> {
    let contiguous =
        line_spans.windows(2).all(|w| w[1].start == w[0].end + 1);
    if contiguous {
        let span = Span::new(line_spans[0].start, line_spans[line_spans.len() - 1].end);
        return crate::inline::parse_inlines(src, span, diags, refdefs);
    }
    let mut out = Vec::new();
    for (i, span) in line_spans.iter().enumerate() {
        if i > 0 {
            // 直前行の直後にある改行1バイトを Text として繋ぐ(実在のソーススパン)。
            let nl = line_spans[i - 1].end;
            out.push(SmlInline::Text(Span::new(nl, nl + 1)));
        }
        out.extend(crate::inline::parse_inlines(src, *span, diags, refdefs));
    }
    out
}

/// パラグラフ本文の各行を HTML ブロック/インラインらしきパターンで検査し、Tier3
/// (M6 D40)の `HtmlNotSupported` Warning を積む(意味グラフには落とさずリテラル扱い
/// のまま継続する)。
fn check_html_like_lines(src: &str, line_spans: &[Span], diags: &mut Vec<Diag>) {
    for span in line_spans {
        if looks_like_html_line(span.slice(src)) {
            diags.push(Diag::new(
                DiagKind::HtmlNotSupported,
                *span,
                "HTML ブロック/インラインらしき行です(SML は HTML を構造化しません。リテラル扱いのまま継続します)",
            ));
        }
    }
}

fn build_block(src: &str, rb: RawBlock, diags: &mut Vec<Diag>, refdefs: &RefDefs) -> SmlBlock {
    let attrs = rb.attr_line_span.map(|span| parse_attr_line(src, span, diags));

    let kind = match rb.kind {
        RawKind::Heading { level, line_span } => {
            let (text_span, id_tag) = extract_trailing_id_tag(src, line_span, diags);
            // 行頭マーカー(`#`×level + 空白)は level に既に反映済みなので、
            // インライン本文には含めない。
            let text_span = strip_heading_marker(src, text_span, level);
            // M6(D40): 見出し閉じ装飾(`# H #####`)の末尾 `#` 列を除去する。
            let text_span = strip_atx_closing_hashes(src, text_span);
            let inline = crate::inline::parse_inlines(src, text_span, diags, refdefs);
            BlockKind::Heading { level, inline, id_tag }
        }
        RawKind::SetextHeading { level, line_spans } => {
            // M6(D40、監査②9): 段落直後の Setext 下線(`===`/`---`)を見出しへ昇格する。
            let last = *line_spans.last().expect("Setext 見出しは最低1行のテキストを持つ");
            let (last_text_span, id_tag) = extract_trailing_id_tag(src, last, diags);
            let mut spans = line_spans.clone();
            *spans.last_mut().expect("非空を確認済み") = last_text_span;
            let inline = parse_paragraph_inlines(src, &spans, diags, refdefs);
            BlockKind::Heading { level, inline, id_tag }
        }
        RawKind::Paragraph { line_spans } => {
            check_html_like_lines(src, &line_spans, diags);
            let inline = parse_paragraph_inlines(src, &line_spans, diags, refdefs);
            BlockKind::Paragraph { inline }
        }
        RawKind::ThematicBreak => BlockKind::ThematicBreak,
        RawKind::LinkRefDef { line_span } => match parse_link_ref_def_line(src, line_span) {
            Some((label, url, title)) => BlockKind::LinkRefDef { label, url, title },
            None => BlockKind::LinkRefDef { label: String::new(), url: line_span, title: None },
        },
        RawKind::Quote { inner_lines } => {
            let inner_raw = crate::scan::scan_lines(src, &inner_lines, diags);
            let blocks = build_blocks_with(src, inner_raw, diags, refdefs);
            BlockKind::Quote { blocks }
        }
        RawKind::GfmTable { header_span, row_spans } => {
            let body = crate::gfm_table::parse_gfm_table_body(src, header_span, &row_spans, diags);
            BlockKind::GfmTable(body)
        }
        RawKind::List { ordered, start, item_line_spans } => {
            // D24(2026-07-14 裁定): item_line_spans はインデント無し(ルート)・
            // インデント有り(ネスト候補)の両方を含む文書順のフラット列(scan.rs が
            // マーカー行として一括りにしている)。ここでインデント量(2スペース/レベル)
            // を解釈して木構造に組み立てる(sml-spec §6.1 の table.rs と同じ手法)。
            let mut idx = 0;
            let items = parse_list_items(src, &item_line_spans, &mut idx, diags, 0, refdefs);
            BlockKind::List { ordered, items, start }
        }
        RawKind::Fence { marker_line_span, body_span } => {
            // scan.rs の is_fence_open が既に kind ワードを検証済みなので、ここで
            // None になるのは内部不整合(バグ)。
            let fence_kind = match fence_kind_word(marker_line_span.slice(src)).as_deref() {
                Some("table") => FenceKind::Table,
                Some("math") => FenceKind::Math,
                Some("figure") => FenceKind::Figure,
                Some("record") => FenceKind::Record,
                _ => unreachable!("scan.rs はフェンス種別を検証済みのはず"),
            };
            let (_, id_tag) = extract_trailing_id_tag(src, marker_line_span, diags);
            let (fence_attrs, remaining_body) = split_fence_attrs(src, body_span, fence_kind, diags);
            // sml-spec §6: フェンス内属性行に id は書けない(ID はマーカーの {#...} のみ)。
            for al in &fence_attrs {
                for (key, _, span) in &al.entries {
                    if key == "id" {
                        diags.push(Diag::new(
                            DiagKind::IdNotAllowedHere,
                            *span,
                            "フェンス内属性行に id は書けません(フェンスマーカーの {#...} を使ってください)",
                        ));
                    }
                }
            }
            let body = match fence_kind {
                FenceKind::Table => {
                    let date_format = extract_date_format(&fence_attrs, diags);
                    FenceBody::Table(crate::table::parse_table_body(src, remaining_body, date_format, diags))
                }
                FenceKind::Math => FenceBody::MathTex(remaining_body),
                FenceKind::Figure => FenceBody::Figure,
                FenceKind::Record => {
                    let date_format = extract_date_format(&fence_attrs, diags);
                    FenceBody::Record(crate::record::parse_record_body(src, remaining_body, date_format, diags))
                }
            };
            BlockKind::Fence(FenceBlock { fence_kind, id_tag, fence_attrs, body })
        }
        RawKind::CodeFence { marker_line_span, body_span } => {
            // D10(2026-07-14 改定): コードフェンス開始行末尾の `{#id}` を見出しと
            // 同じ規則で抽出する(行型ブロック)。
            let (text_span, id_tag) = extract_trailing_id_tag(src, marker_line_span, diags);
            // M6(D40、監査②8): `~~~` フェンスも `` ``` `` と同等に扱う。
            let lang = text_span.slice(src).trim_start_matches(['`', '~']).trim().to_string();
            BlockKind::CodeFence { lang, body: body_span, id_tag }
        }
    };

    check_id_placement(&attrs, &kind, diags);
    check_id_value(&attrs, &kind, diags);
    check_unknown_attr_keys(&attrs, diags);

    SmlBlock { span: rb.full_span, attrs, kind }
}

/// sml-spec §4.1 + D17: ブロック前置属性行(意味エッジ宣言用)のキーが
/// `supports` / `depends-on` / `cites` / `id` / `alias` / `class`(D23、2026-07-14
/// 裁定)のいずれでもなければ `UnknownAttrKey`(`Warning`)。`apply_block_attrs`
/// (strata-build)は未知キーを従来どおり黙って無視し続けるため、これは「エッジが
/// 張られないタイポ」に気付くための警告に過ぎない。フェンス内属性行(`::table`/
/// `::figure` の `[caption=...]` 等、`fb.fence_attrs`)は語彙が別物なのでこの検査の
/// 対象外。
fn check_unknown_attr_keys(attrs: &Option<AttrLine>, diags: &mut Vec<Diag>) {
    const KNOWN: [&str; 6] = ["supports", "depends-on", "cites", "id", "alias", "class"];
    let Some(attr_line) = attrs else { return };
    for (key, _, span) in &attr_line.entries {
        if !KNOWN.contains(&key.as_str()) {
            diags.push(Diag::new(
                DiagKind::UnknownAttrKey,
                *span,
                format!("属性キー '{key}' は既知のキー(supports/depends-on/cites/id/alias/class)ではありません(タイポの可能性。エッジは張られません)"),
            ));
        }
    }
}

/// sml-spec §4: 「id を書けるのはプローズブロックの属性行だけ(行型は `{#}` を使う。
/// 重複はエラー)」の実装。行型ブロック(見出し・フェンスマーカー・コードフェンス
/// 開始行、D10)の前置属性行に `id=` キーがある場合を検出する。2ケースに分岐する:
///
/// - 同じブロックが自身の `{#...}` タグも持つ(併記) → `DuplicateId`(既存の挙動)
/// - `{#...}` タグは無く、属性行の `id=` のみがある → `IdNotAllowedHere`(新設)
///
/// **リスト全体は D11(2026-07-14 改定)によりプローズブロック扱い**: リスト全体を
/// 指す単一の行が存在しないため、段落と同様に前置属性行 `[id=...]` で ID を与える。
/// 項目の `{#...}` とは別エンティティであり併記可(重複エラーにしない)。M1 実装の
/// 「リストは常に IdNotAllowedHere」はこの改定で廃止された。
fn check_id_placement(attrs: &Option<AttrLine>, kind: &BlockKind, diags: &mut Vec<Diag>) {
    let Some(attr_line) = attrs else { return };

    enum LineTypeState {
        /// プローズブロック(段落・リスト全体〈D11〉)。id= はここでのみ許される。
        NotLineType,
        /// 行型ブロックで、直接対応する `{#...}` タグを持つ(併記なら DuplicateId)。
        HasOwnIdTag,
        /// 行型ブロックで、`{#...}` タグとの直接対応が無い(常に IdNotAllowedHere)。
        NoDirectIdTag,
    }

    let state = match kind {
        BlockKind::Heading { id_tag, .. } => {
            if id_tag.is_some() { LineTypeState::HasOwnIdTag } else { LineTypeState::NoDirectIdTag }
        }
        BlockKind::Fence(fb) => {
            if fb.id_tag.is_some() { LineTypeState::HasOwnIdTag } else { LineTypeState::NoDirectIdTag }
        }
        BlockKind::CodeFence { id_tag, .. } => {
            if id_tag.is_some() { LineTypeState::HasOwnIdTag } else { LineTypeState::NoDirectIdTag }
        }
        // M6(D40): blockquote・GFM 表はプローズ扱い(前置属性行で id を与える、
        // List/Paragraph と同じ規則)。水平線・参照リンク定義行は id 概念を持たない
        // ブロックなので同様に許容側(診断なし)に倒す(裁量、最終報告参照)。
        BlockKind::Paragraph { .. }
        | BlockKind::List { .. }
        | BlockKind::Quote { .. }
        | BlockKind::GfmTable(_)
        | BlockKind::ThematicBreak
        | BlockKind::LinkRefDef { .. } => LineTypeState::NotLineType,
    };

    if matches!(state, LineTypeState::NotLineType) {
        return;
    }

    for (key, _, span) in &attr_line.entries {
        if key == "id" {
            match state {
                LineTypeState::HasOwnIdTag => diags.push(Diag::new(
                    DiagKind::DuplicateId,
                    *span,
                    "行型ブロックの {#...} と属性行の id= が併記されています",
                )),
                LineTypeState::NoDirectIdTag => diags.push(Diag::new(
                    DiagKind::IdNotAllowedHere,
                    *span,
                    "id を書けるのはプローズブロックの属性行だけです(行型ブロックは {#...} を使ってください)",
                )),
                LineTypeState::NotLineType => unreachable!("早期リターン済み"),
            }
        }
    }
}

/// sml-spec §3.2(2026-07-13 裁定): 属性行の `id` の値は裸トークン(ULID または
/// 人間ラベル)のみ。引用符付き(`[id="..."]`)・リスト(`[id=[a, b]]`)は
/// `BadIdValue`、裸トークンでも字句が `[A-Za-z0-9_-]+` の外なら `BadKeyCharset`。
/// 診断化しないと fmt が「ULID を発行しないまま静かに素通り」する経路が残る。
///
/// 対象はプローズブロック(段落・リスト全体〈D11、2026-07-14 改定〉)。行型ブロック
/// (見出し・フェンス・コードフェンス〈D10〉)は `check_id_placement` が `id=` の
/// 存在自体を弾く(DuplicateId / IdNotAllowedHere)ため、ここでは値の検証を重ねない。
fn check_id_value(attrs: &Option<AttrLine>, kind: &BlockKind, diags: &mut Vec<Diag>) {
    if !matches!(
        kind,
        BlockKind::Paragraph { .. } | BlockKind::List { .. } | BlockKind::Quote { .. } | BlockKind::GfmTable(_)
    ) {
        return;
    }
    let Some(attr_line) = attrs else { return };
    for (key, value, span) in &attr_line.entries {
        if key != "id" {
            continue;
        }
        match value {
            AttrValue::Single(v) => {
                if v.parse::<Ulid>().is_err() && !is_valid_key_charset(v) {
                    diags.push(Diag::new(
                        DiagKind::BadKeyCharset,
                        *span,
                        format!("id ラベル '{v}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
                    ));
                }
            }
            AttrValue::Quoted(_) | AttrValue::List(_) => {
                diags.push(Diag::new(
                    DiagKind::BadIdValue,
                    *span,
                    "id の値は裸トークン(ULID またはラベル)のみです(引用符・リストは不可)",
                ));
            }
        }
    }
}

/// 見出し行の text_span から行頭マーカー(`#`×level + 後続空白)を取り除く。
/// レベルは AST の `level` フィールドが持つので、インライン本文に `#` は残さない。
fn strip_heading_marker(src: &str, span: Span, level: u8) -> Span {
    let bytes = span.slice(src).as_bytes();
    let mut off = 0;
    let mut hashes = 0u8;
    while off < bytes.len() && bytes[off] == b'#' && hashes < level {
        off += 1;
        hashes += 1;
    }
    while off < bytes.len() && (bytes[off] == b' ' || bytes[off] == b'\t') {
        off += 1;
    }
    Span::new(span.start + off, span.end)
}

/// M6(D40): ATX 見出しの閉じ装飾(`# H #####`)の末尾 `#` 列を除去する。CommonMark
/// 準拠: 末尾の `#` 列の直前に空白が必須(無ければテキストの一部として残す)。
fn strip_atx_closing_hashes(src: &str, span: Span) -> Span {
    let text = span.slice(src);
    let trimmed_end = text.trim_end();
    let before_hashes = trimmed_end.trim_end_matches('#');
    if before_hashes.len() == trimmed_end.len() || before_hashes.is_empty() {
        return span;
    }
    if !before_hashes.ends_with(' ') && !before_hashes.ends_with('\t') {
        return span;
    }
    let new_len = before_hashes.trim_end().len();
    Span::new(span.start, span.start + new_len)
}

/// リスト項目行の text_span から項目マーカー(`- ` / `N. ` と後続空白)を取り除く。
/// 予期しない形(マーカーが検出できない)なら安全側に倒して span をそのまま返す。
/// M6(D40): 代替マーカー(`*`/`+` 箇条書き、`N)` 順序)にも対応する。
fn strip_list_marker(src: &str, span: Span) -> Span {
    let bytes = span.slice(src).as_bytes();
    let mut off = 0;
    while off < bytes.len() && bytes[off] == b' ' {
        off += 1;
    }
    if off < bytes.len() && matches!(bytes[off], b'-' | b'*' | b'+') {
        off += 1;
    } else {
        let digits_start = off;
        while off < bytes.len() && bytes[off].is_ascii_digit() {
            off += 1;
        }
        if off == digits_start || off >= bytes.len() || !matches!(bytes[off], b'.' | b')') {
            return span; // マーカー無し(内部不整合)。全体を返す
        }
        off += 1;
    }
    while off < bytes.len() && (bytes[off] == b' ' || bytes[off] == b'\t') {
        off += 1;
    }
    Span::new(span.start + off, span.end)
}

fn is_valid_key_charset(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

// ---- D24(2026-07-14 裁定): ネストしたリスト項目のパース ------------------------
//
// `table.rs` の次元木パーサ(`parse_dim_list` / `parse_member_list`)と同じ手法:
// インデント量を絶対列(2スペース/レベル)として解釈し、レベルが下がれば呼び出し元に
// 制御を返す(再帰下降)。「リストとして解釈できないインデント行には診断を出す」
// (D24)を `InconsistentIndent`(table.rs と同一種別。全か無かの整合上、既存の
// テーブル本体インデント診断と同じく `Error` 重大度)で実現する — 診断を出しつつ
// その行はスキップして処理を続ける(パーサは1件のエラーで止まらない、sml-spec §8.2)。

/// 行頭の半角スペースの個数を数える(2スペース/レベルのインデント量)。
fn leading_space_count(src: &str, span: Span) -> usize {
    span.slice(src).as_bytes().iter().take_while(|&&b| b == b' ').count()
}

/// インデント量(スペース数)を「2スペース単位のレベル」に変換する。
/// 戻り値の bool は「2の倍数だったか」(false なら `InconsistentIndent` 対象)。
fn list_level_of(indent: usize) -> (usize, bool) {
    (indent / 2, indent.is_multiple_of(2))
}

/// 行(インデント込み)の先頭マーカーが番号付き(`N.`/`N)`)かどうか。この行が既に
/// scan.rs でマーカー行と判定済みであることを前提とする。M6(D40): 代替マーカー
/// (`*`/`+`)も箇条書きとして正しく判定するため `scan::list_marker_ordered` に委譲する
/// (旧実装は「`-` でなければ番号付き」という誤判定をしていた — alt bullet 追加に伴う
/// 修正)。
fn line_marker_is_ordered(src: &str, span: Span) -> bool {
    let stripped = span.slice(src).trim_start_matches(' ');
    crate::scan::list_marker_ordered(stripped).unwrap_or(false)
}

/// M6(D40 Tier2): タスクリストのチェック状態(`- [ ] text` / `- [x] text`)。項目
/// マーカーを除いた `text_span` の先頭が `[ ] `/`[x] `(大文字 X も許容)なら、それを
/// 取り除いたスパンと `Some(checked)` を返す。該当しなければ元の span と `None`。
fn strip_task_marker(src: &str, span: Span) -> (Span, Option<bool>) {
    let text = span.slice(src);
    for (prefix, checked) in [("[ ] ", false), ("[ ]\t", false), ("[x] ", true), ("[x]\t", true), ("[X] ", true), ("[X]\t", true)] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return (Span::new(span.end - rest.len(), span.end), Some(checked));
        }
    }
    (span, None)
}

/// `lines[*idx]` から、インデントレベルが `level` のリスト項目を連続して読み取る
/// (レベルが `level` 未満になったら親スコープに戻ったとみなして止まる)。各項目の
/// 直後の行がさらに1段深ければ、それを子リスト(D24)として再帰的に読み取る。
fn parse_list_items(
    src: &str,
    lines: &[Span],
    idx: &mut usize,
    diags: &mut Vec<Diag>,
    level: usize,
    refdefs: &RefDefs,
) -> Vec<ListItem> {
    let mut items = Vec::new();
    while *idx < lines.len() {
        let line_span = lines[*idx];
        let indent = leading_space_count(src, line_span);
        let (line_level, aligned) = list_level_of(indent);
        if !aligned {
            if line_level < level {
                break;
            }
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line_span,
                "リスト項目のインデントが2スペース単位で揃っていません",
            ));
            *idx += 1;
            continue;
        }
        if line_level < level {
            break;
        }
        if line_level > level {
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line_span,
                "リスト項目のインデントが深すぎます(対応する親項目がありません)",
            ));
            *idx += 1;
            continue;
        }

        *idx += 1;
        let (text_span, id_tag) = extract_trailing_id_tag(src, line_span, diags);
        // 項目マーカー(`- ` / `N. `、インデント込み)はインライン本文に含めない。
        let text_span = strip_list_marker(src, text_span);
        // M6(D40 Tier2、監査④): タスクリストのチェック状態を項目マーカーの直後から読む。
        let (text_span, checked) = strip_task_marker(src, text_span);
        let inline = crate::inline::parse_inlines(src, text_span, diags, refdefs);

        let mut child = None;
        if *idx < lines.len() {
            let next_indent = leading_space_count(src, lines[*idx]);
            let (next_level, next_aligned) = list_level_of(next_indent);
            if next_aligned && next_level == level + 1 {
                let ordered = line_marker_is_ordered(src, lines[*idx]);
                let start = if ordered {
                    list_marker_start_value_of(src, lines[*idx])
                } else {
                    None
                };
                let nested = parse_list_items(src, lines, idx, diags, level + 1, refdefs);
                child = Some(Box::new(ListBlock { ordered, items: nested, start }));
            }
        }

        items.push(ListItem { span: line_span, inline, id_tag, child, checked });
    }
    items
}

/// M6(D40): ネストしたリストの先頭項目行から順序リストの開始値を読む(トップレベル
/// リストの `RawKind::List.start` と同じ規則、`scan::list_marker_start_value` に委譲)。
fn list_marker_start_value_of(src: &str, span: Span) -> Option<u64> {
    let stripped = span.slice(src).trim_start_matches(' ');
    crate::scan::list_marker_start_value(stripped)
}

/// id タグ(宣言側: `{#...}` / フロントマターの `id:`)専用。ULID か人間ラベルかだけを
/// 区別する — 宣言側は doc 修飾(`/`)を受理しない(D42: doc 修飾は参照側のみの構文)。
pub(crate) fn parse_ref_target(token: &str) -> RefTarget {
    match token.parse::<Ulid>() {
        Ok(u) => RefTarget::Ulid(u),
        Err(_) => RefTarget::Label(token.to_string()),
    }
}

/// 参照側(`ref:` セル/record 値)専用。ULID / `<文書alias>/<ブロックalias>`
/// (D41/D42)/ 無修飾ラベルを振り分ける。`inline.rs::resolve_target` の
/// スキーム参照(`[text](ref:...)` 等)と同じ分割規則だが、こちらは診断を積まない
/// (このパースの既存呼び出し元 `value.rs` がもともと字句検証をしていなかった
/// 挙動を維持する裁量。最終報告参照)。
pub(crate) fn parse_scoped_ref_target(token: &str) -> RefTarget {
    if let Ok(u) = token.parse::<Ulid>() {
        return RefTarget::Ulid(u);
    }
    if let Some((doc, alias)) = token.split_once('/')
        && !doc.is_empty()
        && !alias.is_empty()
        && !alias.contains('/')
    {
        return RefTarget::DocLabel { doc: doc.to_string(), alias: alias.to_string() };
    }
    RefTarget::Label(token.to_string())
}

struct ParsedIdTagInner {
    tag: IdTag,
    id_part_span: Span,
    alias_span: Option<Span>,
}

/// `{#...}` の内側(`inner_span` で示される範囲)をパースする。
fn parse_id_tag_inner(src: &str, inner_span: Span) -> ParsedIdTagInner {
    let inner_text = inner_span.slice(src);
    const MARKER: &str = " alias=";
    if let Some(idx) = inner_text.find(MARKER) {
        let id_part = inner_text[..idx].trim();
        let alias_part = inner_text[idx + MARKER.len()..].trim();
        let id_part_span = Span::new(inner_span.start, inner_span.start + idx);
        let alias_span = Span::new(inner_span.start + idx + MARKER.len(), inner_span.end);
        ParsedIdTagInner {
            tag: IdTag {
                id: parse_ref_target(id_part),
                alias: Some(alias_part.to_string()),
                inner_span,
            },
            id_part_span,
            alias_span: Some(alias_span),
        }
    } else {
        let id_part = inner_text.trim();
        ParsedIdTagInner {
            tag: IdTag { id: parse_ref_target(id_part), alias: None, inner_span },
            id_part_span: inner_span,
            alias_span: None,
        }
    }
}

/// 行末の `{#...}` タグを抽出する。見つかれば `(タグ手前までのテキストspan, タグ)` を
/// 返す。「行末」は末尾の空白を許容する(sml-parser-m1-handoff.md の裁量事項)。
fn extract_trailing_id_tag(src: &str, line_span: Span, diags: &mut Vec<Diag>) -> (Span, Option<IdTag>) {
    let text = line_span.slice(src);
    let trimmed = text.trim_end();
    if !trimmed.ends_with('}') {
        return (line_span, None);
    }
    let Some(rel_open) = trimmed.rfind("{#") else {
        return (line_span, None);
    };
    let before_ok = rel_open == 0 || matches!(trimmed.as_bytes()[rel_open - 1], b' ' | b'\t');
    if !before_ok {
        return (line_span, None);
    }

    let abs_open = line_span.start + rel_open;
    let abs_close = line_span.start + trimmed.len() - 1; // '}' の位置
    let inner_span = Span::new(abs_open + 2, abs_close);
    let parsed = parse_id_tag_inner(src, inner_span);

    if let RefTarget::Label(label) = &parsed.tag.id {
        if !is_valid_key_charset(label) {
            diags.push(Diag::new(
                DiagKind::BadKeyCharset,
                parsed.id_part_span,
                format!("id ラベル '{label}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
            ));
        }
        // sml-spec §3.1(2026-07-13 裁定): alias を書けるのは ULID の id だけ。
        // ドラフトでは `{#label}` とだけ書き、fmt がラベルを alias へ昇格させる。
        // ここで弾かないと fmt が既存 alias を静かに破棄する経路が生まれる。
        if parsed.tag.alias.is_some() {
            diags.push(Diag::new(
                DiagKind::AliasWithoutUlid,
                inner_span,
                format!("非 ULID の id '{label}' に alias は併記できません(ドラフトでは {{#{label}}} とだけ書いてください)"),
            ));
        }
    }
    if let Some(alias) = &parsed.tag.alias
        && !is_valid_key_charset(alias)
    {
        diags.push(Diag::new(
            DiagKind::BadKeyCharset,
            parsed.alias_span.unwrap_or(inner_span),
            format!("alias '{alias}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
        ));
    }

    // タグ手前の空白を除いたテキスト範囲。
    let bytes = src.as_bytes();
    let mut trim_pos = abs_open;
    while trim_pos > line_span.start && matches!(bytes[trim_pos - 1], b' ' | b'\t') {
        trim_pos -= 1;
    }
    let text_span = Span::new(line_span.start, trim_pos);
    (text_span, Some(parsed.tag))
}

/// 属性行(`[key=value, ...]`)1行をパースする。`line_span` は `[` から `]` までを
/// 含む(前後に空白があってもよい)。
fn parse_attr_line(src: &str, line_span: Span, diags: &mut Vec<Diag>) -> AttrLine {
    let text = line_span.slice(src);
    let leading_ws = text.len() - text.trim_start().len();
    let trimmed = text.trim();

    let open_abs = line_span.start + leading_ws;
    let close_abs = open_abs + trimmed.len() - 1; // ']' の位置
    let inner_span = Span::new(open_abs + 1, close_abs);

    let entries = parse_attr_entries(src, inner_span, diags);
    AttrLine { span: line_span, entries }
}

/// `[...]` の内側をトップレベルのカンマで分割し、各エントリを `key=value` として解釈する。
/// 引用符 `"..."` の中と、リスト値 `[...]` のネストの中のカンマ/角括弧は無視する。
fn parse_attr_entries(src: &str, inner_span: Span, diags: &mut Vec<Diag>) -> Vec<(String, AttrValue, Span)> {
    let bytes = src.as_bytes();
    let mut entries = Vec::new();
    let mut i = inner_span.start;
    let end = inner_span.end;

    while i < end {
        while i < end && matches!(bytes[i], b' ' | b'\t' | b',') {
            i += 1;
        }
        if i >= end {
            break;
        }
        let entry_start = i;
        let mut depth = 0i32;
        let mut in_quotes = false;
        let mut j = i;
        while j < end {
            let b = bytes[j];
            if in_quotes {
                if b == b'"' {
                    in_quotes = false;
                }
            } else {
                match b {
                    b'"' => in_quotes = true,
                    b'[' => depth += 1,
                    b']' => depth -= 1,
                    b',' if depth <= 0 => break,
                    _ => {}
                }
            }
            j += 1;
        }
        let entry_end = j;
        let entry_span = Span::new(entry_start, entry_end);
        let entry_text = &src[entry_start..entry_end];

        if let Some(eq_idx) = entry_text.find('=') {
            let key_raw = entry_text[..eq_idx].trim();
            let value_raw = entry_text[eq_idx + 1..].trim();
            if !is_valid_key_charset(key_raw) {
                diags.push(Diag::new(
                    DiagKind::BadKeyCharset,
                    entry_span,
                    format!("属性キー '{key_raw}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
                ));
            }
            entries.push((key_raw.to_string(), parse_attr_value(value_raw), entry_span));
        } else {
            // "=" の無い不正なエントリ。パーサは止まらず、キー欄に丸ごと入れて続行する。
            entries.push((entry_text.trim().to_string(), AttrValue::Single(String::new()), entry_span));
        }

        i = entry_end;
        if i < end && bytes[i] == b',' {
            i += 1;
        }
    }

    entries
}

fn parse_attr_value(value_raw: &str) -> AttrValue {
    if value_raw.len() >= 2 && value_raw.starts_with('"') && value_raw.ends_with('"') {
        return AttrValue::Quoted(value_raw[1..value_raw.len() - 1].to_string());
    }
    if value_raw.len() >= 2 && value_raw.starts_with('[') && value_raw.ends_with(']') {
        let inner = &value_raw[1..value_raw.len() - 1];
        let items: Vec<String> = inner
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
                    s[1..s.len() - 1].to_string()
                } else {
                    s.to_string()
                }
            })
            .collect();
        return AttrValue::List(items);
    }
    AttrValue::Single(value_raw.to_string())
}

/// フェンス本体の先頭にある属性行を読み取り、`(fence_attrs, 残りの本体スパン)` を返す。
///
/// - `::figure` は本体が属性行のみで完結する(sml-spec §6.3)ため、空行を挟みつつ
///   本体全体を属性行として読み切る
/// - `::table` / `::math` / `::record`(D28)は先頭の連続属性行(間の空行は許容)
///   だけを読み、それ以降は本体としてそのまま次の層(table.rs / MathTex span /
///   record.rs)に渡す
fn split_fence_attrs(
    src: &str,
    body_span: Span,
    fence_kind: FenceKind,
    diags: &mut Vec<Diag>,
) -> (Vec<AttrLine>, Span) {
    let lines = split_lines_range(src, body_span);

    if matches!(fence_kind, FenceKind::Figure) {
        let mut attrs = Vec::new();
        for line in &lines {
            if looks_like_attr_line(src, line.content) {
                attrs.push(parse_attr_line(src, line.content, diags));
            }
            // 空行、および仕様上想定されない行は寛容に読み飛ばす(figure 本体は
            // 属性行のみが仕様。パーサは止まらず続行する)。
        }
        return (attrs, Span::new(body_span.end, body_span.end));
    }

    let mut attrs = Vec::new();
    let mut last_consumed_full_end = body_span.start;
    let mut idx = 0;
    loop {
        let mut j = idx;
        while j < lines.len() && lines[j].content.slice(src).trim().is_empty() {
            j += 1;
        }
        if j < lines.len() && looks_like_attr_line(src, lines[j].content) {
            attrs.push(parse_attr_line(src, lines[j].content, diags));
            last_consumed_full_end = lines[j].full.end;
            idx = j + 1;
        } else {
            break;
        }
    }
    (attrs, Span::new(last_consumed_full_end, body_span.end))
}

/// フェンス内属性行から `date-format=` を読み取る(D29、sml-spec §1.5)。`::table` /
/// `::record` の両方が使う共通ロジック。未対応の値は `BadDateFormat` を積んで
/// `None`(既定の ISO のみへフォールバック — 診断は出すが処理は止めない)。
/// `date-format` キーが複数回宣言されていた場合は最初の1件だけを採用する
/// (裁量: フロントマターの「後勝ち」とは逆だが、後続の重複は単に無視するだけで
/// 診断はしない — 後勝ちにする実益が薄いための簡易化)。
fn extract_date_format(fence_attrs: &[AttrLine], diags: &mut Vec<Diag>) -> Option<DateFormat> {
    for al in fence_attrs {
        for (key, value, span) in &al.entries {
            if key != "date-format" {
                continue;
            }
            let raw = match value {
                AttrValue::Quoted(s) => s.clone(),
                AttrValue::Single(s) => s.clone(),
                AttrValue::List(_) => {
                    diags.push(Diag::new(
                        DiagKind::BadDateFormat,
                        *span,
                        "date-format はリスト値にできません".to_string(),
                    ));
                    return None;
                }
            };
            return match crate::value::parse_date_format(&raw) {
                Some(fmt) => Some(fmt),
                None => {
                    diags.push(Diag::new(
                        DiagKind::BadDateFormat,
                        *span,
                        format!(
                            "未対応の date-format '{raw}' です(対応: \"YYYY年M月\" / \"YYYY年M月D日\")"
                        ),
                    ));
                    None
                }
            };
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scan::scan_from;

    fn parse(src: &str) -> (Vec<SmlBlock>, Vec<Diag>) {
        let mut diags = Vec::new();
        let raw = scan_from(src, 0, &mut diags);
        let blocks = build_blocks(src, raw, &mut diags);
        (blocks, diags)
    }

    #[test]
    fn heading_with_no_id() {
        let (blocks, diags) = parse("# Title\n");
        assert!(diags.is_empty());
        match &blocks[0].kind {
            BlockKind::Heading { id_tag, .. } => assert!(id_tag.is_none()),
            _ => panic!("expected heading"),
        }
    }

    /// 見出しのインライン本文に行頭マーカー(`## `)が混入しないこと。
    #[test]
    fn heading_marker_stripped_from_inline() {
        let src = "## 見出しテスト {#my-label}\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Heading { level, inline, .. } => {
                assert_eq!(*level, 2);
                match &inline[0] {
                    crate::ast::SmlInline::Text(sp) => {
                        assert_eq!(sp.slice(src), "見出しテスト");
                    }
                    other => panic!("expected text, got {other:?}"),
                }
            }
            _ => panic!("expected heading"),
        }
    }

    /// リスト項目のインライン本文に項目マーカー(`- ` / `N. `)が混入しないこと。
    #[test]
    fn list_marker_stripped_from_inline() {
        for (src, expected) in [
            ("- 項目テキスト\n", "項目テキスト"),
            ("1. 番号付き項目\n", "番号付き項目"),
        ] {
            let (blocks, diags) = parse(src);
            assert!(diags.is_empty(), "{diags:?}");
            match &blocks[0].kind {
                BlockKind::List { items, .. } => match &items[0].inline[0] {
                    crate::ast::SmlInline::Text(sp) => {
                        assert_eq!(sp.slice(src), expected);
                    }
                    other => panic!("expected text, got {other:?}"),
                },
                _ => panic!("expected list"),
            }
        }
    }

    /// sml-spec §6: フェンス内属性行に id を書くと IdNotAllowedHere。
    #[test]
    fn fence_internal_attr_id_not_allowed() {
        let src = "::math {#my-formula}\n[id=other-id]\nx^2\n::\n";
        let (_, diags) = parse(src);
        assert!(
            diags.iter().any(|d| d.kind == DiagKind::IdNotAllowedHere),
            "{diags:?}"
        );
    }

    #[test]
    fn heading_with_ulid_tag() {
        let ulid = Ulid::new().to_string();
        let src = format!("# Title {{#{ulid}}}\n");
        let (blocks, diags) = parse(&src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Heading { id_tag: Some(tag), .. } => {
                assert!(matches!(tag.id, RefTarget::Ulid(_)));
                assert!(tag.alias.is_none());
            }
            other => panic!("expected heading with id_tag, got {other:?}"),
        }
    }

    #[test]
    fn heading_with_label_tag() {
        let (blocks, diags) = parse("# Title {#my-label}\n");
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Heading { id_tag: Some(tag), .. } => {
                assert_eq!(tag.id, RefTarget::Label("my-label".to_string()));
            }
            other => panic!("expected heading with id_tag, got {other:?}"),
        }
    }

    #[test]
    fn heading_with_ulid_and_alias() {
        let ulid = Ulid::new().to_string();
        let src = format!("# Title {{#{ulid} alias=my-label}}\n");
        let (blocks, diags) = parse(&src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Heading { id_tag: Some(tag), .. } => {
                assert!(matches!(tag.id, RefTarget::Ulid(_)));
                assert_eq!(tag.alias.as_deref(), Some("my-label"));
            }
            other => panic!("expected heading with id_tag, got {other:?}"),
        }
    }

    #[test]
    fn list_item_id_tag_forms() {
        let src = "- one {#item-one}\n- two\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::List { items, .. } => {
                assert_eq!(items[0].id_tag.as_ref().unwrap().id, RefTarget::Label("item-one".into()));
                assert!(items[1].id_tag.is_none());
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn fence_marker_id_tag_forms() {
        let src = "::table {#eval-table}\n@rows:\n::\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Fence(fb) => {
                assert_eq!(fb.id_tag.as_ref().unwrap().id, RefTarget::Label("eval-table".into()));
            }
            _ => panic!("expected fence"),
        }
    }

    #[test]
    fn attr_line_single_value() {
        let (blocks, diags) = parse("[supports=eval-table]\nParagraph.\n");
        assert!(diags.is_empty());
        let entries = &blocks[0].attrs.as_ref().unwrap().entries;
        assert_eq!(entries[0].0, "supports");
        assert_eq!(entries[0].1, AttrValue::Single("eval-table".to_string()));
    }

    #[test]
    fn attr_line_list_value() {
        let (blocks, diags) = parse("[supports=[claim-1, claim-2]]\nParagraph.\n");
        assert!(diags.is_empty());
        let entries = &blocks[0].attrs.as_ref().unwrap().entries;
        assert_eq!(entries[0].1, AttrValue::List(vec!["claim-1".to_string(), "claim-2".to_string()]));
    }

    #[test]
    fn attr_line_quoted_value() {
        // D17: ブロック前置属性行のキーは既知のもの(supports 等)を使う。`caption` は
        // フェンス内属性行専用の語彙であり、ここで使うと `UnknownAttrKey`(Warning)が
        // 発生してこのテストの主眼(引用符付き値のパース)とは無関係な診断が混ざる。
        let (blocks, diags) = parse("[cites=\"a b c\"]\nParagraph.\n");
        assert!(diags.is_empty());
        let entries = &blocks[0].attrs.as_ref().unwrap().entries;
        assert_eq!(entries[0].1, AttrValue::Quoted("a b c".to_string()));
    }

    #[test]
    fn attr_line_multiple_entries() {
        let (blocks, diags) = parse("[id=foo, supports=bar]\nParagraph.\n");
        assert!(diags.is_empty());
        let entries = &blocks[0].attrs.as_ref().unwrap().entries;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "id");
        assert_eq!(entries[1].0, "supports");
    }

    #[test]
    fn orphan_attr_line_diag() {
        let (_, diags) = parse("[id=foo]\n\nParagraph.\n");
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].kind, DiagKind::OrphanAttrLine);
    }

    #[test]
    fn duplicate_id_on_heading() {
        let (_, diags) = parse("[id=foo]\n# Title {#bar}\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::DuplicateId), "{diags:?}");
    }

    #[test]
    fn bad_key_charset_on_attr_key() {
        let (_, diags) = parse("[bad key=1]\nParagraph.\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{diags:?}");
    }

    /// sml-spec §3.1(2026-07-13 裁定): 非 ULID の id に alias は併記できない。
    #[test]
    fn alias_on_label_id_tag_is_diagnosed() {
        for src in [
            "# Title {#my-label alias=other}\n",
            "- item {#item-label alias=x}\n",
            "::table {#tbl alias=x}\n@rows:\n  - a: [b]\n::\n",
        ] {
            let (_, diags) = parse(src);
            assert!(
                diags.iter().any(|d| d.kind == DiagKind::AliasWithoutUlid),
                "expected AliasWithoutUlid for {src:?}, got {diags:?}"
            );
        }
    }

    /// ULID の id + alias は正当(3.1 の正規形)。AliasWithoutUlid を誤発火しないこと。
    #[test]
    fn alias_on_ulid_id_tag_is_not_diagnosed() {
        let (_, diags) = parse("# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=my-label}\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    /// sml-spec §3.2(2026-07-13 裁定): 属性行の id 値は裸トークンのみ。
    #[test]
    fn quoted_or_list_id_value_is_diagnosed() {
        for src in ["[id=\"quoted\"]\nParagraph.\n", "[id=[a, b]]\nParagraph.\n"] {
            let (_, diags) = parse(src);
            assert!(
                diags.iter().any(|d| d.kind == DiagKind::BadIdValue),
                "expected BadIdValue for {src:?}, got {diags:?}"
            );
        }
    }

    /// 属性行の id 値が裸トークンでも字句不正なら BadKeyCharset(ULID は無条件で正当)。
    #[test]
    fn attr_id_label_charset_is_validated() {
        let (_, diags) = parse("[id=bad.label]\nParagraph.\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{diags:?}");

        let (_, diags) = parse("[id=01ARZ3NDEKTSV4RRFFQ69G5FAV]\nParagraph.\n");
        assert!(diags.is_empty(), "{diags:?}");
        let (_, diags) = parse("[id=good-label]\nParagraph.\n");
        assert!(diags.is_empty(), "{diags:?}");
    }

    #[test]
    fn bad_key_charset_on_alias() {
        let (_, diags) = parse("# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV alias=bad.alias}\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{diags:?}");
    }

    #[test]
    fn fence_attrs_and_remaining_body_split() {
        let src = "::table {#t}\n[caption=\"c\"]\n\n@rows:\n  - a: [b]\n::\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Fence(fb) => {
                assert_eq!(fb.fence_attrs.len(), 1);
                match &fb.body {
                    FenceBody::Table(_) => {}
                    other => panic!("expected table body, got {other:?}"),
                }
            }
            _ => panic!("expected fence"),
        }
    }

    // ---- D28: `::record` フェンスの block 層配線 -----------------------------------

    #[test]
    fn record_fence_marker_id_tag_forms() {
        let src = "::record {#basic-info}\n姓: 山田\n::\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Fence(fb) => {
                assert_eq!(fb.fence_kind, crate::ast::FenceKind::Record);
                assert_eq!(fb.id_tag.as_ref().unwrap().id, RefTarget::Label("basic-info".into()));
                match &fb.body {
                    crate::ast::FenceBody::Record(rb) => {
                        assert_eq!(rb.entries.len(), 1);
                        assert_eq!(rb.entries[0].key, "姓");
                    }
                    other => panic!("expected record body, got {other:?}"),
                }
            }
            other => panic!("expected fence, got {other:?}"),
        }
    }

    /// D29: フェンス属性 `date-format=` が record の値パースに配線されていること
    /// (extract_date_format 経由。value.rs / record.rs のテストは date_format を
    /// 直接渡しているため、ここで attr 文字列からの配線を別途確認する)。
    #[test]
    fn record_date_format_attr_is_wired_into_value_parsing() {
        let src = "::record\n[date-format=\"YYYY年M月\"]\n在籍期間: 2020年10月\n::\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Fence(fb) => match &fb.body {
                crate::ast::FenceBody::Record(rb) => {
                    assert_eq!(
                        rb.entries[0].value,
                        crate::ast::CellRaw::Date(crate::ast::DateRaw { y: 2020, m: 10, d: None })
                    );
                }
                other => panic!("expected record body, got {other:?}"),
            },
            other => panic!("expected fence, got {other:?}"),
        }
    }

    /// D29: 未対応の `date-format=` 値は `BadDateFormat`(処理は止めず None にフォールバック)。
    #[test]
    fn unsupported_date_format_value_is_diagnosed() {
        let src = "::record\n[date-format=\"MM/DD/YYYY\"]\n日付: 何か\n::\n";
        let (_, diags) = parse(src);
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadDateFormat), "{diags:?}");
    }

    /// table フェンスにも `date-format=` が配線されること(D29「表セルと record 値で共通」)。
    #[test]
    fn table_date_format_attr_is_wired_into_cell_parsing() {
        let src = "::table\n[date-format=\"YYYY年M月D日\"]\n@cells:\n  a | b : 2020年10月1日\n::\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Fence(fb) => match &fb.body {
                crate::ast::FenceBody::Table(tb) => {
                    assert_eq!(
                        tb.cells[0].value,
                        crate::ast::CellRaw::Date(crate::ast::DateRaw { y: 2020, m: 10, d: Some(1) })
                    );
                }
                other => panic!("expected table body, got {other:?}"),
            },
            other => panic!("expected fence, got {other:?}"),
        }
    }

    #[test]
    fn figure_body_is_attrs_only() {
        let src = "::figure {#f}\n[kind=chart]\n[caption=\"c\"]\n::\n";
        let (blocks, diags) = parse(src);
        assert!(diags.is_empty(), "{diags:?}");
        match &blocks[0].kind {
            BlockKind::Fence(fb) => {
                assert_eq!(fb.fence_attrs.len(), 2);
                assert!(matches!(fb.body, FenceBody::Figure));
            }
            _ => panic!("expected fence"),
        }
    }
}
