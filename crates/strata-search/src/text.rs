//! ノード payload → 検索用テキストの抽出(strata-context の inline.rs/label.rs と
//! 同種のロジックだが、strata-search は strata-context に依存しない(用途が違う —
//! こちらは「引用可能な清書」ではなく「マッチ判定用の生テキスト」)ため、
//! 必要分だけこのクレート内に持つ(小さな重複、裁量。最終報告参照)。

use strata_core::{CellValue, DateValue, Figure, Inline, MathNode, NodePayload};

/// ノード1つぶんの検索用テキスト。
pub(crate) struct NodeTexts {
    /// 「見出し的」テキスト(Section の見出し、Table/Figure の caption、Term の name、
    /// Document の title)。存在すればランキングの一次シグナル・表示ラベルの一次候補に使う。
    pub heading: Option<String>,
    /// マッチ判定に使う全文(見出し的テキストを含む。空文字列もありうる — 例: List
    /// 自身は子を持つだけでテキストを持たない)。
    pub body: String,
}

/// `NodePayload` から `NodeTexts` を組み立てる。
pub(crate) fn node_texts(payload: &NodePayload) -> NodeTexts {
    match payload {
        NodePayload::Section(s) => {
            let h = plain_text(&s.heading);
            NodeTexts { heading: Some(h.clone()), body: h }
        }
        NodePayload::Para(p) => NodeTexts { heading: None, body: plain_text(&p.inline) },
        NodePayload::List(_) => NodeTexts { heading: None, body: String::new() },
        NodePayload::Table(t) => {
            let caption = t.caption.as_ref().map(|c| plain_text(c));
            let mut body = caption.clone().unwrap_or_default();
            for dim in &t.rows {
                push_dim_text(dim, &mut body);
            }
            for dim in &t.cols {
                push_dim_text(dim, &mut body);
            }
            for cell in &t.cells {
                let v = cell_value_text(&cell.value);
                if !v.is_empty() {
                    body.push(' ');
                    body.push_str(&v);
                }
            }
            NodeTexts { heading: caption, body }
        }
        NodePayload::Math(m) => NodeTexts { heading: None, body: math_leaf_text(&m.tree) },
        NodePayload::Figure(Figure::Chart(c)) => {
            let caption = c.caption.as_ref().map(|cap| plain_text(cap));
            let mut body = caption.clone().unwrap_or_default();
            for v in c.depicts.values() {
                body.push(' ');
                body.push_str(v);
            }
            NodeTexts { heading: caption, body }
        }
        NodePayload::Figure(Figure::Image(img)) => {
            let caption = img.caption.as_ref().map(|cap| plain_text(cap));
            let mut body = caption.clone().unwrap_or_default();
            body.push(' ');
            body.push_str(&img.alt);
            for v in img.depicts.values() {
                body.push(' ');
                body.push_str(v);
            }
            NodeTexts { heading: caption, body }
        }
        NodePayload::Code(c) => NodeTexts { heading: None, body: format!("{} {}", c.lang, c.src) },
        NodePayload::Term(t) => NodeTexts { heading: Some(t.name.clone()), body: t.name.clone() },
        NodePayload::Anchor(a) => NodeTexts { heading: None, body: plain_text(&a.inline) },
        NodePayload::Value(v) => {
            let s = match &v.scalar {
                strata_core::Scalar::Number(n) => n.to_string(),
                strata_core::Scalar::Text(s) => s.clone(),
                strata_core::Scalar::Bool(b) => b.to_string(),
            };
            let body = match &v.unit {
                Some(u) => format!("{s} {u}"),
                None => s,
            };
            NodeTexts { heading: None, body }
        }
        NodePayload::Document(d) => NodeTexts { heading: d.title.clone(), body: d.title.clone().unwrap_or_default() },
        NodePayload::Record(r) => {
            let mut body = String::new();
            for e in &r.entries {
                if !body.is_empty() {
                    body.push('\n');
                }
                body.push_str(&e.key);
                body.push_str(": ");
                body.push_str(&cell_value_text(&e.value));
            }
            NodeTexts { heading: None, body }
        }
        NodePayload::Quote(_) => NodeTexts { heading: None, body: String::new() },
        NodePayload::ThematicBreak(_) => NodeTexts { heading: None, body: String::new() },
    }
}

fn push_dim_text(dim: &strata_core::Dim, out: &mut String) {
    out.push(' ');
    out.push_str(&dim.name);
    for m in &dim.members {
        out.push(' ');
        out.push_str(&m.key);
        if let Some(label) = &m.label {
            out.push(' ');
            out.push_str(&plain_text(label));
        }
        for child in &m.children {
            push_dim_text(child, out);
        }
    }
}

/// `strata-typst`/`strata-context` の `node_type` に相当する snake_case のノード種別名
/// (strata-core の `#[serde(tag = "type", rename_all = "snake_case")]` と同じ字句)。
pub(crate) fn node_type_name(payload: &NodePayload) -> &'static str {
    match payload {
        NodePayload::Section(_) => "section",
        NodePayload::Para(_) => "para",
        NodePayload::List(_) => "list",
        NodePayload::Table(_) => "table",
        NodePayload::Math(_) => "math",
        NodePayload::Figure(_) => "figure",
        NodePayload::Code(_) => "code",
        NodePayload::Term(_) => "term",
        NodePayload::Anchor(_) => "anchor",
        NodePayload::Value(_) => "value",
        NodePayload::Document(_) => "document",
        NodePayload::Record(_) => "record",
        NodePayload::Quote(_) => "quote",
        NodePayload::ThematicBreak(_) => "thematic_break",
    }
}

/// インライン列から整形を捨てたプレーンテキストを合成する(strata-context::inline::plain_text
/// と同じロジック、依存を避けるための小さな重複)。
pub(crate) fn plain_text(inlines: &[Inline]) -> String {
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

/// `MathNode` の葉(Num/Ident/Op/Text)を空白区切りで連結しただけの検索用テキスト
/// (組版はしない。変数名・演算子トークンで数式ブロックを見つけられれば v0 としては十分)。
fn math_leaf_text(node: &MathNode) -> String {
    let mut out = String::new();
    collect_math_leaves(node, &mut out);
    out
}

fn collect_math_leaves(node: &MathNode, out: &mut String) {
    let mut push = |s: &str| {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(s);
    };
    match node {
        MathNode::Num { v } | MathNode::Ident { v } | MathNode::Op { v } => push(v),
        MathNode::Text { s } => push(s),
        MathNode::Row { items } => {
            for i in items {
                collect_math_leaves(i, out);
            }
        }
        MathNode::Frac { num, den } => {
            collect_math_leaves(num, out);
            collect_math_leaves(den, out);
        }
        MathNode::Sup { base, sup } => {
            collect_math_leaves(base, out);
            collect_math_leaves(sup, out);
        }
        MathNode::Sub { base, sub } => {
            collect_math_leaves(base, out);
            collect_math_leaves(sub, out);
        }
        MathNode::SubSup { base, sub, sup } => {
            collect_math_leaves(base, out);
            collect_math_leaves(sub, out);
            collect_math_leaves(sup, out);
        }
        MathNode::UnderOver { base, under, over } => {
            collect_math_leaves(base, out);
            if let Some(u) = under {
                collect_math_leaves(u, out);
            }
            if let Some(o) = over {
                collect_math_leaves(o, out);
            }
        }
        MathNode::Sqrt { body } => collect_math_leaves(body, out),
        MathNode::Root { radicand, index } => {
            collect_math_leaves(radicand, out);
            collect_math_leaves(index, out);
        }
        MathNode::Fenced { body, .. } => collect_math_leaves(body, out),
    }
}

/// `CellValue` のプレーンテキスト表現(strata-context::label::cell_value_text と同じ、
/// 依存を避けるための小さな重複)。
pub(crate) fn cell_value_text(v: &CellValue) -> String {
    match v {
        CellValue::Number { v } => v.to_string(),
        CellValue::Text { v } => v.clone(),
        CellValue::Ref { to } => to.0.to_string(),
        CellValue::Empty => String::new(),
        CellValue::Quantity { v, unit } => format!("{v} {unit}"),
        CellValue::Date(d) => format_date(d),
        CellValue::Period { from, to } => match to {
            Some(t) => format!("{} 〜 {}", format_date(from), format_date(t)),
            None => format!("{} 〜 現在", format_date(from)),
        },
    }
}

fn format_date(d: &DateValue) -> String {
    match d.d {
        Some(day) => format!("{:04}-{:02}-{:02}", d.y, d.m, day),
        None => format!("{:04}-{:02}", d.y, d.m),
    }
}
