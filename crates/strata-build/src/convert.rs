//! Pass 2: ノード構築とエッジの materialise(sml-build-m3-handoff.md D-B5)。
//!
//! Pass 1(`resolve.rs`)が構築した `Registry` を消費し、SML-AST を
//! `strata_core::Graph` へ写す。参照ターゲットの解決は「ULID ならそのまま /
//! ラベルなら alias 表 → 無ければ `UnresolvedAlias`」(2パス方式の第2パス)。

use std::collections::{BTreeMap, HashMap};

use strata_sml::{
    AttrLine, AttrValue, BlockKind, CellEntry, CellRaw, DimNode, FenceBlock, FenceBody, MemberNode, RefScheme,
    RefTarget, SmlBlock, SmlDocument, SmlInline, Span,
};
use strata_sml::EmphKind as SmlEmphKind;

use strata_core::{
    Cell, CellValue, Chart, Code, Dim, Document, EmphKind as CoreEmphKind, Encoding, Figure, Graph, ImageFigure,
    Inline, List as CoreList, Mark, MathBlock, Member, Node, NodeId, NodePayload, Para, Rel, Section, Table, Term,
};
use strata_core::CellCoord as CoreCellCoord;
use ulid::Ulid;

use crate::error::BuildError;
use crate::resolve::{NodeKindTag, Registration, Registry};
use crate::{math, term};

struct Builder<'a> {
    src: &'a str,
    reg: Registry,
    graph: Graph,
    term_ids: HashMap<String, NodeId>,
    errors: Vec<BuildError>,
    ord: HashMap<NodeId, u32>,
}

pub(crate) fn run(src: &str, doc: &SmlDocument, reg: Registry) -> (Graph, Option<NodeId>, Vec<BuildError>) {
    let mut b = Builder { src, reg, graph: Graph::default(), term_ids: HashMap::new(), errors: Vec::new(), ord: HashMap::new() };

    let root = b.reg.document_id;
    if let Some(doc_id) = root {
        let title = doc.frontmatter.as_ref().and_then(|fm| fm.title.clone());
        b.graph.insert(Node::new(doc_id, NodePayload::Document(Document { title })));
    }

    let mut stack: Vec<(u8, NodeId)> = Vec::new();
    for block in &doc.blocks {
        b.build_block(block, &mut stack, root);
    }

    (b.graph, root, b.errors)
}

fn current_parent(stack: &[(u8, NodeId)], root: Option<NodeId>) -> Option<NodeId> {
    stack.last().map(|&(_, id)| id).or(root)
}

impl<'a> Builder<'a> {
    fn add_contains(&mut self, parent: NodeId, child: NodeId) {
        let counter = self.ord.entry(parent).or_insert(0);
        let ord = *counter;
        *counter += 1;
        self.graph.link(parent, Rel::Contains, child, Some(ord));
    }

    fn build_block(&mut self, block: &SmlBlock, stack: &mut Vec<(u8, NodeId)>, root: Option<NodeId>) {
        let Registration { id, .. } = self.reg.by_span[&block.span];

        match &block.kind {
            BlockKind::Heading { level, inline, .. } => {
                while let Some(&(lvl, _)) = stack.last() {
                    if lvl >= *level {
                        stack.pop();
                    } else {
                        break;
                    }
                }
                let parent = current_parent(stack, root);
                let heading = self.convert_inlines(id, inline);
                self.graph.insert(Node::new(id, NodePayload::Section(Section { heading })));
                if let Some(p) = parent {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
                stack.push((*level, id));
            }
            BlockKind::Paragraph { inline } => {
                let inline = self.convert_inlines(id, inline);
                self.graph.insert(Node::new(id, NodePayload::Para(Para { inline })));
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
            }
            BlockKind::List { ordered, items } => {
                self.graph.insert(Node::new(id, NodePayload::List(CoreList { ordered: *ordered })));
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
                self.build_list_items(id, items);
            }
            BlockKind::Fence(fb) => {
                match fb.fence_kind {
                    strata_sml::FenceKind::Table => self.build_table(id, fb),
                    strata_sml::FenceKind::Math => self.build_math(id, fb),
                    strata_sml::FenceKind::Figure => self.build_figure(id, fb, block.span),
                }
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
            }
            BlockKind::CodeFence { lang, body, .. } => {
                let src = body.slice(self.src).to_string();
                self.graph.insert(Node::new(id, NodePayload::Code(Code { lang: lang.clone(), src })));
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
            }
        }
    }

    /// D24: リスト項目列を canonical へ写す。各項目は Para ノードとして親 List に
    /// contains され(既存の平坦リストのグラフ表現と同一)、項目が子リストを持てば
    /// 新しい List ノードを**親項目(Para)の子**として contains で接続し再帰する。
    /// ネストした List ノード自体は SML 上に ID を書く場所が無いため、build が
    /// その場で ID を自動生成する(build ごとに変わる。裁量 — 最終報告に記載)。
    fn build_list_items(&mut self, parent: NodeId, items: &[strata_sml::ListItem]) {
        for item in items {
            let Registration { id: item_id, .. } = self.reg.by_span[&item.span];
            let inline = self.convert_inlines(item_id, &item.inline);
            self.graph.insert(Node::new(item_id, NodePayload::Para(Para { inline })));
            self.add_contains(parent, item_id);
            if let Some(child) = &item.child {
                let child_list_id = NodeId::new();
                self.graph
                    .insert(Node::new(child_list_id, NodePayload::List(CoreList { ordered: child.ordered })));
                self.add_contains(item_id, child_list_id);
                self.build_list_items(child_list_id, &child.items);
            }
        }
    }

    /// ブロック前置属性行の意味エッジ(supports/depends-on/cites、sml-spec §4.1)と
    /// class タグ(D23)を materialise する。`id`/`alias` キーは Pass 1 が既に消費済み
    /// なのでここでは無視、カタログに無いキーも(将来拡張の前方互換のため)黙って無視する。
    fn apply_block_attrs(&mut self, from: NodeId, attrs: &Option<AttrLine>) {
        let Some(al) = attrs else { return };
        for (key, value, span) in &al.entries {
            match key.as_str() {
                "supports" | "depends-on" | "cites" => {
                    let rel = match key.as_str() {
                        "supports" => Rel::Supports,
                        "depends-on" => Rel::DependsOn,
                        "cites" => Rel::Cites,
                        _ => unreachable!("外側の match で3キーに限定済み"),
                    };
                    for token in attr_value_tokens(value) {
                        let to = self.resolve_attr_target(&token, *span);
                        self.graph.link(from, rel, to, None);
                    }
                }
                "class" => self.apply_class_attr(from, value, *span),
                _ => {}
            }
        }
    }

    /// class タグ(D23、sml-spec §1.4)を検証して Node.classes に格納する。
    /// 値は単一または `[a, b]` リスト、字句は key と同じ `[A-Za-z0-9_-]+`。
    /// build の成否・グラフ構造は class に非依存(全ノードを常に格納する)ため、
    /// 字句違反があってもこのブロック自体の構築は続ける — `errors` に積むだけ
    /// (最終的に全か無かで弾かれるのは呼び出し元の `build` の責任)。
    fn apply_class_attr(&mut self, from: NodeId, value: &AttrValue, span: Span) {
        for token in attr_value_tokens(value) {
            if is_valid_class_tag(&token) {
                if let Some(node) = self.graph.nodes.get_mut(&from)
                    && !node.classes.contains(&token)
                {
                    node.classes.push(token);
                }
            } else {
                self.errors.push(BuildError::BadClass {
                    span,
                    msg: format!("class '{token}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
                });
            }
        }
    }

    /// `ULID / エイリアス / term:<用語名>` トークンを解決する(属性行の値用)。
    fn resolve_attr_target(&mut self, token: &str, span: Span) -> NodeId {
        if let Some(rest) = token.strip_prefix("term:") {
            if let Ok(u) = rest.parse::<Ulid>() {
                return NodeId(u);
            }
            return self.term_id(rest);
        }
        if let Ok(u) = token.parse::<Ulid>() {
            return NodeId(u);
        }
        if let Some(&id) = self.reg.alias_table.get(token) {
            return id;
        }
        self.errors.push(BuildError::UnresolvedAlias { alias: token.to_string(), span });
        NodeId::new()
    }

    /// `RefTarget`(インライン参照・セル参照)の解決。ULID ならそのまま、ラベルなら
    /// alias 表を引く(2パス方式の第2パス、sml-build-m3-handoff.md D-B5)。
    fn resolve_ref_target(&mut self, target: &RefTarget, span: Span) -> NodeId {
        match target {
            RefTarget::Ulid(u) => NodeId(*u),
            RefTarget::Label(l) => {
                if let Some(&id) = self.reg.alias_table.get(l) {
                    id
                } else {
                    self.errors.push(BuildError::UnresolvedAlias { alias: l.clone(), span });
                    NodeId::new()
                }
            }
        }
    }

    fn term_id(&mut self, name: &str) -> NodeId {
        if let Some(&id) = self.term_ids.get(name) {
            return id;
        }
        let id = term::derive_term_id(name);
        self.term_ids.insert(name.to_string(), id);
        if !self.graph.nodes.contains_key(&id) {
            self.graph.insert(Node::new(id, NodePayload::Term(Term { name: name.to_string() })));
        }
        id
    }

    fn convert_inlines(&mut self, from: NodeId, inlines: &[SmlInline]) -> Vec<Inline> {
        inlines.iter().map(|i| self.convert_inline(from, i)).collect()
    }

    fn convert_inline(&mut self, from: NodeId, inline: &SmlInline) -> Inline {
        match inline {
            SmlInline::Text(span) => Inline::Text { s: span.slice(self.src).to_string() },
            SmlInline::Emph { kind, children } => {
                Inline::Emph { kind: convert_emph_kind(*kind), children: self.convert_inlines(from, children) }
            }
            SmlInline::MathTex(span) => {
                let tex = span.slice(self.src);
                match tex2math::parse_normalized(tex) {
                    Ok(tree) => Inline::Math { tree: math::convert(tree) },
                    Err(e) => {
                        self.errors.push(BuildError::Math { span: *span, msg: format!("{e:?}") });
                        // 全か無かにより最終結果は破棄されるが、後続処理をパニックさせない
                        // ためのプレースホルダ(sml-build-m3-handoff.md D-B1)。
                        Inline::Text { s: tex.to_string() }
                    }
                }
            }
            SmlInline::Ref { scheme, target, coord, text } => {
                let text_str = text.slice(self.src).to_string();
                let to = self.resolve_ref_target(target, *text);
                self.validate_scheme(*scheme, to, *text);
                let coord = coord.as_ref().map(|c| CoreCellCoord { row_path: c.row_path.clone(), col_path: c.col_path.clone() });
                self.graph.link(from, Rel::RefersTo, to, None);
                Inline::Ref { to, rel: Rel::RefersTo, coord, text: text_str }
            }
            SmlInline::TermRef { name_or_id, text } => {
                let text_str = text.slice(self.src).to_string();
                let to = match name_or_id {
                    RefTarget::Ulid(u) => NodeId(*u),
                    RefTarget::Label(name) => self.term_id(name),
                };
                self.graph.link(from, Rel::TermRef, to, None);
                Inline::Term { to, text: text_str }
            }
        }
    }

    /// scheme(table:/fig:/math:/cell:)の対象ノード型を検証する。target が未知
    /// (`UnresolvedAlias` 済み、またはファイル外の ULID)なら検証をスキップする
    /// (dangling は build 後の `invariants::validate` が別途検出する)。
    fn validate_scheme(&mut self, scheme: RefScheme, to: NodeId, span: Span) {
        let expected: Option<NodeKindTag> = match scheme {
            RefScheme::Ref => None,
            RefScheme::Table => Some(NodeKindTag::Table),
            RefScheme::Fig => Some(NodeKindTag::Figure),
            RefScheme::Math => Some(NodeKindTag::Math),
            RefScheme::Cell => Some(NodeKindTag::Table),
        };
        let Some(expected) = expected else { return };
        let Some(&actual) = self.reg.node_kind.get(&to) else { return };
        if actual != expected {
            self.errors.push(BuildError::RefTypeMismatch {
                span,
                msg: format!("参照スキームが対象ノードの型と一致しません(期待: {expected:?}, 実際: {actual:?})"),
            });
        }
    }

    fn build_table(&mut self, id: NodeId, fb: &FenceBlock) {
        let FenceBody::Table(tb) = &fb.body else { unreachable!("Table フェンスは常に Table body") };
        let rows = self.convert_dims(&tb.rows);
        let cols = self.convert_dims(&tb.cols);
        let cells = tb.cells.iter().map(|c| self.convert_cell(c)).collect();
        // フェンス内の `[caption=...]` を strata_core::Table.caption へ写す(D16)。
        let entries: Vec<&(String, AttrValue, Span)> = fb.fence_attrs.iter().flat_map(|al| al.entries.iter()).collect();
        let caption = find_entry(&entries, "caption").map(|(_, v, _)| vec![Inline::Text { s: attr_value_string(v) }]);
        self.graph.insert(Node::new(id, NodePayload::Table(Table { rows, cols, cells, caption })));
    }

    fn convert_dims(&self, dims: &[DimNode]) -> Vec<Dim> {
        dims.iter().map(|d| Dim { name: d.name.clone(), members: self.convert_members(&d.members) }).collect()
    }

    fn convert_members(&self, members: &[MemberNode]) -> Vec<Member> {
        members
            .iter()
            .map(|m| Member {
                key: m.key.clone(),
                label: m.label.as_ref().map(|l| vec![Inline::Text { s: l.clone() }]),
                children: self.convert_dims(&m.children),
            })
            .collect()
    }

    fn convert_cell(&mut self, c: &CellEntry) -> Cell {
        let value = match &c.value {
            CellRaw::Number(v) => CellValue::Number { v: *v },
            CellRaw::Quantity { v, unit } => CellValue::Quantity { v: *v, unit: unit.clone() },
            CellRaw::Text(s) => CellValue::Text { v: s.clone() },
            CellRaw::Empty => CellValue::Empty,
            CellRaw::Ref(target) => CellValue::Ref { to: self.resolve_ref_target(target, c.span) },
        };
        Cell { row_path: c.row_path.clone(), col_path: c.col_path.clone(), value }
    }

    fn build_math(&mut self, id: NodeId, fb: &FenceBlock) {
        let FenceBody::MathTex(span) = &fb.body else { unreachable!("Math フェンスは常に MathTex body") };
        let tex = span.slice(self.src);
        match tex2math::parse_normalized(tex) {
            Ok(tree) => {
                self.graph.insert(Node::new(id, NodePayload::Math(MathBlock { tree: math::convert(tree) })));
            }
            Err(e) => {
                self.errors.push(BuildError::Math { span: *span, msg: format!("{e:?}") });
                self.graph.insert(Node::new(
                    id,
                    NodePayload::Math(MathBlock { tree: strata_core::MathNode::Text { s: tex.to_string() } }),
                ));
            }
        }
    }

    fn build_figure(&mut self, id: NodeId, fb: &FenceBlock, block_span: Span) {
        let entries: Vec<&(String, AttrValue, Span)> = fb.fence_attrs.iter().flat_map(|al| al.entries.iter()).collect();
        let kind = find_entry(&entries, "kind").map(|(_, v, _)| attr_value_string(v));
        match kind.as_deref() {
            Some("chart") => self.build_chart_figure(id, &entries, block_span),
            Some("image") => self.build_image_figure(id, &entries, block_span),
            Some(other) => self.errors.push(BuildError::BadFigure {
                span: block_span,
                msg: format!("未知の figure kind '{other}' です(chart / image のみ許可)"),
            }),
            None => self.errors.push(BuildError::BadFigure {
                span: block_span,
                msg: "figure に kind 属性がありません(chart / image のいずれか必須)".to_string(),
            }),
        }
    }

    fn build_chart_figure(&mut self, id: NodeId, entries: &[&(String, AttrValue, Span)], block_span: Span) {
        let data_ref = find_entry(entries, "data-ref").map(|(_, v, s)| (attr_value_string(v), *s));
        let mark = find_entry(entries, "mark").map(|(_, v, _)| attr_value_string(v));
        let x = find_entry(entries, "encode-x").map(|(_, v, _)| attr_value_string(v));
        let y = find_entry(entries, "encode-y").map(|(_, v, _)| attr_value_string(v));
        let color = find_entry(entries, "encode-color").map(|(_, v, _)| attr_value_string(v));
        let caption = find_entry(entries, "caption").map(|(_, v, _)| vec![Inline::Text { s: attr_value_string(v) }]);

        let Some((data_ref_str, data_ref_span)) = data_ref else {
            self.errors.push(BuildError::BadFigure { span: block_span, msg: "chart figure に data-ref がありません".to_string() });
            return;
        };
        let Some(mark_str) = mark else {
            self.errors.push(BuildError::BadFigure { span: block_span, msg: "chart figure に mark がありません".to_string() });
            return;
        };
        let Some(mark) = parse_mark(&mark_str) else {
            self.errors.push(BuildError::BadFigure { span: block_span, msg: format!("未知の mark '{mark_str}' です") });
            return;
        };
        let (Some(x), Some(y)) = (x, y) else {
            self.errors.push(BuildError::BadFigure { span: block_span, msg: "chart figure に encode-x / encode-y が必要です".to_string() });
            return;
        };

        let data_ref = self.resolve_attr_target(&data_ref_str, data_ref_span);
        // `[depicts=...]` / `[depicts.<key>=...]` を ImageFigure.depicts と同形で写す(D16)。
        let depicts = fold_depicts(entries);
        self.graph.insert(Node::new(
            id,
            NodePayload::Figure(Figure::Chart(Chart { data_ref, mark, encode: Encoding { x, y, color }, caption, depicts })),
        ));
    }

    fn build_image_figure(&mut self, id: NodeId, entries: &[&(String, AttrValue, Span)], block_span: Span) {
        let src = find_entry(entries, "src").map(|(_, v, _)| attr_value_string(v));
        let alt = find_entry(entries, "alt").map(|(_, v, _)| attr_value_string(v));
        let caption = find_entry(entries, "caption").map(|(_, v, _)| vec![Inline::Text { s: attr_value_string(v) }]);

        let Some(src) = src else {
            self.errors.push(BuildError::BadFigure { span: block_span, msg: "image figure に src がありません".to_string() });
            return;
        };
        let Some(alt) = alt else {
            self.errors.push(BuildError::BadFigure { span: block_span, msg: "image figure に alt がありません".to_string() });
            return;
        };

        let depicts = fold_depicts(entries);

        self.graph.insert(Node::new(id, NodePayload::Figure(Figure::Image(ImageFigure { src, alt, depicts, caption }))));
    }
}

/// `[depicts=...]` / `[depicts.<key>=...]` の畳み規則(sml-spec §6.3、D16)。
/// 裸の `depicts` は `"description"` キー、`depicts.<key>` はその `<key>` をキーにする。
/// `ImageFigure.depicts` と `Chart.depicts`(D16)で共有する。
fn fold_depicts(entries: &[&(String, AttrValue, Span)]) -> BTreeMap<String, String> {
    let mut depicts: BTreeMap<String, String> = BTreeMap::new();
    for (k, v, _) in entries {
        if k == "depicts" {
            depicts.insert("description".to_string(), attr_value_string(v));
        } else if let Some(rest) = k.strip_prefix("depicts.") {
            depicts.insert(rest.to_string(), attr_value_string(v));
        }
    }
    depicts
}

fn convert_emph_kind(k: SmlEmphKind) -> CoreEmphKind {
    match k {
        SmlEmphKind::Strong => CoreEmphKind::Strong,
        SmlEmphKind::Em => CoreEmphKind::Em,
        SmlEmphKind::Code => CoreEmphKind::Code,
    }
}

fn attr_value_tokens(v: &AttrValue) -> Vec<String> {
    match v {
        AttrValue::Single(s) => vec![s.clone()],
        AttrValue::Quoted(s) => vec![s.clone()],
        AttrValue::List(items) => items.clone(),
    }
}

fn attr_value_string(v: &AttrValue) -> String {
    match v {
        AttrValue::Single(s) => s.clone(),
        AttrValue::Quoted(s) => s.clone(),
        AttrValue::List(items) => items.join(", "),
    }
}

fn find_entry<'e>(entries: &[&'e (String, AttrValue, Span)], key: &str) -> Option<&'e (String, AttrValue, Span)> {
    entries.iter().find(|(k, _, _)| k == key).copied()
}

/// class タグの字句制約(D23、sml-spec §7の key 字句と同じ `[A-Za-z0-9_-]+`)。
fn is_valid_class_tag(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse_mark(s: &str) -> Option<Mark> {
    match s {
        "line" => Some(Mark::Line),
        "bar" => Some(Mark::Bar),
        "point" => Some(Mark::Point),
        "area" => Some(Mark::Area),
        _ => None,
    }
}
