//! Pass 1: エイリアス解決の第1パス(sml-build-m3-handoff.md D-B5)。
//!
//! 全ブロック(+リスト項目)を走査し、「ULID 登録」と「alias → ULID 表」を構築する。
//! ここで収集した情報を Pass 2(`convert.rs`)が消費する。
//!
//! - alias の重複 → `DuplicateAlias`(全件収集)
//! - ULID 未付与(`RefTarget::Label` の id、または ID を持たないブロック)→ `MissingId`
//!
//! ブロックの Span をキーに `Registration` を引けるようにしておくことで、Pass 2 は
//! 同じ木を辿るだけで(再解決せずに)各ブロック自身の NodeId と種別を取得できる。

use std::collections::HashMap;

use strata_sml::{AttrLine, AttrValue, BlockKind, IdTag, RefTarget, SmlDocument, Span};
use strata_core::NodeId;
use ulid::Ulid;

use crate::error::BuildError;

/// canonical 上でこのブロックがどのノード型になるか(scheme 検証用の粗い分類)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NodeKindTag {
    Document,
    Section,
    Para,
    List,
    Table,
    Math,
    Figure,
    Code,
    /// `::record` フェンス(D28、sml-spec §1.5)。
    Record,
    /// blockquote(M6 D40)。
    Quote,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Registration {
    pub id: NodeId,
}

/// doc alias → (ブロック alias → NodeId)。ワークスペース横断参照(D42)の解決に
///使う、build ごとに使い捨てのインメモリ index(「store/index 分離」の index 側)。
/// 各文書の `Registry::alias_table` をそのまま複製したものを、文書の frontmatter
/// alias をキーにまとめる。単一ファイル build ではこの index は無い
/// (`None` → `CrossDocRef`)。
pub(crate) type CrossDocIndex = HashMap<String, HashMap<String, NodeId>>;

pub(crate) struct Registry {
    /// ブロック(またはリスト項目)の Span → 解決済み(またはプレースホルダの)登録情報。
    pub by_span: HashMap<Span, Registration>,
    /// alias → NodeId。重複がある場合は最初の定義を採用する(重複自体は別途
    /// `DuplicateAlias` として報告されるので、どちらを採用しても最終結果は Err)。
    pub alias_table: HashMap<String, NodeId>,
    /// NodeId → そのノード自身に定義された alias(D26、sml-spec §1.5)。1つの ID には
    /// 定義箇所が1つしか無い(alias はその ID を持つブロック自身の `{#ULID alias=x}` /
    /// `[id=ULID, alias=x]` からしか生まれない)ため、alias_table(alias→id、複数
    /// alias が同じ id を指しうる)とは逆向きの1:1マップになる。Pass 2(`convert.rs`)が
    /// `Node.alias` へそのまま写す。
    pub id_alias: HashMap<NodeId, String>,
    /// NodeId → 種別。scheme(table:/fig:/math:/cell:)の対象ノード型検証に使う。
    pub node_kind: HashMap<NodeId, NodeKindTag>,
    /// フロントマターが有効な ULID の id を持つ場合のみ `Some`。
    pub document_id: Option<NodeId>,
}

pub(crate) fn build_registry(doc: &SmlDocument, errors: &mut Vec<BuildError>) -> Registry {
    let mut reg = Registry {
        by_span: HashMap::new(),
        alias_table: HashMap::new(),
        id_alias: HashMap::new(),
        node_kind: HashMap::new(),
        document_id: None,
    };
    let mut alias_defs: HashMap<String, Vec<Span>> = HashMap::new();

    if let Some(fm) = &doc.frontmatter {
        match &fm.id {
            Some((RefTarget::Ulid(u), _)) => {
                let id = NodeId(*u);
                reg.document_id = Some(id);
                reg.node_kind.insert(id, NodeKindTag::Document);
                // D41: フロントマターの `alias:` を Document ノードの alias として登録する
                // (D26 の Node.alias に乗る、block レベルの alias と同じ表・同じ重複検出
                // 経路を共有する)。
                if let Some((alias, alias_span)) = &fm.alias {
                    alias_defs.entry(alias.clone()).or_default().push(*alias_span);
                    reg.alias_table.entry(alias.clone()).or_insert(id);
                    reg.id_alias.insert(id, alias.clone());
                }
            }
            _ => {
                // `RefTarget::Label` はフロントマターパーサが既に `BadIdValue` を積んで
                // いるが、build 視点では「ULID 未付与」でもあるため MissingId も積む。
                // id 自体が無いケースも同様(fmt が常に注入するはずの id が無い)。
                errors.push(BuildError::MissingId { span: fm.open_span });
            }
        }
    }

    for block in &doc.blocks {
        register_block(block, &mut reg, &mut alias_defs, errors);
    }

    for (alias, spans) in alias_defs {
        if spans.len() > 1 {
            errors.push(BuildError::DuplicateAlias { alias, spans });
        }
    }

    reg
}

fn register_block(
    block: &strata_sml::SmlBlock,
    reg: &mut Registry,
    alias_defs: &mut HashMap<String, Vec<Span>>,
    errors: &mut Vec<BuildError>,
) {
    match &block.kind {
        BlockKind::Heading { id_tag, .. } => {
            register_line_type(block.span, id_tag, NodeKindTag::Section, reg, alias_defs, errors);
        }
        BlockKind::Paragraph { .. } => {
            register_prose(block.span, &block.attrs, NodeKindTag::Para, reg, alias_defs, errors);
        }
        BlockKind::List { items, .. } => {
            register_prose(block.span, &block.attrs, NodeKindTag::List, reg, alias_defs, errors);
            register_list_items(items, reg, alias_defs, errors);
        }
        BlockKind::Fence(fb) => {
            let kind = match fb.fence_kind {
                strata_sml::FenceKind::Table => NodeKindTag::Table,
                strata_sml::FenceKind::Math => NodeKindTag::Math,
                strata_sml::FenceKind::Figure => NodeKindTag::Figure,
                strata_sml::FenceKind::Record => NodeKindTag::Record,
            };
            register_line_type(block.span, &fb.id_tag, kind, reg, alias_defs, errors);
        }
        BlockKind::CodeFence { id_tag, .. } => {
            register_line_type(block.span, id_tag, NodeKindTag::Code, reg, alias_defs, errors);
        }
        // M6(D40): blockquote はプローズ扱い(前置属性行で id)。中身のブロックも
        // 再帰的に登録する。GFM 表もプローズ扱い(Table ノードへブリッジ)。
        BlockKind::Quote { blocks } => {
            register_prose(block.span, &block.attrs, NodeKindTag::Quote, reg, alias_defs, errors);
            for inner in blocks {
                register_block(inner, reg, alias_defs, errors);
            }
        }
        BlockKind::GfmTable(_) => {
            register_prose(block.span, &block.attrs, NodeKindTag::Table, reg, alias_defs, errors);
        }
        // M6(D40): 参照リンク定義行は非可視メタ(ノードを作らない)。水平線は fmt が
        // ID を注入しない(SML 上に ID の置き場が無い)ため、ここでは登録せず Pass 2
        // (convert.rs)が文書内位置から決定的に ID を導出する(裁量、最終報告参照)。
        BlockKind::LinkRefDef { .. } | BlockKind::ThematicBreak => {}
    }
}

/// D24: リスト項目列を文書順に走査し、各項目(Para 扱い)→ その子リスト(あれば
/// 再帰)の順で登録する。ネストした子リスト自体は SML 上に ID を書く場所が無い
/// (前置属性行を置けない)ため、ここでは登録しない — 子 List ノードの ID は
/// Pass 2(convert.rs)が build ごとに自動生成する(裁量。最終報告に記載)。
fn register_list_items(
    items: &[strata_sml::ListItem],
    reg: &mut Registry,
    alias_defs: &mut HashMap<String, Vec<Span>>,
    errors: &mut Vec<BuildError>,
) {
    for item in items {
        register_line_type(item.span, &item.id_tag, NodeKindTag::Para, reg, alias_defs, errors);
        if let Some(child) = &item.child {
            register_list_items(&child.items, reg, alias_defs, errors);
        }
    }
}

/// 行型ブロック(見出し・フェンス・コードフェンス・リスト項目)の ID 登録。
fn register_line_type(
    span_for_missing: Span,
    id_tag: &Option<IdTag>,
    kind: NodeKindTag,
    reg: &mut Registry,
    alias_defs: &mut HashMap<String, Vec<Span>>,
    errors: &mut Vec<BuildError>,
) {
    match id_tag {
        Some(tag) => match &tag.id {
            RefTarget::Ulid(u) => {
                let id = NodeId(*u);
                reg.by_span.insert(span_for_missing, Registration { id });
                reg.node_kind.insert(id, kind);
                if let Some(alias) = &tag.alias {
                    alias_defs.entry(alias.clone()).or_default().push(tag.inner_span);
                    reg.alias_table.entry(alias.clone()).or_insert(id);
                    reg.id_alias.insert(id, alias.clone());
                }
            }
            // id タグ(宣言側)は doc 修飾を生成しない(`parse_ref_target` の契約、
            // sml-sml/src/block.rs)。到達すれば防御的に MissingId 同様に扱う。
            RefTarget::Label(_) | RefTarget::DocLabel { .. } => {
                errors.push(BuildError::MissingId { span: tag.inner_span });
                reg.by_span.insert(span_for_missing, placeholder());
            }
        },
        None => {
            errors.push(BuildError::MissingId { span: span_for_missing });
            reg.by_span.insert(span_for_missing, placeholder());
        }
    }
}

/// プローズブロック(段落・リスト全体)の ID 登録。id/alias は前置属性行から読む。
fn register_prose(
    span: Span,
    attrs: &Option<AttrLine>,
    kind: NodeKindTag,
    reg: &mut Registry,
    alias_defs: &mut HashMap<String, Vec<Span>>,
    errors: &mut Vec<BuildError>,
) {
    let id_entry = attrs.as_ref().and_then(|al| al.entries.iter().find(|(k, _, _)| k == "id"));

    let resolved = match id_entry {
        Some((_, AttrValue::Single(s), entry_span)) => match s.parse::<Ulid>() {
            Ok(u) => {
                let id = NodeId(u);
                reg.node_kind.insert(id, kind);
                Some(id)
            }
            Err(_) => {
                // 非 ULID ラベル(ドラフト段階)。fmt 未実行の合図。
                errors.push(BuildError::MissingId { span: *entry_span });
                None
            }
        },
        // `[id="..."]` / `[id=[a, b]]` は既にパーサが `BadIdValue` を積んでいる
        // (`BuildError::Parse` として収集済み)。ここで MissingId を重ねて出すと
        // 同じ問題に対して2種類のエラーが出て紛らわしいので、追加の診断はしない。
        Some((_, AttrValue::Quoted(_) | AttrValue::List(_), _)) => None,
        None => {
            errors.push(BuildError::MissingId { span });
            None
        }
    };

    let id = resolved.unwrap_or_else(NodeId::new);
    reg.by_span.insert(span, Registration { id });

    if resolved.is_some()
        && let Some((_, AttrValue::Single(alias), alias_span)) =
            attrs.as_ref().and_then(|al| al.entries.iter().find(|(k, _, _)| k == "alias"))
    {
        alias_defs.entry(alias.clone()).or_default().push(*alias_span);
        reg.alias_table.entry(alias.clone()).or_insert(id);
        reg.id_alias.insert(id, alias.clone());
    }
}

fn placeholder() -> Registration {
    Registration { id: NodeId::new() }
}
