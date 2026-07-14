use strata_core::{
    Graph, Node, NodeId, NodePayload, Para, Rel, Section, Table, Term, Value, Scalar, Code, List, Anchor, Inline, EmphKind, MathNode, CellValue, DimTree
};
use std::collections::HashMap;

/// グラフから美しい HTML をレンダリングする
pub fn render_to_html(graph: &Graph, root_id: NodeId) -> Result<String, String> {
    let mut renderer = HtmlRenderer::new(graph);
    let content = renderer.render_node(root_id, 1)?;
    
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="ja">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=device-width, initial-scale=1.0">
    <title>Strata Document</title>
    <style>
        :root {{
            --primary-color: #2b3a42;
            --text-color: #333333;
            --bg-color: #ffffff;
            --border-color: #dddddd;
            --header-bg: #f7f9fa;
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, "Noto Sans JP", sans-serif;
            color: var(--text-color);
            background-color: var(--bg-color);
            line-height: 1.6;
            margin: 0;
            padding: 40px 20px;
        }}
        
        .container {{
            max-width: 800px;
            margin: 0 auto;
            background: #fff;
            padding: 40px;
            box-shadow: 0 4px 6px rgba(0,0,0,0.05);
            border-radius: 8px;
            border: 1px solid #eee;
        }}

        h1, h2, h3, h4 {{
            color: var(--primary-color);
            margin-top: 1.5em;
            margin-bottom: 0.5em;
            font-weight: 700;
        }}
        
        h1 {{
            font-size: 2.2rem;
            border-bottom: 3px solid var(--primary-color);
            padding-bottom: 8px;
            margin-top: 0;
            text-align: center;
        }}

        h2 {{
            font-size: 1.5rem;
            border-bottom: 1px solid var(--border-color);
            padding-bottom: 6px;
        }}

        p {{
            margin: 0 0 1em 0;
        }}

        ul, ol {{
            margin: 0 0 1em 0;
            padding-left: 20px;
        }}

        li {{
            margin-bottom: 0.3em;
        }}

        /* Table design */
        table {{
            width: 100%;
            border-collapse: collapse;
            margin: 1.5em 0;
            font-size: 0.95rem;
        }}

        th, td {{
            border: 1px solid var(--border-color);
            padding: 10px 12px;
            text-align: left;
        }}

        th {{
            background-color: var(--header-bg);
            font-weight: 600;
            color: var(--primary-color);
        }}

        td {{
            vertical-align: top;
        }}

        /* Code block */
        pre {{
            background: #f4f4f4;
            border: 1px solid #ddd;
            border-left: 3px solid var(--primary-color);
            padding: 15px;
            overflow-x: auto;
            border-radius: 4px;
        }}

        code {{
            font-family: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, Courier, monospace;
            font-size: 0.9em;
        }}

        /* Math */
        .math-block {{
            margin: 1.5em 0;
            text-align: center;
            overflow-x: auto;
        }}

        /* Utility links */
        a {{
            color: #1a0dab;
            text-decoration: none;
        }}
        a:hover {{
            text-decoration: underline;
        }}

        /* Term definitions & references */
        .term-def {{
            font-weight: bold;
            border-bottom: 1px dashed var(--primary-color);
        }}

        /* Print optimization */
        @media print {{
            body {{
                background-color: #fff;
                padding: 0;
            }}
            .container {{
                box-shadow: none;
                border: none;
                padding: 0;
                max-width: 100%;
            }}
            h1, h2, h3, h4 {{
                page-break-after: avoid;
            }}
            table, pre {{
                page-break-inside: avoid;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        {content}
    </div>
</body>
</html>"#
    );
    
    Ok(html)
}

struct HtmlRenderer<'a> {
    graph: &'a Graph,
}

impl<'a> HtmlRenderer<'a> {
    fn new(graph: &'a Graph) -> Self {
        Self { graph }
    }

    fn render_node(&mut self, node_id: NodeId, depth: usize) -> Result<String, String> {
        let node = self.graph.nodes.get(&node_id)
            .ok_or_else(|| format!("Node not found in graph: {:?}", node_id))?;

        match &node.payload {
            NodePayload::Section(s) => {
                let heading_html = self.render_inlines(&s.heading)?;
                let tag = if depth <= 6 { format!("h{}", depth) } else { "h6".to_string() };
                
                let mut children_html = String::new();
                let children = self.graph.children_of(node_id);
                for child_id in children {
                    children_html.push_str(&self.render_node(child_id, depth + 1)?);
                }

                Ok(format!(
                    "<{tag}>{heading_html}</{tag}>\n{children_html}"
                ))
            }
            NodePayload::Para(p) => {
                let inline_html = self.render_inlines(&p.inline)?;
                Ok(format!("<p>{inline_html}</p>\n"))
            }
            NodePayload::List(l) => {
                let tag = if l.ordered { "ol" } else { "ul" };
                let mut items_html = String::new();
                
                let children = self.graph.children_of(node_id);
                for child_id in children {
                    let child_node = self.graph.nodes.get(&child_id)
                        .ok_or_else(|| format!("Child node not found: {:?}", child_id))?;
                    
                    let child_content = match &child_node.payload {
                        NodePayload::Para(p) => self.render_inlines(&p.inline)?,
                        _ => self.render_node(child_id, depth)?, // ネストされたリストなど
                    };
                    items_html.push_str(&format!("  <li>{child_content}</li>\n"));
                }
                
                Ok(format!("<{tag}>\n{items_html}</{tag}>\n"))
            }
            NodePayload::Table(t) => {
                self.render_table(t)
            }
            NodePayload::Math(m) => {
                let mathml = self.render_math(&m.tree);
                Ok(format!(
                    "<div class=\"math-block\"><math display=\"block\">{mathml}</math></div>\n"
                ))
            }
            NodePayload::Code(c) => {
                let escaped_src = html_escape(&c.src);
                Ok(format!(
                    "<pre><code class=\"language-{}\">{}</code></pre>\n",
                    c.lang, escaped_src
                ))
            }
            NodePayload::Term(t) => {
                // 用語定義
                Ok(format!("<span class=\"term-def\">{}</span>", html_escape(&t.name)))
            }
            NodePayload::Value(v) => {
                let val_str = match &v.scalar {
                    Scalar::Number(n) => n.to_string(),
                    Scalar::Text(s) => html_escape(s),
                    Scalar::Bool(b) => b.to_string(),
                };
                let unit_str = v.unit.as_deref().unwrap_or("");
                Ok(format!("<span class=\"value\">{}{}</span>", val_str, unit_str))
            }
            NodePayload::Anchor(a) => {
                let inner = self.render_inlines(&a.inline)?;
                Ok(format!("<span id=\"anchor-{}\">{}</span>", node_id.0, inner))
            }
            NodePayload::Figure(f) => {
                // 図のレンダリング (今回は簡易実装)
                Ok(format!("<!-- Figure kind={:?} -->", f))
            }
            NodePayload::Document(_) => {
                // 文書ルート(D12)。M3 時点では HTML レンダラの接続対象外
                // (strata-build のスコープ境界)。フォールバックとして無視する。
                Ok(String::new())
            }
        }
    }

    fn render_inlines(&mut self, inlines: &[Inline]) -> Result<String, String> {
        let mut out = String::new();
        for inline in inlines {
            match inline {
                Inline::Text { s } => {
                    out.push_str(&html_escape(s));
                }
                Inline::Emph { kind, children } => {
                    let tag = match kind {
                        EmphKind::Strong => "strong",
                        EmphKind::Em => "em",
                        EmphKind::Code => "code",
                    };
                    let inner = self.render_inlines(children)?;
                    out.push_str(&format!("<{tag}>{inner}</{tag}>"));
                }
                Inline::Math { tree } => {
                    let mathml = self.render_math(tree);
                    out.push_str(&format!("<math>{mathml}</math>"));
                }
                Inline::Ref { to, .. } => {
                    // 他ノードへのリンク。対象ノードの heading などのラベルを解決して表示したいが、
                    // 簡易的に "こちら" や ID、あるいはアンカーテキストとする。
                    // 実際には、ターゲットノードを取得して名前を表示するのが親切。
                    let label = if let Some(target) = self.graph.nodes.get(to) {
                        match &target.payload {
                            NodePayload::Section(s) => {
                                // section の heading をラベルにする
                                self.render_inlines(&s.heading)?
                            }
                            NodePayload::Term(t) => t.name.clone(),
                            _ => "参照".to_string(),
                        }
                    } else {
                        "参照".to_string()
                    };
                    out.push_str(&format!("<a href=\"#anchor-{}\">{}</a>", to.0, label));
                }
                Inline::Term { to, .. } => {
                    if let Some(target) = self.graph.nodes.get(to) {
                        if let NodePayload::Term(t) = &target.payload {
                            out.push_str(&format!("<span class=\"term-ref\">{}</span>", html_escape(&t.name)));
                        }
                    }
                }
                Inline::Anchor { to } => {
                    if let Some(target) = self.graph.nodes.get(to) {
                        if let NodePayload::Anchor(a) = &target.payload {
                            let inner = self.render_inlines(&a.inline)?;
                            out.push_str(&format!("<span id=\"anchor-{}\">{}</span>", to.0, inner));
                        }
                    }
                }
            }
        }
        Ok(out)
    }

    fn render_math(&self, node: &MathNode) -> String {
        match node {
            MathNode::Num { v } => format!("<mn>{}</mn>", html_escape(v)),
            MathNode::Ident { v } => format!("<mi>{}</mi>", html_escape(v)),
            MathNode::Op { v } => format!("<mo>{}</mo>", html_escape(v)),
            MathNode::Row { items } => {
                let inner: String = items.iter().map(|n| self.render_math(n)).collect();
                format!("<mrow>{}</mrow>", inner)
            }
            MathNode::Frac { num, den } => {
                format!(
                    "<mfrac>{}{}</mfrac>",
                    self.render_math(num),
                    self.render_math(den)
                )
            }
            MathNode::Sup { base, sup } => {
                format!(
                    "<msup>{}{}</msup>",
                    self.render_math(base),
                    self.render_math(sup)
                )
            }
            MathNode::Sub { base, sub } => {
                format!(
                    "<msub>{}{}</msub>",
                    self.render_math(base),
                    self.render_math(sub)
                )
            }
            MathNode::SubSup { base, sub, sup } => {
                format!(
                    "<msubsup>{}{}{}</msubsup>",
                    self.render_math(base),
                    self.render_math(sub),
                    self.render_math(sup)
                )
            }
            MathNode::UnderOver { base, under, over } => {
                match (under, over) {
                    (Some(u), Some(o)) => format!(
                        "<munderover>{}{}{}</munderover>",
                        self.render_math(base),
                        self.render_math(u),
                        self.render_math(o)
                    ),
                    (Some(u), None) => format!(
                        "<munder>{}{}</munder>",
                        self.render_math(base),
                        self.render_math(u)
                    ),
                    (None, Some(o)) => format!(
                        "<mover>{}{}</mover>",
                        self.render_math(base),
                        self.render_math(o)
                    ),
                    (None, None) => self.render_math(base),
                }
            }
            MathNode::Sqrt { body } => {
                format!("<msqrt>{}</msqrt>", self.render_math(body))
            }
            MathNode::Root { radicand, index } => {
                format!(
                    "<mroot>{}{}</mroot>",
                    self.render_math(radicand),
                    self.render_math(index)
                )
            }
            MathNode::Fenced { open, close, body } => {
                format!(
                    "<mrow><mo>{}</mo>{}<mo>{}</mo></mrow>",
                    html_escape(open),
                    self.render_math(body),
                    html_escape(close)
                )
            }
            MathNode::Text { s } => format!("<mtext>{}</mtext>", html_escape(s)),
        }
    }

    fn render_table(&mut self, table: &Table) -> Result<String, String> {
        let d_row = max_depth(&table.rows);
        let d_col = max_depth(&table.cols);

        let row_leaves = get_leaves(&table.rows);
        let col_leaves = get_leaves(&table.cols);

        let mut out = String::new();
        out.push_str("<table>\n");

        // 1. Column headers
        if d_col > 0 {
            out.push_str("  <thead>\n");
            let col_headers = build_col_headers(&table.cols, d_col);
            for (level, row) in col_headers.into_iter().enumerate() {
                out.push_str("    <tr>\n");
                
                // 左上隅の空領域
                if level == 0 && d_row > 0 {
                    out.push_str(&format!(
                        "      <th colspan=\"{}\" rowspan=\"{}\">&nbsp;</th>\n",
                        d_row, d_col
                    ));
                }

                for cell in row {
                    let label_html = match &cell.label {
                        Some(inlines) => self.render_inlines(inlines)?,
                        None => html_escape(&cell.key),
                    };
                    
                    let span_attrs = format_span(cell.colspan, cell.rowspan);
                    out.push_str(&format!(
                        "      <th{}>{}</th>\n",
                        span_attrs, label_html
                    ));
                }
                out.push_str("    </tr>\n");
            }
            out.push_str("  </thead>\n");
        }

        // 2. Body
        out.push_str("  <tbody>\n");
        let row_headers = build_row_headers(&table.rows, d_row);

        // データセル高速アクセスのためのマップ構築
        let mut cell_map = HashMap::new();
        for cell in &table.cells {
            cell_map.insert((&cell.row_path, &cell.col_path), &cell.value);
        }

        for r in 0..row_leaves.len() {
            out.push_str("    <tr>\n");
            
            // 行ヘッダ
            if d_row > 0 {
                for cell in &row_headers[r] {
                    let label_html = match &cell.label {
                        Some(inlines) => self.render_inlines(inlines)?,
                        None => html_escape(&cell.key),
                    };
                    
                    let span_attrs = format_span(cell.colspan, cell.rowspan);
                    out.push_str(&format!(
                        "      <th{}>{}</th>\n",
                        span_attrs, label_html
                    ));
                }
            }

            // データセル
            for c in 0..col_leaves.len() {
                let row_path = &row_leaves[r];
                let col_path = &col_leaves[c];
                
                let val_html = match cell_map.get(&(row_path, col_path)) {
                    Some(CellValue::Number { v }) => v.to_string(),
                    Some(CellValue::Text { v }) => html_escape(v),
                    Some(CellValue::Ref { to }) => {
                        let label = if let Some(target) = self.graph.nodes.get(to) {
                            match &target.payload {
                                NodePayload::Value(val) => {
                                    match &val.scalar {
                                        Scalar::Number(n) => n.to_string(),
                                        Scalar::Text(s) => html_escape(s),
                                        Scalar::Bool(b) => b.to_string(),
                                    }
                                }
                                _ => "値".to_string()
                            }
                        } else {
                            "値".to_string()
                        };
                        format!("<a href=\"#anchor-{}\">{}</a>", to.0, label)
                    }
                    Some(CellValue::Quantity { v, unit }) => format!("{} {}", v, html_escape(unit)),
                    Some(CellValue::Empty) | None => "&nbsp;".to_string(),
                };

                out.push_str(&format!("      <td>{}</td>\n", val_html));
            }
            out.push_str("    </tr>\n");
        }
        out.push_str("  </tbody>\n");
        out.push_str("</table>\n");

        Ok(out)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

fn format_span(colspan: usize, rowspan: usize) -> String {
    let mut out = String::new();
    if colspan > 1 {
        out.push_str(&format!(" colspan=\"{}\"", colspan));
    }
    if rowspan > 1 {
        out.push_str(&format!(" rowspan=\"{}\"", rowspan));
    }
    out
}

// 次元の木の深さと葉の数計算
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

#[derive(Clone)]
struct HeaderCell {
    label: Option<Vec<Inline>>,
    key: String,
    colspan: usize,
    rowspan: usize,
}

fn build_col_headers(tree: &DimTree, max_depth: usize) -> Vec<Vec<HeaderCell>> {
    let mut rows = (0..max_depth).map(|_| Vec::new()).collect::<Vec<_>>();
    
    fn recurse(
        tree: &DimTree,
        level: usize,
        max_depth: usize,
        rows: &mut Vec<Vec<HeaderCell>>,
    ) {
        if tree.is_empty() {
            return;
        }
        for dim in tree {
            for member in &dim.members {
                let colspan = count_leaves(&member.children);
                let rowspan = if member.children.is_empty() {
                    max_depth - level
                } else {
                    1
                };
                
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
                let colspan = if member.children.is_empty() {
                    max_depth - level
                } else {
                    1
                };

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
