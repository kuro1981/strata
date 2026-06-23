use strata_core::{
    Graph, Node, NodeId, NodePayload, Para, Rel, Section, Table, Term, Value, Scalar, Code, List, Anchor, Inline, EmphKind, MathNode, CellValue, DimTree
};
use std::collections::HashMap;

/// グラフから美しい Typst ソースコードをレンダリングする
pub fn render_to_typst(graph: &Graph, root_id: NodeId) -> Result<String, String> {
    let mut renderer = TypstRenderer::new(graph);
    let content = renderer.render_node(root_id, 1)?;
    
    let doc = format!(
        r##"// Strata Document - Generated Typst Source

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

{content}
"##
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

    fn render_node(&mut self, node_id: NodeId, depth: usize) -> Result<String, String> {
        let node = self.graph.nodes.get(&node_id)
            .ok_or_else(|| format!("Node not found in graph: {:?}", node_id))?;

        match &node.payload {
            NodePayload::Section(s) => {
                let heading_typst = self.render_inlines(&s.heading)?;
                let prefix = "=".repeat(depth);
                
                let mut children_html = String::new();
                let children = self.graph.children_of(node_id);
                for child_id in children {
                    children_html.push_str(&self.render_node(child_id, depth + 1)?);
                }

                Ok(format!(
                    "{} {} <anchor-{}>\n\n{}",
                    prefix, heading_typst, node_id.0, children_html
                ))
            }
            NodePayload::Para(p) => {
                let inline_typst = self.render_inlines(&p.inline)?;
                Ok(format!("{} <anchor-{}>\n\n", inline_typst, node_id.0))
            }
            NodePayload::List(l) => {
                let marker = if l.ordered { "+" } else { "-" };
                let mut items_typst = String::new();
                
                let children = self.graph.children_of(node_id);
                for child_id in children {
                    let child_node = self.graph.nodes.get(&child_id)
                        .ok_or_else(|| format!("Child node not found: {:?}", child_id))?;
                    
                    let child_content = match &child_node.payload {
                        NodePayload::Para(p) => self.render_inlines(&p.inline)?,
                        _ => self.render_node(child_id, depth)?, // ネストされたリストなど
                    };
                    items_typst.push_str(&format!("{} {} <anchor-{}>\n", marker, child_content, child_id.0));
                }
                
                Ok(format!("{}\n", items_typst))
            }
            NodePayload::Table(t) => {
                self.render_table(t)
            }
            NodePayload::Math(m) => {
                let math_str = self.render_math(&m.tree);
                Ok(format!(
                    "$ {} $ <anchor-{}>\n\n",
                    math_str, node_id.0
                ))
            }
            NodePayload::Code(c) => {
                Ok(format!(
                    "```{}\n{}\n``` <anchor-{}>\n\n",
                    c.lang, c.src, node_id.0
                ))
            }
            NodePayload::Term(t) => {
                Ok(format!("*{}*", typst_escape(&t.name)))
            }
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
                Ok(format!("[] <anchor-{}> {}", node_id.0, inner))
            }
            NodePayload::Figure(f) => {
                Ok(format!("// Figure: {:?}\n", f))
            }
        }
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
                Inline::Ref { to, .. } => {
                    let label = if let Some(target) = self.graph.nodes.get(to) {
                        match &target.payload {
                            NodePayload::Section(s) => {
                                self.render_inlines(&s.heading)?
                            }
                            NodePayload::Term(t) => t.name.clone(),
                            _ => "参照".to_string(),
                        }
                    } else {
                        "参照".to_string()
                    };
                    out.push_str(&format!("#link(<anchor-{}>)[{}]", to.0, label));
                }
                Inline::Term { to } => {
                    if let Some(target) = self.graph.nodes.get(to) {
                        if let NodePayload::Term(t) = &target.payload {
                            out.push_str(&format!("*{}*", typst_escape(&t.name)));
                        }
                    }
                }
                Inline::Anchor { to } => {
                    if let Some(target) = self.graph.nodes.get(to) {
                        if let NodePayload::Anchor(a) = &target.payload {
                            let inner = self.render_inlines(&a.inline)?;
                            out.push_str(&format!("[] <anchor-{}> {}", to.0, inner));
                        }
                    }
                }
            }
        }
        Ok(out)
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
                format!(
                    "({}) / ({})",
                    self.render_math(num),
                    self.render_math(den)
                )
            }
            MathNode::Sup { base, sup } => {
                format!(
                    "({})^({})",
                    self.render_math(base),
                    self.render_math(sup)
                )
            }
            MathNode::Sub { base, sub } => {
                format!(
                    "({})_({})",
                    self.render_math(base),
                    self.render_math(sub)
                )
            }
            MathNode::SubSup { base, sub, sup } => {
                format!(
                    "({})_({})^({})",
                    self.render_math(base),
                    self.render_math(sub),
                    self.render_math(sup)
                )
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
            MathNode::Sqrt { body } => {
                format!("sqrt({})", self.render_math(body))
            }
            MathNode::Root { radicand, index } => {
                format!("root({}, {})", self.render_math(index), self.render_math(radicand))
            }
            MathNode::Fenced { open, close, body } => {
                // Typst の fences 表現
                format!("{} {} {}", open, self.render_math(body), close)
            }
            MathNode::Text { s } => format!("\"{}\"", s.replace('"', "\\\"")),
        }
    }

    fn render_table(&mut self, table: &Table) -> Result<String, String> {
        let d_row = max_depth(&table.rows);
        let d_col = max_depth(&table.cols);

        let row_leaves = get_leaves(&table.rows);
        let col_leaves = get_leaves(&table.cols);

        let mut out = String::new();
        
        // table定義の生成
        // カラム比率：行ヘッダ側を少し狭く、データ側を均等に
        let mut col_specs = Vec::new();
        for _ in 0..d_row {
            col_specs.push("auto".to_string());
        }
        for _ in 0..col_leaves.len() {
            col_specs.push("1fr".to_string());
        }
        
        out.push_str(&format!(
            "#table(\n  columns: ({}),\n  stroke: 0.5pt + luma(150),\n  fill: (x, y) => if y < {} or x < {} {{ rgb(\"#f7f9fa\") }} else {{ none }},\n",
            col_specs.join(", "),
            d_col,
            d_row
        ));

        // 1. Column headers
        if d_col > 0 {
            let col_headers = build_col_headers(&table.cols, d_col);
            for (level, row) in col_headers.into_iter().enumerate() {
                // 左上隅の空領域
                if level == 0 && d_row > 0 {
                    out.push_str(&format!(
                        "  table.cell(colspan: {}, rowspan: {})[],\n",
                        d_row, d_col
                    ));
                }

                for cell in row {
                    let label_typst = match &cell.label {
                        Some(inlines) => self.render_inlines(inlines)?,
                        None => typst_escape(&cell.key),
                    };
                    
                    let span_attrs = format_span(cell.colspan, cell.rowspan);
                    out.push_str(&format!(
                        "  table.cell{}[*{}*],\n",
                        span_attrs, label_typst
                    ));
                }
            }
        }

        // 2. Body
        let row_headers = build_row_headers(&table.rows, d_row);

        // データセル高速アクセスのためのマップ構築
        let mut cell_map = HashMap::new();
        for cell in &table.cells {
            cell_map.insert((&cell.row_path, &cell.col_path), &cell.value);
        }

        for r in 0..row_leaves.len() {
            // 行ヘッダ
            if d_row > 0 {
                for cell in &row_headers[r] {
                    let label_typst = match &cell.label {
                        Some(inlines) => self.render_inlines(inlines)?,
                        None => typst_escape(&cell.key),
                    };
                    
                    let span_attrs = format_span(cell.colspan, cell.rowspan);
                    out.push_str(&format!(
                        "  table.cell{}[*{}*],\n",
                        span_attrs, label_typst
                    ));
                }
            }

            // データセル
            for c in 0..col_leaves.len() {
                let row_path = &row_leaves[r];
                let col_path = &col_leaves[c];
                
                let val_typst = match cell_map.get(&(row_path, col_path)) {
                    Some(CellValue::Number { v }) => v.to_string(),
                    Some(CellValue::Text { v }) => self.render_inlines(&[Inline::Text { s: v.clone() }])?,
                    Some(CellValue::Ref { to }) => {
                        let label = if let Some(target) = self.graph.nodes.get(to) {
                            match &target.payload {
                                NodePayload::Value(val) => {
                                    match &val.scalar {
                                        Scalar::Number(n) => n.to_string(),
                                        Scalar::Text(s) => typst_escape(s),
                                        Scalar::Bool(b) => b.to_string(),
                                    }
                                }
                                _ => "値".to_string()
                            }
                        } else {
                            "値".to_string()
                        };
                        format!("#link(<anchor-{}>)[{}]", to.0, label)
                    }
                    Some(CellValue::Empty) | None => "".to_string(),
                };

                out.push_str(&format!("  [{}],\n", val_typst));
            }
        }
        out.push_str(")\n\n");

        Ok(out)
    }
}

fn typst_escape(s: &str) -> String {
    // Typst 特殊文字をエスケープ
    // * _ ` $ < > @ # & \ 等
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

fn typst_math_escape(s: &str) -> String {
    // 数式内でのエスケープ
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
