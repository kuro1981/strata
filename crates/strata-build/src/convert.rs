//! Pass 2: ノード構築とエッジの materialise(sml-build-m3-handoff.md D-B5)。
//!
//! Pass 1(`resolve.rs`)が構築した `Registry` を消費し、SML-AST を
//! `strata_core::Graph` へ写す。参照ターゲットの解決は「ULID ならそのまま /
//! ラベルなら alias 表 → 無ければ `UnresolvedAlias`」(2パス方式の第2パス)。

use std::collections::{BTreeMap, HashMap};

use strata_sml::{
    AttrLine, AttrValue, BlockKind, CellEntry, CellRaw, DateRaw, DimNode, FenceBlock, FenceBody, MemberNode,
    RecordEntry as SmlRecordEntry, RefScheme, RefTarget, SmlBlock, SmlDocument, SmlInline, Span,
};
use strata_sml::EmphKind as SmlEmphKind;

use strata_core::{
    Cell, CellValue, Chart, Code, DateValue, Dim, Document, EmphKind as CoreEmphKind, Encoding, Figure, Graph,
    ImageFigure, Inline, List as CoreList, Mark, MathBlock, Member, Node, NodeId, NodePayload, Para, Quote,
    Record, RecordEntry, Rel, Section, Table, Term, ThematicBreak,
};
use strata_core::CellCoord as CoreCellCoord;
use ulid::Ulid;

use crate::error::BuildError;
use crate::resolve::{NodeKindTag, Registration, Registry};
use crate::{list_child, math, term};

struct Builder<'a> {
    src: &'a str,
    reg: Registry,
    graph: Graph,
    term_ids: HashMap<String, NodeId>,
    errors: Vec<BuildError>,
    ord: HashMap<NodeId, u32>,
    /// M6(D40): 水平線ノードの決定的 ID 導出用の出現カウンタ(裁量、最終報告参照)。
    hr_count: u32,
}

pub(crate) fn run(src: &str, doc: &SmlDocument, reg: Registry) -> (Graph, Option<NodeId>, Vec<BuildError>) {
    let mut b = Builder {
        src,
        reg,
        graph: Graph::default(),
        term_ids: HashMap::new(),
        errors: Vec::new(),
        ord: HashMap::new(),
        hr_count: 0,
    };

    let root = b.reg.document_id;
    if let Some(doc_id) = root {
        let title = doc.frontmatter.as_ref().and_then(|fm| fm.title.clone());
        b.graph.insert(Node::new(doc_id, NodePayload::Document(Document { title })));
    }

    let mut stack: Vec<(u8, NodeId)> = Vec::new();
    for block in &doc.blocks {
        b.build_block(block, &mut stack, root);
    }

    // D26(2026-07-15): 解決済みエイリアスを graph JSON に出力する(ビューのアドレス
    // 規約の柱)。Pass 1(resolve.rs)が集めた「その ID 自身が定義した alias」を
    // Node.alias へそのまま写す。id_alias は NodeId をキーとする1:1マップなので
    // 全ノード構築後にまとめて適用できる。
    for (id, alias) in &b.reg.id_alias {
        if let Some(node) = b.graph.nodes.get_mut(id) {
            node.alias = Some(alias.clone());
        }
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
        // M6(D40): 参照リンク定義行は非可視メタ — グラフにノードを作らない(監査②4)。
        if matches!(&block.kind, BlockKind::LinkRefDef { .. }) {
            return;
        }
        // M6(D40): 水平線は SML 上に ID の置き場が無いため registry に登録されない。
        // 文書内の出現順から決定的に ID を導出する(D27 の子 List ノードと同型の裁量)。
        if matches!(&block.kind, BlockKind::ThematicBreak) {
            let id = derive_thematic_break_id(root, self.hr_count);
            self.hr_count += 1;
            self.graph.insert(Node::new(id, NodePayload::ThematicBreak(ThematicBreak {})));
            if let Some(p) = current_parent(stack, root) {
                self.add_contains(p, id);
            }
            return;
        }

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
                self.graph.insert(Node::new(id, NodePayload::Para(Para { inline, checked: None })));
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
            }
            BlockKind::List { ordered, items, start } => {
                // M6(D40、監査②5): 順序リストの開始値を保存する。1 は既定なので省略。
                let start = start.filter(|&s| *ordered && s != 1);
                self.graph.insert(Node::new(id, NodePayload::List(CoreList { ordered: *ordered, start })));
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
                self.build_list_items(id, items);
            }
            // M6(D40): blockquote — NodePayload::Quote。中身のブロックは Quote ノードを
            // ルートとする独立の見出しスタックで再帰構築する(引用内の見出しネストが
            // 外側の文書階層へ漏れないように)。
            BlockKind::Quote { blocks } => {
                self.graph.insert(Node::new(id, NodePayload::Quote(Quote {})));
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
                let mut inner_stack: Vec<(u8, NodeId)> = Vec::new();
                for inner in blocks {
                    self.build_block(inner, &mut inner_stack, Some(id));
                }
            }
            // M6(D40 Tier2): GFM パイプ表をフラット2次元の Table ノードへブリッジする。
            BlockKind::GfmTable(tb) => {
                let table = self.bridge_gfm_table(tb);
                self.graph.insert(Node::new(id, NodePayload::Table(table)));
                if let Some(p) = current_parent(stack, root) {
                    self.add_contains(p, id);
                }
                self.apply_block_attrs(id, &block.attrs);
            }
            BlockKind::Fence(fb) => {
                match fb.fence_kind {
                    strata_sml::FenceKind::Table => self.build_table(id, fb),
                    strata_sml::FenceKind::Math => self.build_math(id, fb),
                    strata_sml::FenceKind::Figure => self.build_figure(id, fb, block.span),
                    strata_sml::FenceKind::Record => self.build_record(id, fb),
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
            BlockKind::LinkRefDef { .. } | BlockKind::ThematicBreak => {
                unreachable!("冒頭で早期リターン済み")
            }
        }
    }

    /// M6(D40 Tier2): GFM パイプ表 → フラット2次元 Table。列はヘッダセルから
    /// 1次元(`col`)、行は自動採番の1次元(`row`、key は `r1`〜`rN`)を作る。
    /// 列 member の key はヘッダテキストが key 字句(`[A-Za-z0-9_-]+`)に収まり
    /// 重複しなければそのまま使い(`cell:` 参照で意味のある座標が書ける)、
    /// 収まらなければ `c1`〜`cN` に落として元テキストを label に保持する(裁量、
    /// 最終報告参照)。
    fn bridge_gfm_table(&mut self, tb: &strata_sml::GfmTableBody) -> Table {
        let header_texts: Vec<String> =
            tb.header.iter().map(|s| s.slice(self.src).trim().to_string()).collect();

        let mut col_keys: Vec<String> = Vec::with_capacity(header_texts.len());
        for (i, h) in header_texts.iter().enumerate() {
            let candidate_ok = !h.is_empty()
                && h.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                && !col_keys.contains(h);
            col_keys.push(if candidate_ok { h.clone() } else { format!("c{}", i + 1) });
        }

        let col_members: Vec<Member> = header_texts
            .iter()
            .zip(&col_keys)
            .map(|(h, key)| Member {
                key: key.clone(),
                label: if h == key { None } else { Some(vec![Inline::Text { s: h.clone() }]) },
                children: vec![],
            })
            .collect();

        let row_keys: Vec<String> = (1..=tb.rows.len()).map(|i| format!("r{i}")).collect();
        let row_members: Vec<Member> =
            row_keys.iter().map(|k| Member { key: k.clone(), label: None, children: vec![] }).collect();

        let mut cells = Vec::new();
        for (r, row) in tb.rows.iter().enumerate() {
            for (c, raw) in row.iter().enumerate() {
                if matches!(raw, CellRaw::Empty) {
                    continue;
                }
                // GFM 表セルには座標スパンの概念が無い(型付きパースは層1で完了済みで
                // Ref も解決不要な値が大半)ため、診断位置には空スパンを使う。
                let value = self.convert_cell_raw(raw, Span::new(0, 0));
                cells.push(Cell {
                    row_path: vec![row_keys[r].clone()],
                    col_path: vec![col_keys[c].clone()],
                    value,
                });
            }
        }

        Table {
            rows: vec![Dim { name: "row".to_string(), members: row_members }],
            cols: vec![Dim { name: "col".to_string(), members: col_members }],
            cells,
            caption: None,
        }
    }

    /// D24: リスト項目列を canonical へ写す。各項目は Para ノードとして親 List に
    /// contains され(既存の平坦リストのグラフ表現と同一)、項目が子リストを持てば
    /// 新しい List ノードを**親項目(Para)の子**として contains で接続し再帰する。
    /// ネストした List ノード自体は SML 上に ID を書く場所が無いため、build が
    /// 親項目の ULID + 位置から決定的に導出する(D27、`list_child::derive_list_child_id`。
    /// D9 の Term 安定 ID と同型)。同一入力を2回 build しても全ノード ID が一致する。
    fn build_list_items(&mut self, parent: NodeId, items: &[strata_sml::ListItem]) {
        for item in items {
            let Registration { id: item_id, .. } = self.reg.by_span[&item.span];
            let inline = self.convert_inlines(item_id, &item.inline);
            // M6(D40 Tier2): タスクリストのチェック状態を項目 Para に写す。
            self.graph.insert(Node::new(item_id, NodePayload::Para(Para { inline, checked: item.checked })));
            self.add_contains(parent, item_id);
            if let Some(child) = &item.child {
                // D24 時点では1項目につき子リストは高々1つなので position は常に 0
                // (list_child.rs のドキュメント参照)。
                let child_list_id = list_child::derive_list_child_id(item_id, 0);
                let start = child.start.filter(|&s| child.ordered && s != 1);
                self.graph
                    .insert(Node::new(child_list_id, NodePayload::List(CoreList { ordered: child.ordered, start })));
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

    /// M6(D40): エスケープ(`\*` → リテラル `*`)の unescape はここ(グラフ側)で
    /// 行う — ソース(SML テキスト)は fmt のバイト保存契約によりバックスラッシュ込みで
    /// 保存されたまま(監査②1)。unescape の結果、隣接する Text が細切れになるため、
    /// 連続する `Inline::Text` はここで1つに結合する。
    fn convert_inlines(&mut self, from: NodeId, inlines: &[SmlInline]) -> Vec<Inline> {
        let mut out: Vec<Inline> = Vec::with_capacity(inlines.len());
        for i in inlines {
            let conv = self.convert_inline(from, i);
            if let (Some(Inline::Text { s: prev }), Inline::Text { s }) = (out.last_mut(), &conv) {
                prev.push_str(s);
            } else {
                out.push(conv);
            }
        }
        out
    }

    fn convert_inline(&mut self, from: NodeId, inline: &SmlInline) -> Inline {
        match inline {
            SmlInline::Text(span) => Inline::Text { s: span.slice(self.src).to_string() },
            // M6(D40、監査②1): `\X` はグラフ上では unescape 済みのリテラル `X`。
            SmlInline::Escaped(span) => {
                let escaped = span.slice(self.src);
                Inline::Text { s: escaped[1..].to_string() }
            }
            // M6(D40、監査②2): 外部リンク/画像。Edge は張らない(意味エッジ対象外)。
            SmlInline::Link { url, text } => Inline::Link {
                url: url.slice(self.src).to_string(),
                text: text.slice(self.src).to_string(),
            },
            SmlInline::Image { url, alt } => Inline::Image {
                url: url.slice(self.src).to_string(),
                alt: alt.slice(self.src).to_string(),
            },
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
        let value = self.convert_cell_raw(&c.value, c.span);
        Cell { row_path: c.row_path.clone(), col_path: c.col_path.clone(), value }
    }

    /// `CellRaw` → `CellValue` の型付き変換(D4 + D29)。`::table` の `@cells` と
    /// `::record` の本体で共有する(sml-spec §1.5「表セルと record 値で共通」)。
    fn convert_cell_raw(&mut self, v: &CellRaw, span: Span) -> CellValue {
        match v {
            CellRaw::Number(v) => CellValue::Number { v: *v },
            CellRaw::Quantity { v, unit } => CellValue::Quantity { v: *v, unit: unit.clone() },
            CellRaw::Text(s) => CellValue::Text { v: s.clone() },
            CellRaw::Empty => CellValue::Empty,
            CellRaw::Ref(target) => CellValue::Ref { to: self.resolve_ref_target(target, span) },
            CellRaw::Date(d) => CellValue::Date(convert_date(d)),
            CellRaw::Period { from, to } => {
                CellValue::Period { from: convert_date(from), to: to.as_ref().map(convert_date) }
            }
        }
    }

    /// D28: `::record` の本体をグラフへ格納する。フェンス内属性(`date-format=` 等)は
    /// 値パース時点(strata-sml 層)で既に消費済みなので、ここでは順序保存列を
    /// そのまま canonical の `Record` へ写すだけでよい。
    fn build_record(&mut self, id: NodeId, fb: &FenceBlock) {
        let FenceBody::Record(rb) = &fb.body else { unreachable!("Record フェンスは常に Record body") };
        let entries: Vec<RecordEntry> = rb
            .entries
            .iter()
            .map(|e: &SmlRecordEntry| RecordEntry { key: e.key.clone(), value: self.convert_cell_raw(&e.value, e.span) })
            .collect();
        self.graph.insert(Node::new(id, NodePayload::Record(Record { entries })));
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

/// `DateRaw`(strata-sml、層1)→ `DateValue`(strata-core、canonical)の直写し変換(D29)。
fn convert_date(d: &DateRaw) -> DateValue {
    DateValue { y: d.y, m: d.m, d: d.d }
}

fn convert_emph_kind(k: SmlEmphKind) -> CoreEmphKind {
    match k {
        SmlEmphKind::Strong => CoreEmphKind::Strong,
        SmlEmphKind::Em => CoreEmphKind::Em,
        SmlEmphKind::Code => CoreEmphKind::Code,
        SmlEmphKind::Strike => CoreEmphKind::Strike,
    }
}

/// M6(D40): 水平線ノードの決定的 ID 導出(裁量)。SML 上に ID の置き場が無いため、
/// D27(子 List ノード)と同型に「文書ルート ID(無ければ nil)+ 文書内の出現順」
/// から Sha256 で導出する。同一入力の再 build で ID が一致する(ID 安定)。
fn derive_thematic_break_id(root: Option<NodeId>, index: u32) -> NodeId {
    use sha2::{Digest, Sha256};
    let ns = root.map(|r| r.0).unwrap_or_else(Ulid::nil);
    let mut hasher = Sha256::new();
    hasher.update(format!("strata:thematic-break:v0:{ns}:{index}"));
    let hash = hasher.finalize();
    let bytes: [u8; 16] = hash[..16].try_into().expect("SHA-256 digest is 32 bytes");
    NodeId(Ulid(u128::from_be_bytes(bytes)))
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
