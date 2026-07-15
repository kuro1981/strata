//! strata-md — canonical グラフ(層2)→ 素の Markdown(GFM)のレンダラ(D38、WP-M2)。
//!
//! `render --format md` の実装本体。strata-typst(一次レンダラ)と並ぶ二次レンダラで、
//! 人間向けの最小依存ビュー(GitHub・チャット・エディタプレビューでそのまま読める)を
//! 目的とする。strata-context(AI 向けコンテキストビュー)とは役割が違う:
//! **`{#ULID}` タグ・alias・意味エッジは一切出さない**(D38)。
//!
//! 実装は strata-typst の流儀(`RenderOutput` / `--hide` サブツリー非描画 / warnings)を
//! そのまま踏襲する。strata-context の inline 変換モジュールは公開されておらず
//! (`mod inline;` は private)、かつ役割が違う(context の `Ref` は常にリンク無しの
//! 短いラベル表示。md は見出しへは実際に GFM アンカーリンクを張り、それ以外は
//! テキスト退化という別ポリシー)ため、共有はせず本クレートで独立に実装した
//! (最終報告に理由を明記)。

use std::collections::{HashMap, HashSet};

use strata_core::{
    CellCoord, CellValue, Chart, DateValue, DimTree, EmphKind, Figure, Graph, ImageFigure, Inline, List, Mark,
    MathNode, NodeId, NodePayload, Record, Scalar, Table, Term,
};

/// `render --format md --hide <class>` の結果。strata-typst::RenderOutput と同形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderOutput {
    pub text: String,
    pub warnings: Vec<String>,
}

/// `--hide` を使わない既定経路。内部は `render_to_md_with_hide` に `hide: &[]` で
/// 委譲するだけ(strata-typst::render_to_typst と同じ形)。
///
/// `fallback_title` はシグネチャの対称性のために受け取るが、Markdown の本文には
/// 使わない(裁量、最終報告参照): GFM には Typst の `#set document(title: ...)` に
/// 相当する「本文と独立したメタタイトル」の置き場が無く、素の Markdown では最初の
/// `#` 見出しがそのまま人間にとっての「タイトル」であるため、二重表示を避けて
/// 本文(`Document` の子ノード列)だけを描画する。
pub fn render_to_md(graph: &Graph, root: NodeId, fallback_title: &str) -> Result<String, String> {
    render_to_md_with_hide(graph, root, fallback_title, &[]).map(|out| out.text)
}

/// D23: `render --format md --hide <class>` の本体。strata-typst と同じサブツリー
/// 非描画+隠し `Ref` の Warning 化(HiddenRef)を行う。
pub fn render_to_md_with_hide(
    graph: &Graph,
    root: NodeId,
    fallback_title: &str,
    hide: &[String],
) -> Result<RenderOutput, String> {
    render_to_md_workspace_with_hide(graph, root, fallback_title, hide, None)
}

/// D44(sml-spec.md §1.11): `render --workspace --format md` の本体。単一文書版
/// (`render_to_md_with_hide`)に、クロスドキュメント `Ref` の**相対 .md リンク+
/// アンカー**化(見出し対象)/文書名付き退化テキスト化(それ以外)を足したもの。
/// `workspace` が `None` なら単一文書モードと完全に同じ経路になる(後方互換)。
pub fn render_to_md_workspace_with_hide(
    graph: &Graph,
    root: NodeId,
    fallback_title: &str,
    hide: &[String],
    workspace: Option<WorkspaceRenderCtx<'_>>,
) -> Result<RenderOutput, String> {
    let _ = fallback_title; // 裁量: MD 本文にはタイトルを別出ししない(上記ドキュメント参照)。
    let hide_set: HashSet<&str> = hide.iter().map(String::as_str).collect();
    let hidden = compute_hidden(graph, &hide_set);
    let mut renderer = MdRenderer::new(graph, hidden, workspace);
    let text = renderer.render_root(root)?;
    Ok(RenderOutput { text, warnings: renderer.warnings })
}

/// D44: ワークスペース全体の見出しアンカーを1回だけ事前計算する(`render_to_md_with_hide`
/// が単一文書内で行う `collect_anchors` と同じスラグ規則を、文書ごとに独立した
/// スラグ表として全メンバーぶん積む)。クロスドキュメント `Ref` が他文書の見出しへ
/// 実リンクを張るために必要(D44: MD のページ間リンクは相対 `.md` リンク+アンカー)。
/// `--hide` は考慮しない(非表示ノードへの参照は `is_hidden` 分岐が先に処理し、この表を
/// 参照しないため実害無し)。
pub fn compute_workspace_anchors(graph: &Graph, doc_roots: &[NodeId]) -> HashMap<NodeId, String> {
    let mut anchors = HashMap::new();
    for &root in doc_roots {
        let Some(node) = graph.nodes.get(&root) else { continue };
        let body_roots: Vec<NodeId> =
            if matches!(node.payload, NodePayload::Document(_)) { graph.children_of(root) } else { vec![root] };

        fn walk(graph: &Graph, id: NodeId, out: &mut Vec<NodeId>) {
            out.push(id);
            for child in graph.children_of(id) {
                walk(graph, child, out);
            }
        }
        let mut ordered_ids = Vec::new();
        for &id in &body_roots {
            walk(graph, id, &mut ordered_ids);
        }

        let mut table = AnchorTable::default();
        for id in ordered_ids {
            if let Some(NodePayload::Section(s)) = graph.nodes.get(&id).map(|n| &n.payload) {
                let slug = table.assign(&plain_text(&s.heading));
                anchors.insert(id, slug);
            }
        }
    }
    anchors
}

/// D44: `render --workspace --format md` 専用のクロスドキュメント描画コンテキスト。
/// 単一文書モード(`render_to_md_with_hide`)では使わない。
#[derive(Clone, Copy)]
pub struct WorkspaceRenderCtx<'a> {
    /// ノード → 所属文書(`strata_build::DocRoot::path`)。`strata_build::doc_ownership`
    /// が作る表をそのまま渡す想定。
    pub doc_of: &'a HashMap<NodeId, String>,
    /// 文書(`DocRoot::path`)→ 出力ファイル名 stem(相対 `.md` リンクの構築に使う。
    /// D44: 出力ファイル名はメンバーのファイル名 stem 由来)。
    pub doc_stems: &'a HashMap<String, String>,
    /// 文書(`DocRoot::path`)→ 退化テキストに添える表示名。
    pub doc_titles: &'a HashMap<String, String>,
    /// 今まさに描画している文書の `DocRoot::path`。
    pub current_doc: &'a str,
    /// `compute_workspace_anchors` が事前計算した、見出しノード→所属文書内アンカー。
    pub cross_anchors: &'a HashMap<NodeId, String>,
}

/// class(D23/D46)による非表示化の対象ノード集合。D46 で定義を strata-core
/// (`effective_classes`/`has_effective_class`)に一本化した(strata-typst::compute_hidden
/// と同じロジック — 「実効 class(自身+祖先の和集合)が `hide` のいずれかを含む」)。
fn compute_hidden(graph: &Graph, hide: &HashSet<&str>) -> HashSet<NodeId> {
    if hide.is_empty() {
        return HashSet::new();
    }
    let parents = strata_core::parent_index(graph);
    graph
        .nodes
        .keys()
        .copied()
        .filter(|&id| strata_core::has_effective_class(graph, &parents, id, hide))
        .collect()
}

struct MdRenderer<'a> {
    graph: &'a Graph,
    hidden: HashSet<NodeId>,
    warnings: Vec<String>,
    /// 見出し(Section)→ GFM アンカースラグ の事前計算表(D38: 見出しへの Ref は
    /// アンカーリンク)。前方参照(まだ描画していない見出しへの Ref)にも対応するため、
    /// 本文を描画する前に文書順で1回だけ全見出しを走査して確定させる(GitHub の
    /// スラガーと同じ「重複は -1, -2 を付番」規則、`AnchorTable` 参照)。
    anchors: HashMap<NodeId, String>,
    /// D44: `Some` ならワークスペースモード(クロスドキュメント Ref の相対リンク化)。
    workspace: Option<WorkspaceRenderCtx<'a>>,
}

impl<'a> MdRenderer<'a> {
    fn new(graph: &'a Graph, hidden: HashSet<NodeId>, workspace: Option<WorkspaceRenderCtx<'a>>) -> Self {
        Self { graph, hidden, warnings: Vec::new(), anchors: HashMap::new(), workspace }
    }

    fn is_hidden(&self, id: NodeId) -> bool {
        self.hidden.contains(&id)
    }

    fn render_root(&mut self, root: NodeId) -> Result<String, String> {
        let node =
            self.graph.nodes.get(&root).ok_or_else(|| format!("Node not found in graph: {:?}", root))?;

        let body_roots: Vec<NodeId> = if matches!(node.payload, NodePayload::Document(_)) {
            self.graph.children_of(root)
        } else {
            vec![root]
        };

        self.collect_anchors(&body_roots);

        let mut out = String::new();
        for id in body_roots {
            if self.is_hidden(id) {
                continue;
            }
            out.push_str(&self.render_node(id, 1)?);
        }
        Ok(out)
    }

    /// D38: 見出しの GFM アンカーを本文描画前に確定させる事前パス(前方参照に対応するため)。
    fn collect_anchors(&mut self, body_roots: &[NodeId]) {
        let mut table = AnchorTable::default();
        let mut stack: Vec<NodeId> = body_roots.iter().rev().copied().collect();
        // 文書順(深さ優先)を保つため、子は逆順に積んで pop で取り出す。
        let mut ordered_ids: Vec<NodeId> = Vec::new();
        fn walk(graph: &Graph, id: NodeId, hidden: &HashSet<NodeId>, out: &mut Vec<NodeId>) {
            if hidden.contains(&id) {
                return;
            }
            out.push(id);
            for child in graph.children_of(id) {
                walk(graph, child, hidden, out);
            }
        }
        for &id in body_roots {
            walk(self.graph, id, &self.hidden, &mut ordered_ids);
        }
        stack.clear();
        for id in ordered_ids {
            if let Some(NodePayload::Section(s)) = self.graph.nodes.get(&id).map(|n| &n.payload) {
                let text = plain_text(&s.heading);
                let slug = table.assign(&text);
                self.anchors.insert(id, slug);
            }
        }
    }

    fn render_node(&mut self, node_id: NodeId, depth: usize) -> Result<String, String> {
        let node =
            self.graph.nodes.get(&node_id).ok_or_else(|| format!("Node not found in graph: {:?}", node_id))?;

        match &node.payload {
            NodePayload::Section(s) => {
                let heading_md = self.render_inlines(&s.heading)?;
                let prefix = "#".repeat(depth.clamp(1, 6));

                let mut children_md = String::new();
                for child_id in self.graph.children_of(node_id) {
                    if self.is_hidden(child_id) {
                        continue;
                    }
                    children_md.push_str(&self.render_node(child_id, depth + 1)?);
                }

                Ok(format!("{prefix} {heading_md}\n\n{children_md}"))
            }
            NodePayload::Para(p) => {
                let inline_md = self.render_inlines(&p.inline)?;
                Ok(format!("{inline_md}\n\n"))
            }
            NodePayload::List(l) => self.render_list(l, node_id),
            NodePayload::Table(t) => self.render_table(t),
            NodePayload::Record(r) => self.render_record(r),
            NodePayload::Math(m) => {
                let tex = to_tex(&m.tree);
                Ok(format!("$$\n{tex}\n$$\n\n"))
            }
            NodePayload::Code(c) => Ok(format!("```{}\n{}\n```\n\n", c.lang, ensure_trailing_newline(&c.src))),
            NodePayload::Figure(f) => self.render_figure(f),
            NodePayload::Term(t) => Ok(md_escape(&t.name)),
            NodePayload::Value(v) => Ok(scalar_text(&v.scalar, v.unit.as_deref())),
            NodePayload::Anchor(a) => {
                // D38 裁量: 素の GFM にはブロック外の任意アンカーの記法が無い(`{#id}` は
                // 禁止語彙)。中身をそのまま透過的にインライン展開する(視覚上は
                // 段落内に自然に溶け込む)。
                self.render_inlines(&a.inline)
            }
            NodePayload::Document(_) => Ok(String::new()),
            NodePayload::Quote(_) => {
                let mut children_md = String::new();
                for child_id in self.graph.children_of(node_id) {
                    if self.is_hidden(child_id) {
                        continue;
                    }
                    children_md.push_str(&self.render_node(child_id, depth)?);
                }
                Ok(format!("{}\n\n", quote_prefix(children_md.trim_end())))
            }
            NodePayload::ThematicBreak(_) => Ok("---\n\n".to_string()),
        }
    }

    fn render_list(&mut self, l: &List, node_id: NodeId) -> Result<String, String> {
        let body = self.render_list_items(l, node_id, 0)?;
        Ok(format!("{body}\n"))
    }

    fn render_list_items(&mut self, l: &List, list_id: NodeId, indent: usize) -> Result<String, String> {
        let pad = "  ".repeat(indent);
        let mut out = String::new();
        let mut next_number = l.start.filter(|_| l.ordered).unwrap_or(1);

        for child_id in self.graph.children_of(list_id) {
            if self.is_hidden(child_id) {
                continue;
            }
            let marker = if l.ordered { format!("{next_number}.") } else { "-".to_string() };
            next_number += 1;

            let child_node = self
                .graph
                .nodes
                .get(&child_id)
                .ok_or_else(|| format!("Child node not found: {:?}", child_id))?;

            match &child_node.payload {
                NodePayload::Para(p) => {
                    let content = self.render_inlines(&p.inline)?;
                    let check = match p.checked {
                        Some(true) => "[x] ",
                        Some(false) => "[ ] ",
                        None => "",
                    };
                    out.push_str(&format!("{pad}{marker} {check}{content}\n"));
                    for sub_id in self.graph.children_of(child_id) {
                        if self.is_hidden(sub_id) {
                            continue;
                        }
                        if let Some(NodePayload::List(sub_list)) =
                            self.graph.nodes.get(&sub_id).map(|n| &n.payload)
                        {
                            let sub_list = sub_list.clone();
                            out.push_str(&self.render_list_items(&sub_list, sub_id, indent + 1)?);
                        }
                    }
                }
                _ => {
                    let content = self.render_node(child_id, 1)?;
                    let content = content.trim_end();
                    out.push_str(&format!("{pad}{marker} {content}\n"));
                }
            }
        }

        Ok(out)
    }

    fn render_inlines(&mut self, inlines: &[Inline]) -> Result<String, String> {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text { s } => out.push_str(&md_escape(s)),
                // インラインコード(EmphKind::Code)は中身を再エスケープしない: CommonMark の
                // コードスパンはバックスラッシュエスケープを一切処理しない生テキストであり
                // (strata-sml::inline の "code の中はネスト不可" と同じ前提)、ここで
                // `md_escape` を適用すると余計なバックスラッシュが再パース時に literal な
                // 文字として増えてしまい round-trip が壊れる(裁量、最終報告参照)。
                Inline::Emph { kind: EmphKind::Code, children } => {
                    out.push_str(&format!("`{}`", plain_text(children)));
                }
                Inline::Emph { kind, children } => {
                    let inner = self.render_inlines(children)?;
                    match kind {
                        EmphKind::Strong => out.push_str(&format!("**{inner}**")),
                        EmphKind::Em => out.push_str(&format!("_{inner}_")),
                        EmphKind::Strike => out.push_str(&format!("~~{inner}~~")),
                        EmphKind::Code => unreachable!("Code は上のアームで処理済み"),
                    }
                }
                Inline::Math { tree } => out.push_str(&format!("${}$", to_tex(tree))),
                Inline::Ref { to, text, coord, .. } => out.push_str(&self.render_ref(*to, text, coord.as_ref())),
                Inline::Term { to, text } => out.push_str(&self.render_term(*to, text)),
                Inline::Anchor { to } => {
                    if let Some(NodePayload::Anchor(a)) = self.graph.nodes.get(to).map(|n| &n.payload) {
                        out.push_str(&self.render_inlines(&a.inline)?);
                    }
                }
                Inline::Link { url, text } => {
                    if text == url {
                        out.push_str(&format!("<{url}>"));
                    } else {
                        out.push_str(&format!("[{}]({})", md_escape(text), url));
                    }
                }
                Inline::Image { url, alt } => {
                    out.push_str(&format!("![{}]({})", md_escape(alt), url));
                }
            }
        }
        Ok(out)
    }

    /// D38: `Ref` の描画。見出し(Section)への参照だけが GFM アンカーリンクになり、
    /// それ以外は「テキスト退化」(`(種別: キャプション)` 形。表示 `text` があれば
    /// 先頭に残し、ロスレス原則(黙って落とさない)を守る)。
    fn render_ref(&mut self, to: NodeId, text: &str, coord: Option<&CellCoord>) -> String {
        let coord_suffix = coord.map(format_coord).unwrap_or_default();

        if self.is_hidden(to) {
            self.warnings.push(format!(
                "-:-: warning: HiddenRef: 参照先 {} は --hide により非表示です。リンクを外しプレーンテキスト化しました。",
                to.0
            ));
            return if !text.is_empty() {
                format!("{}{}", md_escape(text), coord_suffix)
            } else {
                format!("(非表示){}", coord_suffix)
            };
        }

        // D44: クロスドキュメント参照(対象が今描画中の文書と異なる)。見出しへの参照は
        // 相対 `.md` リンク+アンカー(`compute_workspace_anchors` の事前計算表を引く)、
        // それ以外は単一文書時の退化規則(§4.4)に文書名を添えたもの(裁量、最終報告参照)。
        if let Some(ws) = &self.workspace
            && let Some(owner) = ws.doc_of.get(&to)
            && owner != ws.current_doc
        {
            let stem = ws.doc_stems.get(owner).cloned().unwrap_or_else(|| owner.clone());
            if let Some(NodePayload::Section(s)) = self.graph.nodes.get(&to).map(|n| &n.payload) {
                let anchor = ws.cross_anchors.get(&to).cloned().unwrap_or_default();
                let label = if !text.is_empty() { text.to_string() } else { plain_text(&s.heading) };
                return format!("[{}]({stem}.md#{anchor}){}", md_escape(&label), coord_suffix);
            }
            let doc_title = ws.doc_titles.get(owner).cloned().unwrap_or_else(|| owner.clone());
            let (kind, caption) = self.degenerate_descriptor(to);
            let kind = if coord.is_some() { "セル" } else { kind };
            return if !text.is_empty() {
                format!("{}（{}: {}、{}）{}", md_escape(text), kind, caption, doc_title, coord_suffix)
            } else {
                format!("（{}: {}、{}）{}", kind, caption, doc_title, coord_suffix)
            };
        }

        if let Some(NodePayload::Section(s)) = self.graph.nodes.get(&to).map(|n| &n.payload) {
            let anchor = self.anchors.get(&to).cloned().unwrap_or_else(|| "".to_string());
            let label = if !text.is_empty() { text.to_string() } else { plain_text(&s.heading) };
            return format!("[{}](#{}){}", md_escape(&label), anchor, coord_suffix);
        }

        // `cell:` 参照(coord あり)は対象が常に Table だが、意味的には表全体では
        // なく特定セルを指しているので「セル」と明示する(裁量: table 参照と cell
        // 参照を同じ「表」表記で混同させないための上書き)。
        let (kind, caption) = self.degenerate_descriptor(to);
        let kind = if coord.is_some() { "セル" } else { kind };
        if !text.is_empty() {
            format!("{}（{}: {}）{}", md_escape(text), kind, caption, coord_suffix)
        } else {
            format!("（{}: {}）{}", kind, caption, coord_suffix)
        }
    }

    /// 番号付け・アンカー化ができない参照先の「種別」「キャプション相当の短い説明」を
    /// 組み立てる(D38: `(表: キャプション)` 形式のテキスト退化)。
    fn degenerate_descriptor(&self, to: NodeId) -> (&'static str, String) {
        match self.graph.nodes.get(&to).map(|n| &n.payload) {
            Some(NodePayload::Table(t)) => {
                ("表", t.caption.as_ref().map(|c| plain_text(c)).unwrap_or_else(|| "無題".to_string()))
            }
            Some(NodePayload::Math(m)) => ("数式", truncate(&to_tex(&m.tree))),
            Some(NodePayload::Figure(Figure::Chart(c))) => (
                "図",
                c.caption
                    .as_ref()
                    .map(|cap| plain_text(cap))
                    .or_else(|| c.depicts.get("description").cloned())
                    .unwrap_or_else(|| "無題".to_string()),
            ),
            Some(NodePayload::Figure(Figure::Image(img))) => (
                "図",
                img.caption
                    .as_ref()
                    .map(|cap| plain_text(cap))
                    .or_else(|| img.depicts.get("description").cloned())
                    .unwrap_or_else(|| img.alt.clone()),
            ),
            Some(NodePayload::Para(_)) => ("段落", short_label(self.graph, to)),
            Some(NodePayload::List(_)) => ("リスト", short_label(self.graph, to)),
            Some(NodePayload::Code(c)) => ("コード", format!("({})", c.lang)),
            Some(NodePayload::Record(_)) => ("record", short_label(self.graph, to)),
            Some(NodePayload::Anchor(_)) => ("anchor", short_label(self.graph, to)),
            Some(NodePayload::Value(_)) => ("値", short_label(self.graph, to)),
            Some(NodePayload::Quote(_)) => ("引用", short_label(self.graph, to)),
            Some(NodePayload::ThematicBreak(_)) => ("水平線", String::new()),
            Some(NodePayload::Document(_)) | Some(NodePayload::Term(_)) | Some(NodePayload::Section(_)) | None => {
                ("参照", short_label(self.graph, to))
            }
        }
    }

    /// D38: `Term` の描画。`text` があればそれを、無ければ Term ノードの `name` を、
    /// 強調なしのプレーンテキストとして出す(strata-typst::render_term と同じ方針)。
    fn render_term(&self, to: NodeId, text: &str) -> String {
        if !text.is_empty() {
            return md_escape(text);
        }
        match self.graph.nodes.get(&to).map(|n| &n.payload) {
            Some(NodePayload::Term(Term { name })) => md_escape(name),
            _ => String::new(),
        }
    }

    /// D28/D29: `CellValue` の描画(表セルと record 値で共有)。`table_cell` が true なら
    /// GFM パイプ表セルとして安全な形(改行を許さず `|` をエスケープ)にする。
    fn render_cell_value(&self, v: &CellValue) -> String {
        match v {
            CellValue::Number { v } => v.to_string(),
            CellValue::Text { v } => md_escape(v),
            CellValue::Ref { to } => match self.graph.nodes.get(to).map(|n| &n.payload) {
                Some(NodePayload::Value(val)) => scalar_text(&val.scalar, val.unit.as_deref()),
                _ => "値".to_string(),
            },
            CellValue::Quantity { v, unit } => format!("{v} {}", md_escape(unit)),
            CellValue::Empty => String::new(),
            CellValue::Date(d) => format_date(d),
            CellValue::Period { from, to } => format_period(from, to.as_ref()),
        }
    }

    /// D28: Record の標準描画(2列 GFM 表、`キー` / `値` ヘッダ)。
    fn render_record(&mut self, r: &Record) -> Result<String, String> {
        let mut out = String::new();
        out.push_str("| キー | 値 |\n| --- | --- |\n");
        for entry in &r.entries {
            let key = table_cell_escape(&md_escape(&entry.key));
            let val = table_cell_escape(&self.render_cell_value(&entry.value));
            out.push_str(&format!("| {key} | {val} |\n"));
        }
        out.push('\n');
        Ok(out)
    }

    /// D38: `::table` / GFM パイプ表ブリッジ由来の `Table` を GFM パイプ表へ平坦化する。
    /// - ネスト行次元: MultiIndex 方式(親ラベルを各行に繰り返す)。1レベルにつき
    ///   左端に1列(ヘッダは `Dim.name`)。
    /// - ネスト列次元: パス連結ヘッダ(`" / "` 区切り)。
    /// - GFM パイプ表ブリッジ由来の「合成 row 軸」(`bridge_gfm_table` が作る
    ///   `name="row"`・ラベル無し・key が `r1..rN` の1レベル軸)は行ヘッダ列を出さない
    ///   (裁量、最終報告参照 — 素の GFM 表の round-trip に必須)。
    fn render_table(&mut self, table: &Table) -> Result<String, String> {
        let mut out = String::new();
        if let Some(caption) = &table.caption {
            out.push_str(&format!("**表: {}**\n\n", self.render_inlines(caption)?));
        }

        let row_leaves = describe_leaves(&table.rows);
        let col_leaves = describe_leaves(&table.cols);
        let d_row = if is_synthetic_row_axis(&table.rows) { 0 } else { max_depth(&table.rows) };
        let d_col = max_depth(&table.cols);

        let mut cell_map: HashMap<(Vec<String>, Vec<String>), &CellValue> = HashMap::new();
        for cell in &table.cells {
            cell_map.insert((cell.row_path.clone(), cell.col_path.clone()), &cell.value);
        }

        // ヘッダ行。
        let mut header_cells: Vec<String> = Vec::new();
        if d_row > 0 {
            // 非対称な木(枝ごとに深さが違う、稀なケース)で `dim_names_by_depth` が
            // `d_row` に届かない場合は空文字列で埋める(列数を必ず `d_row` に揃える
            // 安全策。実際の対称なネスト次元では常に一致する)。
            let mut names = dim_names_by_depth(&table.rows);
            names.resize(d_row, String::new());
            for name in &names {
                header_cells.push(table_cell_escape(&md_escape(name)));
            }
        }
        if d_col > 0 {
            for leaf in &col_leaves {
                let path_text: Vec<String> = leaf.iter().map(|(k, l)| display_text(k, l)).collect();
                header_cells.push(table_cell_escape(&path_text.join(" / ")));
            }
        } else {
            header_cells.push("値".to_string());
        }
        out.push_str(&format!("| {} |\n", header_cells.join(" | ")));
        out.push_str(&format!("|{}|\n", vec![" --- "; header_cells.len()].join("|")));

        // データ行。
        for row_leaf in &row_leaves {
            let row_path: Vec<String> = row_leaf.iter().map(|(k, _)| k.clone()).collect();
            let mut cells: Vec<String> = Vec::new();
            if d_row > 0 {
                // ヘッダ側と同じ安全策(非対称な木への防御、上記コメント参照)。
                let mut level_texts: Vec<String> = row_leaf.iter().map(|(k, l)| display_text(k, l)).collect();
                level_texts.resize(d_row, String::new());
                for t in &level_texts {
                    cells.push(table_cell_escape(t));
                }
            }
            if d_col > 0 {
                for col_leaf in &col_leaves {
                    let col_path: Vec<String> = col_leaf.iter().map(|(k, _)| k.clone()).collect();
                    let text = cell_map.get(&(row_path.clone(), col_path)).map(|v| self.render_cell_value(v)).unwrap_or_default();
                    cells.push(table_cell_escape(&text));
                }
            } else {
                let text =
                    cell_map.get(&(row_path.clone(), Vec::new())).map(|v| self.render_cell_value(v)).unwrap_or_default();
                cells.push(table_cell_escape(&text));
            }
            out.push_str(&format!("| {} |\n", cells.join(" | ")));
        }
        out.push('\n');
        Ok(out)
    }

    /// D38: `chart` = depicts テキストのプレースホルダ**引用**(blockquote)、
    /// `image` = `![alt](src)`(spec 原文の指示をそのまま採用)。
    fn render_figure(&mut self, f: &Figure) -> Result<String, String> {
        match f {
            Figure::Chart(c) => self.render_chart(c),
            Figure::Image(img) => self.render_image(img),
        }
    }

    fn render_chart(&mut self, c: &Chart) -> Result<String, String> {
        let mut out = String::new();
        if let Some(caption) = &c.caption {
            out.push_str(&format!("**図: {}**\n\n", self.render_inlines(caption)?));
        }
        let desc = c.depicts.get("description").cloned().unwrap_or_else(|| "(チャート)".to_string());
        let data_label = short_label(self.graph, c.data_ref);
        let encode = match &c.encode.color {
            Some(color) => format!("{}: {} × {}(色: {})", mark_str(c.mark), c.encode.x, c.encode.y, color),
            None => format!("{}: {} × {}", mark_str(c.mark), c.encode.x, c.encode.y),
        };
        out.push_str(&quote_prefix(&format!("{}\nデータ: {} / {}", md_escape(&desc), data_label, encode)));
        out.push_str("\n\n");
        Ok(out)
    }

    fn render_image(&mut self, img: &ImageFigure) -> Result<String, String> {
        let mut out = String::new();
        if let Some(caption) = &img.caption {
            out.push_str(&format!("**図: {}**\n\n", self.render_inlines(caption)?));
        }
        out.push_str(&format!("![{}]({})\n\n", md_escape(&img.alt), img.src));
        Ok(out)
    }
}

// --- ユーティリティ -----------------------------------------------------------------

/// GitHub 互換のヘッダアンカースラグ生成+重複時の `-1`/`-2` 付番。
/// 日本語見出しのアンカー化規則(裁量、最終報告参照): Unicode の英数字(CJK 表意
/// 文字を含む)とハイフン・アンダースコアのみを残し、空白の並びを1つのハイフンに
/// 変換、その他の記号は削除する。空文字列になった場合は `section` にフォールバック。
#[derive(Default)]
struct AnchorTable(HashMap<String, u32>);

impl AnchorTable {
    fn assign(&mut self, text: &str) -> String {
        let base = slugify(text);
        let count = self.0.entry(base.clone()).or_insert(0);
        let slug = if *count == 0 { base } else { format!("{base}-{count}") };
        *count += 1;
        slug
    }
}

fn slugify(text: &str) -> String {
    let mut out = String::new();
    let mut pending_hyphen = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !out.is_empty() {
                pending_hyphen = true;
            }
            continue;
        }
        let lower: String = c.to_lowercase().collect();
        for lc in lower.chars() {
            if lc.is_alphanumeric() || lc == '-' || lc == '_' {
                if pending_hyphen {
                    out.push('-');
                    pending_hyphen = false;
                }
                out.push(lc);
            }
            // それ以外の記号は削除(ハイフンは挿入しない、GitHub スラガーと同じ)。
        }
    }
    if out.is_empty() { "section".to_string() } else { out }
}

/// `cell:` 参照の座標を表示テキストへ添える(strata-typst::format_coord と同じ体裁)。
fn format_coord(coord: &CellCoord) -> String {
    let row = coord.row_path.join(".");
    let col = coord.col_path.join(".");
    format!(" ({row}, {col})")
}

/// インライン列をプレーンテキストへ落とす(見出しのアンカー算出・短いラベルに使う)。
fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { s } => out.push_str(s),
            Inline::Emph { children, .. } => out.push_str(&plain_text(children)),
            Inline::Ref { text, .. } => out.push_str(text),
            Inline::Term { text, .. } => out.push_str(text),
            Inline::Link { text, .. } => out.push_str(text),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::Math { .. } | Inline::Anchor { .. } => {}
        }
    }
    out
}

const MAX_LABEL_CHARS: usize = 60;

fn truncate(s: &str) -> String {
    let normalized = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = normalized.chars().count();
    if count <= MAX_LABEL_CHARS {
        return normalized;
    }
    let head: String = normalized.chars().take(MAX_LABEL_CHARS).collect();
    format!("{head}…")
}

/// ノード1つの短いラベル(strata-context::node_short_label のミニ版。参照先の
/// テキスト退化・チャートのデータ参照表示に使う。役割が違うため独立実装 — 最終報告参照)。
fn short_label(graph: &Graph, id: NodeId) -> String {
    let Some(node) = graph.nodes.get(&id) else {
        return format!("(不明なノード {})", id.0);
    };
    match &node.payload {
        NodePayload::Section(s) => truncate(&plain_text(&s.heading)),
        NodePayload::Para(p) => truncate(&plain_text(&p.inline)),
        NodePayload::List(l) => {
            if l.ordered { "順序リスト".to_string() } else { "箇条書きリスト".to_string() }
        }
        NodePayload::Table(t) => t.caption.as_ref().map(|c| truncate(&plain_text(c))).unwrap_or_else(|| "表".to_string()),
        NodePayload::Math(_) => "数式".to_string(),
        NodePayload::Figure(Figure::Chart(c)) => {
            c.depicts.get("description").map(|d| truncate(d)).unwrap_or_else(|| "図(チャート)".to_string())
        }
        NodePayload::Figure(Figure::Image(img)) => {
            img.depicts.get("description").map(|d| truncate(d)).unwrap_or_else(|| truncate(&img.alt))
        }
        NodePayload::Code(c) => format!("コード({})", c.lang),
        NodePayload::Term(t) => t.name.clone(),
        NodePayload::Anchor(a) => truncate(&plain_text(&a.inline)),
        NodePayload::Value(v) => scalar_text(&v.scalar, v.unit.as_deref()),
        NodePayload::Document(d) => d.title.clone().unwrap_or_else(|| "文書".to_string()),
        NodePayload::Record(r) => {
            let first = r.entries.first().map(|e| format!("{}: {}", e.key, cell_value_plain(&e.value)));
            match first {
                Some(f) => format!("record({}件, 先頭: {})", r.entries.len(), truncate(&f)),
                None => "record(空)".to_string(),
            }
        }
        NodePayload::Quote(_) => "引用".to_string(),
        NodePayload::ThematicBreak(_) => "水平線".to_string(),
    }
}

fn cell_value_plain(v: &CellValue) -> String {
    match v {
        CellValue::Number { v } => v.to_string(),
        CellValue::Text { v } => v.clone(),
        CellValue::Ref { to } => format!("→ {}", to.0),
        CellValue::Empty => String::new(),
        CellValue::Quantity { v, unit } => format!("{v} {unit}"),
        CellValue::Date(d) => format_date(d),
        CellValue::Period { from, to } => format_period(from, to.as_ref()),
    }
}

fn scalar_text(scalar: &Scalar, unit: Option<&str>) -> String {
    let s = match scalar {
        Scalar::Number(n) => n.to_string(),
        Scalar::Text(s) => md_escape(s),
        Scalar::Bool(b) => b.to_string(),
    };
    match unit {
        Some(u) => format!("{s}{}", md_escape(u)),
        None => s,
    }
}

/// D29: 日付・期間の素直な表示(strata-typst::format_date/format_period と同じ、
/// 日本語化はビューの仕事なのでやらない)。
fn format_date(d: &DateValue) -> String {
    match d.d {
        Some(day) => format!("{:04}-{:02}-{:02}", d.y, d.m, day),
        None => format!("{:04}-{:02}", d.y, d.m),
    }
}

fn format_period(from: &DateValue, to: Option<&DateValue>) -> String {
    match to {
        Some(t) => format!("{} 〜 {}", format_date(from), format_date(t)),
        None => format!("{} 〜 現在", format_date(from)),
    }
}

fn mark_str(m: Mark) -> &'static str {
    match m {
        Mark::Line => "line",
        Mark::Bar => "bar",
        Mark::Point => "point",
        Mark::Area => "area",
    }
}

/// Markdown 本文向けのエスケープ。強調・打ち消し・コード・リンク・パイプ・見出し
/// マーカーとして誤読されうる記号をバックスラッシュエスケープする。SML 側の
/// `\X`(X は ASCII punctuation)がそのまま `Inline::Text` へ unescape される
/// (strata-build::convert::convert_inline の `Escaped` アーム)ため、ここで
/// 積極的にエスケープしても意味構造(Graph)としては元の Text と往復する
/// (裁量、最終報告参照)。
fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if matches!(c, '\\' | '*' | '_' | '`' | '~' | '[' | ']' | '<' | '#' | '|') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// GFM パイプ表セル向けの追加エスケープ: セル内の改行は落とせないため空白に畳む
/// (表セルは常に単一行の入力から来る想定 — sml-spec §6.1 の `@cells` 記法・GFM 表の
/// 1行1レコード原則、複数行テキストが紛れ込む場合の防御的処置)。`|` は既に
/// `md_escape` でエスケープ済みなのでここでは改行だけを処理する。
fn table_cell_escape(s: &str) -> String {
    s.replace('\n', " ")
}

fn ensure_trailing_newline(s: &str) -> &str {
    s.trim_end_matches('\n')
}

/// blockquote(`>`)のインデント付与。空行も `>` 単独行にする(CommonMark の
/// lazy continuation に頼らず、明示的に引用ブロックを継続させる)。
fn quote_prefix(s: &str) -> String {
    s.lines()
        .map(|l| if l.trim().is_empty() { ">".to_string() } else { format!("> {l}") })
        .collect::<Vec<_>>()
        .join("\n")
}

/// `strata_core::MathNode` → `tex2math::MathNode`(strata-build::math::convert の逆写像)。
/// 両クレートのバリアント集合は 1:1 対応するため素朴な match で写す。
fn to_core_tex_node(n: &MathNode) -> tex2math::MathNode {
    use tex2math::MathNode as T;
    match n {
        MathNode::Num { v } => T::Num { v: v.clone() },
        MathNode::Ident { v } => T::Ident { v: v.clone() },
        MathNode::Op { v } => T::Op { v: v.clone() },
        MathNode::Row { items } => T::Row { items: items.iter().map(to_core_tex_node).collect() },
        MathNode::Frac { num, den } => {
            T::Frac { num: Box::new(to_core_tex_node(num)), den: Box::new(to_core_tex_node(den)) }
        }
        MathNode::Sup { base, sup } => {
            T::Sup { base: Box::new(to_core_tex_node(base)), sup: Box::new(to_core_tex_node(sup)) }
        }
        MathNode::Sub { base, sub } => {
            T::Sub { base: Box::new(to_core_tex_node(base)), sub: Box::new(to_core_tex_node(sub)) }
        }
        MathNode::SubSup { base, sub, sup } => T::SubSup {
            base: Box::new(to_core_tex_node(base)),
            sub: Box::new(to_core_tex_node(sub)),
            sup: Box::new(to_core_tex_node(sup)),
        },
        MathNode::UnderOver { base, under, over } => T::UnderOver {
            base: Box::new(to_core_tex_node(base)),
            under: under.as_ref().map(|b| Box::new(to_core_tex_node(b))),
            over: over.as_ref().map(|b| Box::new(to_core_tex_node(b))),
        },
        MathNode::Sqrt { body } => T::Sqrt { body: Box::new(to_core_tex_node(body)) },
        MathNode::Root { radicand, index } => {
            T::Root { radicand: Box::new(to_core_tex_node(radicand)), index: Box::new(to_core_tex_node(index)) }
        }
        MathNode::Fenced { open, close, body } => {
            T::Fenced { open: open.clone(), close: close.clone(), body: Box::new(to_core_tex_node(body)) }
        }
        MathNode::Text { s } => T::Text { s: s.clone() },
    }
}

/// WP-M1(tex2math::to_tex)を canonical の `MathNode` に対して呼ぶための橋渡し。
fn to_tex(n: &MathNode) -> String {
    tex2math::to_tex(&to_core_tex_node(n))
}

/// ネスト次元木の**深さレベルごと**の `Dim.name` を1つずつ集める(裁量: 表平坦化の
/// 行ヘッダ列見出しに使う)。`table.rows`(トップレベルの `Vec<Dim>`)の要素数と
/// 実際のネスト深さ(`max_depth`)は別物であることに注意 — 例えば
/// `rows: [Dim{name:"company", members:[{children: [Dim{name:"project", ...}]}]}]`
/// は `table.rows.len() == 1` だが深さは2(company→project)。最初に見つかった枝を
/// 辿って各レベルの名前を確定させる(枝ごとに深さが異なる「非対称な木」は本実装の
/// 対象外の稀なケースと判断)。
fn dim_names_by_depth(tree: &DimTree) -> Vec<String> {
    let mut names = Vec::new();
    let mut cur = tree;
    while let Some(dim) = cur.first() {
        names.push(dim.name.clone());
        let Some(member) = dim.members.first() else { break };
        if member.children.is_empty() {
            break;
        }
        cur = &member.children;
    }
    names
}

/// 次元木の深さ(strata-typst::max_depth と同一ロジック)。
fn max_depth(tree: &DimTree) -> usize {
    if tree.is_empty() {
        return 0;
    }
    let mut max = 0;
    for dim in tree {
        for member in &dim.members {
            let d = max_depth(&member.children);
            if d > max {
                max = d;
            }
        }
    }
    max + 1
}

/// 次元木の1レベル分: `(member key, 表示ラベル)`。
type PathSegment = (String, Option<Vec<Inline>>);

/// 各葉について、根から葉までの `(key, label)` の列を集める。`Cell.row_path`/`col_path`
/// (キーのみの列)と、表示テキスト算出に必要な `label` の両方をここから得る。
fn describe_leaves(tree: &DimTree) -> Vec<Vec<PathSegment>> {
    let mut leaves = Vec::new();
    fn recurse(tree: &DimTree, current: &mut Vec<PathSegment>, leaves: &mut Vec<Vec<PathSegment>>) {
        if tree.is_empty() {
            leaves.push(current.clone());
            return;
        }
        for dim in tree {
            for member in &dim.members {
                current.push((member.key.clone(), member.label.clone()));
                if member.children.is_empty() {
                    leaves.push(current.clone());
                } else {
                    recurse(&member.children, current, leaves);
                }
                current.pop();
            }
        }
    }
    recurse(tree, &mut Vec::new(), &mut leaves);
    if leaves.is_empty() {
        leaves.push(Vec::new());
    }
    leaves
}

fn display_text(key: &str, label: &Option<Vec<Inline>>) -> String {
    match label {
        Some(inlines) => plain_text(inlines),
        None => key.to_string(),
    }
}

/// D38 裁量: GFM パイプ表ブリッジ由来の「合成 row 軸」を検出する
/// (`strata_build::convert::bridge_gfm_table` が作る形そのもの: 1レベル・
/// `Dim.name == "row"`・全メンバーが葉かつラベル無し・key が `r1, r2, ...` の連番)。
/// これに一致する場合、行ヘッダ列を出力しない(素の GFM 表は row 軸を持たないため、
/// 出力すると round-trip で余計な列が増える)。手書き `::table` が偶然この形
/// (次元名 "row"・r1..rN の連番キー・ラベル無し)を選ぶ稀なケースは意図せず
/// 行ヘッダが消える誤検出になりうるが、実害は小さいと判断した(最終報告に明記)。
fn is_synthetic_row_axis(rows: &DimTree) -> bool {
    if rows.len() != 1 || rows[0].name != "row" {
        return false;
    }
    let members = &rows[0].members;
    if members.is_empty() {
        return false;
    }
    members.iter().enumerate().all(|(i, m)| m.children.is_empty() && m.label.is_none() && m.key == format!("r{}", i + 1))
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod golden;
#[cfg(test)]
mod roundtrip;
