//! strata-typst — canonical グラフ(層2)→ Typst マークアップのレンダラ(Milestone 4)。
//!
//! スコープは docs/sml-render-m4-handoff.md D-R2(sml-spec.md §1.3 D21/D22)。
//! Typst を一次レンダラとする(D19)。strata-html は凍結対象で本クレートは触れない。
//!
//! 描画が辿るのは `Rel::Contains` のみ(D-R2 6.): supports/depends-on/cites/
//! RefersTo/TermRef はグラフの意味情報であり紙面には出さない。

use std::collections::HashMap;
use strata_core::{
    CellCoord, CellValue, Chart, DimTree, EmphKind, Figure, Graph, ImageFigure, Inline, List, Mark, MathNode,
    NodeId, NodePayload, Scalar, Table, Term,
};

/// グラフから Typst ソースを描画する(D18: 中間 JSON を介さず build → render を直結)。
///
/// `root` は `strata_build::BuildOutput::root` が返すノード(通常は `Document`)。
/// `Document` 以外のノードを渡すこともでき(単体テスト用途)、その場合は文書メタは
/// `fallback_title` のみで組み立て、`root` 自体をそのまま描画する。
///
/// `fallback_title`: `Document.title` も本文中の最初の H1 見出しも無い場合に使う
/// 文書タイトル(D21 の3段フォールバックの最終段)。CLI は入力ファイル名(拡張子抜き)
/// を渡す想定(sml-render-m4-handoff.md D-R2 1. で「シグネチャは裁量」とされた箇所。
/// 呼び出し側にフォールバック名を渡させる形で決定した)。
pub fn render_to_typst(graph: &Graph, root: NodeId, fallback_title: &str) -> Result<String, String> {
    let mut renderer = TypstRenderer::new(graph);
    let (title, content) = renderer.render_root(root, fallback_title)?;

    let doc = format!(
        r##"// Strata Document - Generated Typst Source
#set document(title: "{title}")

#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2.5cm),
)
#set text(
  font: ("Libertinus Serif", "Noto Sans CJK JP", "IPAexMincho"),
  size: 10pt,
  lang: "ja",
)
#set par(
  justify: true,
  leading: 0.65em,
)
// D22: table/math/figure のみ自動番号付けの対象。math.equation の numbering を
// 有効にすると、ブロック数式(display 形)にだけ番号が振られる(インライン数式は
// Typst が非 display と判定するため番号は付かない)。
#set math.equation(numbering: "(1)")

// スタイル定義
#show heading: set text(fill: rgb("#2b3a42"))
#show heading.where(level: 1): it => {{
  v(1em)
  align(center, text(size: 20pt, weight: "bold")[#it.body])
  v(0.5em)
}}
#show heading.where(level: 2): it => {{
  v(0.8em)
  block(
    width: 100%,
    stroke: (bottom: 1pt + rgb("#dddddd")),
    inset: (bottom: 0.5em),
    text(size: 14pt, weight: "bold")[#it.body]
  )
  v(0.3em)
}}

{content}"##,
        title = typst_string_escape(&title),
    );

    Ok(doc)
}

struct TypstRenderer<'a> {
    graph: &'a Graph,
}

impl<'a> TypstRenderer<'a> {
    fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    /// D21: 文書タイトルの3段フォールバック(`Document.title` → 最初の H1 → 呼び出し側
    /// 提供のフォールバック名)と、本文の描画をまとめて行う。
    ///
    /// `root` が `Document` でない場合(単体テストで個々のノードだけを描画したい
    /// 場合など)は、本文としてそのノード自体を描画し、タイトルは
    /// `fallback_title` をそのまま使う。
    fn render_root(&mut self, root: NodeId, fallback_title: &str) -> Result<(String, String), String> {
        let node =
            self.graph.nodes.get(&root).ok_or_else(|| format!("Node not found in graph: {:?}", root))?;

        if let NodePayload::Document(document) = &node.payload {
            let title = document
                .title
                .clone()
                .or_else(|| self.first_h1_title(root))
                .unwrap_or_else(|| fallback_title.to_string());

            let mut content = String::new();
            for child_id in self.graph.children_of(root) {
                content.push_str(&self.render_node(child_id, 1)?);
            }
            Ok((title, content))
        } else {
            let content = self.render_node(root, 1)?;
            Ok((fallback_title.to_string(), content))
        }
    }

    /// D21: 「最初の H1 のプレーンテキスト」。canonical の `Section` は見出しレベルを
    /// 持たない(レベルは contains のネスト位置で表現される)ため、ここでは
    /// 「Document 直下(トップレベル)に ord 順で最初に現れる Section」を運用上の
    /// 「最初の H1」と定義する(sml-render-m4-handoff.md には明記が無く裁量で決めた
    /// 箇所)。
    fn first_h1_title(&self, root: NodeId) -> Option<String> {
        for child_id in self.graph.children_of(root) {
            if let Some(NodePayload::Section(s)) = self.graph.nodes.get(&child_id).map(|n| &n.payload) {
                return Some(self.plain_text(&s.heading));
            }
        }
        None
    }

    /// インライン列をプレーンテキストへ落とす(タイトルフォールバック・Ref の見出し
    /// 代替表記に使う)。整形(強調等)は捨てる。`Inline::Math` はタイトル用途では
    /// 無視する(裁量。数式混じりの見出しをタイトルにするケースは稀と判断)。
    fn plain_text(&self, inlines: &[Inline]) -> String {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text { s } => out.push_str(s),
                Inline::Emph { children, .. } => out.push_str(&self.plain_text(children)),
                Inline::Ref { text, .. } => out.push_str(text),
                Inline::Term { text, .. } => out.push_str(text),
                Inline::Math { .. } | Inline::Anchor { .. } => {}
            }
        }
        out
    }

    fn render_node(&mut self, node_id: NodeId, depth: usize) -> Result<String, String> {
        let node =
            self.graph.nodes.get(&node_id).ok_or_else(|| format!("Node not found in graph: {:?}", node_id))?;

        match &node.payload {
            NodePayload::Section(s) => {
                let heading_typst = self.render_inlines(&s.heading)?;
                let prefix = "=".repeat(depth.max(1));

                let mut children_typst = String::new();
                for child_id in self.graph.children_of(node_id) {
                    children_typst.push_str(&self.render_node(child_id, depth + 1)?);
                }

                Ok(format!("{} {} <{}>\n\n{}", prefix, heading_typst, label(node_id), children_typst))
            }
            NodePayload::Para(p) => {
                let inline_typst = self.render_inlines(&p.inline)?;
                Ok(format!("{} <{}>\n\n", inline_typst, label(node_id)))
            }
            NodePayload::List(l) => self.render_list(l, node_id, depth),
            NodePayload::Table(t) => self.render_table(t, node_id),
            NodePayload::Math(m) => {
                let math_str = self.render_math(&m.tree);
                Ok(format!("$ {} $ <{}>\n\n", math_str, label(node_id)))
            }
            NodePayload::Code(c) => Ok(format!("```{}\n{}\n``` <{}>\n\n", c.lang, c.src, label(node_id))),
            NodePayload::Figure(f) => self.render_figure(f, node_id),
            NodePayload::Term(t) => Ok(typst_escape(&t.name)),
            NodePayload::Value(v) => {
                let val_str = match &v.scalar {
                    Scalar::Number(n) => n.to_string(),
                    Scalar::Text(s) => typst_escape(s),
                    Scalar::Bool(b) => b.to_string(),
                };
                let unit_str = v.unit.as_deref().unwrap_or("");
                Ok(format!("{}{}", val_str, typst_escape(unit_str)))
            }
            NodePayload::Anchor(a) => {
                let inner = self.render_inlines(&a.inline)?;
                Ok(format!("[{}] <{}>", inner, label(node_id)))
            }
            NodePayload::Document(_) => {
                // 通常 Document はルートとしてのみ現れ、render_root が別経路で処理する
                // (子ノードとして contains されることは build 側で起きない)。防御的に
                // 空文字列を返す。
                Ok(String::new())
            }
        }
    }

    /// D22: List ノード自体にもラベルが必要だが、Typst のマークアップ構文
    /// (`- item` の連続)には「リスト全体」を指す単一のトークンが無く、末尾行に
    /// ラベルを続けて書くと直前の項目に付いてしまう(実測で確認済み。
    /// `warning: content labelled multiple times` になり項目側が勝つ)。
    /// そのため `#block[...]<label>` でリスト全体を1つのコンテンツにくるみ、
    /// ブロックへラベルを付ける(sml-render-m4-handoff.md「既知の注意点」に対応する
    /// 実装上の裁量)。
    fn render_list(&mut self, l: &List, node_id: NodeId, depth: usize) -> Result<String, String> {
        let marker = if l.ordered { "+" } else { "-" };
        let mut items_typst = String::new();

        for child_id in self.graph.children_of(node_id) {
            let child_node = self
                .graph
                .nodes
                .get(&child_id)
                .ok_or_else(|| format!("Child node not found: {:?}", child_id))?;

            let child_content = match &child_node.payload {
                NodePayload::Para(p) => self.render_inlines(&p.inline)?,
                _ => self.render_node(child_id, depth)?,
            };
            items_typst.push_str(&format!("{} {} <{}>\n", marker, child_content, label(child_id)));
        }

        Ok(format!("#block[\n{}] <{}>\n\n", items_typst, label(node_id)))
    }

    fn render_inlines(&mut self, inlines: &[Inline]) -> Result<String, String> {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text { s } => {
                    out.push_str(&typst_escape(s));
                }
                Inline::Emph { kind, children } => {
                    let inner = self.render_inlines(children)?;
                    match kind {
                        EmphKind::Strong => out.push_str(&format!("*{}*", inner)),
                        EmphKind::Em => out.push_str(&format!("_{}_", inner)),
                        EmphKind::Code => out.push_str(&format!("`{}`", inner)),
                    };
                }
                Inline::Math { tree } => {
                    let math_str = self.render_math(tree);
                    out.push_str(&format!("${}$", math_str));
                }
                Inline::Ref { to, text, coord, .. } => {
                    out.push_str(&self.render_ref(*to, text, coord.as_ref()));
                }
                Inline::Term { to, text } => {
                    out.push_str(&self.render_term(*to, text));
                }
                Inline::Anchor { to } => {
                    if let Some(NodePayload::Anchor(a)) = self.graph.nodes.get(to).map(|n| &n.payload) {
                        let inner = self.render_inlines(&a.inline)?;
                        out.push_str(&format!("[{}] <{}>", inner, label(*to)));
                    }
                }
            }
        }
        Ok(out)
    }

    /// D22: `Ref` の描画。
    /// - `text` が非空 → `#link(<to>)[text]`(`coord` があれば表示テキストに
    ///   ` (行パス, 列パス)` を添える)。
    /// - `text` が空、かつ対象が番号付け対象(Table/Math/Figure、D22 が自動番号付けを
    ///   規定する3種)→ `@to`。
    /// - `text` が空、かつ対象が番号を持たない(Section/Para/List/Code 等)
    ///   → `#link` + 短い代替表記(Section は見出しテキスト、それ以外は "§"。
    ///   sml-render-m4-handoff.md D-R2 5. で明示的に裁量とされた箇所)。
    fn render_ref(&self, to: NodeId, text: &str, coord: Option<&CellCoord>) -> String {
        let coord_suffix = coord.map(format_coord).unwrap_or_default();

        if !text.is_empty() {
            return format!("#link(<{}>)[{}{}]", label(to), typst_escape(text), coord_suffix);
        }

        match self.graph.nodes.get(&to).map(|n| &n.payload) {
            Some(NodePayload::Table(_)) | Some(NodePayload::Math(_)) | Some(NodePayload::Figure(_)) => {
                format!("@{}", label(to))
            }
            Some(NodePayload::Section(s)) => {
                format!("#link(<{}>)[{}{}]", label(to), typst_escape(&self.plain_text(&s.heading)), coord_suffix)
            }
            Some(_) => format!("#link(<{}>)[§{}]", label(to), coord_suffix),
            None => format!("#link(<{}>)[参照{}]", label(to), coord_suffix),
        }
    }

    /// D22: `Term` の描画。`text` があればそれを、無ければ Term ノードの `name` を、
    /// 強調なしのプレーンテキストとして出す。Term ノード自体はグラフにのみ存在し、
    /// ブロックとして描画されることはない。
    fn render_term(&self, to: NodeId, text: &str) -> String {
        if !text.is_empty() {
            return typst_escape(text);
        }
        match self.graph.nodes.get(&to).map(|n| &n.payload) {
            Some(NodePayload::Term(Term { name })) => typst_escape(name),
            _ => String::new(),
        }
    }

    fn render_math(&self, node: &MathNode) -> String {
        match node {
            MathNode::Num { v } => typst_math_escape(v),
            MathNode::Ident { v } => typst_math_escape(v),
            MathNode::Op { v } => match v.as_str() {
                "∑" => "sum".to_string(),
                "∏" => "prod".to_string(),
                "∫" => "integral".to_string(),
                _ => typst_math_escape(v),
            },
            MathNode::Row { items } => {
                let inner: Vec<String> = items.iter().map(|n| self.render_math(n)).collect();
                inner.join(" ")
            }
            MathNode::Frac { num, den } => {
                format!("({}) / ({})", self.render_math(num), self.render_math(den))
            }
            MathNode::Sup { base, sup } => {
                format!("({})^({})", self.render_math(base), self.render_math(sup))
            }
            MathNode::Sub { base, sub } => {
                format!("({})_({})", self.render_math(base), self.render_math(sub))
            }
            MathNode::SubSup { base, sub, sup } => {
                format!("({})_({})^({})", self.render_math(base), self.render_math(sub), self.render_math(sup))
            }
            MathNode::UnderOver { base, under, over } => {
                // Typst の数式では、sum などの大型演算子に対して _ と ^ を使うと自動で上下になる
                let mut out = format!("({})", self.render_math(base));
                if let Some(u) = under {
                    out.push_str(&format!("_({})", self.render_math(u)));
                }
                if let Some(o) = over {
                    out.push_str(&format!("^({})", self.render_math(o)));
                }
                out
            }
            MathNode::Sqrt { body } => format!("sqrt({})", self.render_math(body)),
            MathNode::Root { radicand, index } => {
                format!("root({}, {})", self.render_math(index), self.render_math(radicand))
            }
            MathNode::Fenced { open, close, body } => {
                format!("{} {} {}", open, self.render_math(body), close)
            }
            MathNode::Text { s } => format!("\"{}\"", s.replace('"', "\\\"")),
        }
    }

    /// D22: Table → `#figure(table(...), caption: ...) <label>`。
    fn render_table(&mut self, table: &Table, node_id: NodeId) -> Result<String, String> {
        let d_row = max_depth(&table.rows);
        let d_col = max_depth(&table.cols);

        let row_leaves = get_leaves(&table.rows);
        let col_leaves = get_leaves(&table.cols);

        let mut out = String::new();

        let mut col_specs = Vec::new();
        for _ in 0..d_row {
            col_specs.push("auto".to_string());
        }
        for _ in 0..col_leaves.len() {
            col_specs.push("1fr".to_string());
        }

        out.push_str(&format!(
            "table(\n    columns: ({}),\n    stroke: 0.5pt + luma(150),\n    fill: (x, y) => if y < {} or x < {} {{ rgb(\"#f7f9fa\") }} else {{ none }},\n",
            col_specs.join(", "),
            d_col,
            d_row
        ));

        // 1. Column headers
        if d_col > 0 {
            let col_headers = build_col_headers(&table.cols, d_col);
            for (level, row) in col_headers.into_iter().enumerate() {
                if level == 0 && d_row > 0 {
                    out.push_str(&format!("    table.cell(colspan: {}, rowspan: {})[],\n", d_row, d_col));
                }

                for cell in row {
                    let label_typst = match &cell.label {
                        Some(inlines) => self.render_inlines(inlines)?,
                        None => typst_escape(&cell.key),
                    };

                    let span_attrs = format_span(cell.colspan, cell.rowspan);
                    out.push_str(&format!("    table.cell{}[*{}*],\n", span_attrs, label_typst));
                }
            }
        }

        // 2. Body
        let row_headers = build_row_headers(&table.rows, d_row);

        let mut cell_map = HashMap::new();
        for cell in &table.cells {
            cell_map.insert((&cell.row_path, &cell.col_path), &cell.value);
        }

        for r in 0..row_leaves.len() {
            if d_row > 0 {
                for cell in &row_headers[r] {
                    let label_typst = match &cell.label {
                        Some(inlines) => self.render_inlines(inlines)?,
                        None => typst_escape(&cell.key),
                    };

                    let span_attrs = format_span(cell.colspan, cell.rowspan);
                    out.push_str(&format!("    table.cell{}[*{}*],\n", span_attrs, label_typst));
                }
            }

            let row_path = &row_leaves[r];
            for col_path in &col_leaves {
                let val_typst = match cell_map.get(&(row_path, col_path)) {
                    Some(CellValue::Number { v }) => v.to_string(),
                    Some(CellValue::Text { v }) => self.render_inlines(&[Inline::Text { s: v.clone() }])?,
                    Some(CellValue::Ref { to }) => {
                        let inner = match self.graph.nodes.get(to).map(|n| &n.payload) {
                            Some(NodePayload::Value(val)) => match &val.scalar {
                                Scalar::Number(n) => n.to_string(),
                                Scalar::Text(s) => typst_escape(s),
                                Scalar::Bool(b) => b.to_string(),
                            },
                            _ => "値".to_string(),
                        };
                        format!("#link(<{}>)[{}]", label(*to), inner)
                    }
                    Some(CellValue::Quantity { v, unit }) => format!("{} {}", v, typst_escape(unit)),
                    Some(CellValue::Empty) | None => "".to_string(),
                };

                out.push_str(&format!("    [{}],\n", val_typst));
            }
        }
        out.push_str("  )");

        let caption_part = match &table.caption {
            Some(inlines) => format!(",\n  caption: [{}]", self.render_inlines(inlines)?),
            None => String::new(),
        };

        Ok(format!("#figure(\n  {}{}\n) <{}>\n\n", out, caption_part, label(node_id)))
    }

    /// D22 4.: Chart / Image は `#figure` に包む。Chart の中身は SVG 実描画をしない
    /// プレースホルダ枠(box)+ `depicts["description"]` + `data_ref` への参照
    /// (`@data_ref` — D-R2 で明記された記法)+ mark/encode の短い併記。見栄えの
    /// 詳細は裁量(sml-render-m4-handoff.md D-R2 4.)。
    fn render_figure(&mut self, f: &Figure, node_id: NodeId) -> Result<String, String> {
        match f {
            Figure::Chart(c) => self.render_chart(c, node_id),
            Figure::Image(img) => self.render_image(img, node_id),
        }
    }

    fn render_chart(&mut self, c: &Chart, node_id: NodeId) -> Result<String, String> {
        let desc = c.depicts.get("description").map(|s| typst_escape(s));
        let encode = match &c.encode.color {
            Some(color) => format!(
                "{}: {} × {}(色: {})",
                mark_to_str(c.mark),
                typst_escape(&c.encode.x),
                typst_escape(&c.encode.y),
                typst_escape(color)
            ),
            None => format!("{}: {} × {}", mark_to_str(c.mark), typst_escape(&c.encode.x), typst_escape(&c.encode.y)),
        };

        let mut body = String::new();
        body.push_str("  box(width: 100%, height: 4cm, stroke: 0.5pt + luma(150))[\n");
        body.push_str("    #align(center + horizon)[\n");
        body.push_str("      チャート(プレースホルダ) #linebreak()\n");
        if let Some(desc) = desc {
            body.push_str(&format!("      {} #linebreak()\n", desc));
        }
        body.push_str(&format!("      データ: @{} #linebreak()\n", label(c.data_ref)));
        body.push_str(&format!("      {}\n", encode));
        body.push_str("    ]\n");
        body.push_str("  ]");

        let caption_part = match &c.caption {
            Some(inlines) => format!(",\n  caption: [{}]", self.render_inlines(inlines)?),
            None => String::new(),
        };

        Ok(format!("#figure(\n{}{}\n) <{}>\n\n", body, caption_part, label(node_id)))
    }

    fn render_image(&mut self, img: &ImageFigure, node_id: NodeId) -> Result<String, String> {
        let alt = typst_escape(&img.alt);
        let src = typst_escape(&img.src);
        let desc = img.depicts.get("description").map(|s| typst_escape(s));

        let mut body = String::new();
        body.push_str("  box(width: 100%, height: 4cm, stroke: 0.5pt + luma(150))[\n");
        body.push_str("    #align(center + horizon)[\n");
        body.push_str("      画像(プレースホルダ) #linebreak()\n");
        body.push_str(&format!("      alt: {} #linebreak()\n", alt));
        if let Some(desc) = desc {
            body.push_str(&format!("      {} #linebreak()\n", desc));
        }
        body.push_str(&format!("      src: {}\n", src));
        body.push_str("    ]\n");
        body.push_str("  ]");

        let caption_part = match &img.caption {
            Some(inlines) => format!(",\n  caption: [{}]", self.render_inlines(inlines)?),
            None => String::new(),
        };

        Ok(format!("#figure(\n{}{}\n) <{}>\n\n", body, caption_part, label(node_id)))
    }
}

/// ブロックノードの Typst ラベル文字列(D22: `<ULID>`)。ULID をそのままラベル名に
/// 使う(Crockford base32 = `[0-9A-Z]` なので Typst のラベル字句と衝突しない)。
fn label(id: NodeId) -> String {
    id.0.to_string()
}

/// `cell:` 参照の座標(§5.3)を表示テキストへ添える(D-R2 5.: 体裁は裁量)。
/// `" (行path, 列path)"` の形。path の各セグメントは `.` で連結する。
fn format_coord(coord: &CellCoord) -> String {
    let row = coord.row_path.join(".");
    let col = coord.col_path.join(".");
    format!(" ({}, {})", typst_escape(&row), typst_escape(&col))
}

fn mark_to_str(m: Mark) -> &'static str {
    match m {
        Mark::Line => "line",
        Mark::Bar => "bar",
        Mark::Point => "point",
        Mark::Area => "area",
    }
}

/// Typst マークアップ(コンテンツモード)向けのエスケープ。
fn typst_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('<', "\\<")
        .replace('>', "\\>")
        .replace('@', "\\@")
        .replace('#', "\\#")
        .replace('&', "\\&")
}

/// Typst の文字列リテラル(`"..."`)向けのエスケープ。マークアップエスケープとは
/// 別物(バックスラッシュと二重引用符のみ)。`#set document(title: "...")` に使う。
fn typst_string_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn typst_math_escape(s: &str) -> String {
    s.replace('_', "\\_").replace('^', "\\^")
}

fn format_span(colspan: usize, rowspan: usize) -> String {
    let mut parts = Vec::new();
    if colspan > 1 {
        parts.push(format!("colspan: {}", colspan));
    }
    if rowspan > 1 {
        parts.push(format!("rowspan: {}", rowspan));
    }
    if parts.is_empty() {
        "".to_string()
    } else {
        format!("({})", parts.join(", "))
    }
}

// 次元の木の深さと葉の数計算 (html/src/lib.rs からのコピー)
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

fn count_leaves(tree: &DimTree) -> usize {
    if tree.is_empty() {
        return 1;
    }
    let mut count = 0;
    for dim in tree {
        for member in &dim.members {
            if member.children.is_empty() {
                count += 1;
            } else {
                count += count_leaves(&member.children);
            }
        }
    }
    count
}

fn get_leaves(tree: &DimTree) -> Vec<Vec<String>> {
    let mut leaves = Vec::new();
    fn recurse(tree: &DimTree, current: &mut Vec<String>, leaves: &mut Vec<Vec<String>>) {
        if tree.is_empty() {
            leaves.push(current.clone());
            return;
        }
        for dim in tree {
            for member in &dim.members {
                current.push(member.key.clone());
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

// ヘッダ構造の計算用 (html/src/lib.rs からのコピー)
#[derive(Clone)]
struct HeaderCell {
    label: Option<Vec<Inline>>,
    key: String,
    colspan: usize,
    rowspan: usize,
}

fn build_col_headers(tree: &DimTree, max_depth: usize) -> Vec<Vec<HeaderCell>> {
    let mut rows = (0..max_depth).map(|_| Vec::new()).collect::<Vec<_>>();

    fn recurse(tree: &DimTree, level: usize, max_depth: usize, rows: &mut Vec<Vec<HeaderCell>>) {
        if tree.is_empty() {
            return;
        }
        for dim in tree {
            for member in &dim.members {
                let colspan = count_leaves(&member.children);
                let rowspan = if member.children.is_empty() { max_depth - level } else { 1 };

                rows[level].push(HeaderCell {
                    label: member.label.clone(),
                    key: member.key.clone(),
                    colspan,
                    rowspan,
                });

                recurse(&member.children, level + 1, max_depth, rows);
            }
        }
    }

    recurse(tree, 0, max_depth, &mut rows);
    rows
}

#[derive(Clone)]
struct RowHeaderCell {
    label: Option<Vec<Inline>>,
    key: String,
    colspan: usize,
    rowspan: usize,
}

fn build_row_headers(tree: &DimTree, max_depth: usize) -> Vec<Vec<RowHeaderCell>> {
    let num_leaves = count_leaves(tree);
    let mut row_headers = (0..num_leaves).map(|_| Vec::new()).collect::<Vec<_>>();
    let mut current_leaf_index = 0;

    fn recurse(
        tree: &DimTree,
        level: usize,
        max_depth: usize,
        current_leaf_index: &mut usize,
        row_headers: &mut Vec<Vec<RowHeaderCell>>,
    ) {
        if tree.is_empty() {
            return;
        }
        for dim in tree {
            for member in &dim.members {
                let rowspan = count_leaves(&member.children);
                let colspan = if member.children.is_empty() { max_depth - level } else { 1 };

                let target_row = *current_leaf_index;
                row_headers[target_row].push(RowHeaderCell {
                    label: member.label.clone(),
                    key: member.key.clone(),
                    colspan,
                    rowspan,
                });

                if member.children.is_empty() {
                    *current_leaf_index += 1;
                } else {
                    recurse(&member.children, level + 1, max_depth, current_leaf_index, row_headers);
                }
            }
        }
    }

    recurse(tree, 0, max_depth, &mut current_leaf_index, &mut row_headers);
    row_headers
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod golden;
