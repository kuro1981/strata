//! ノード1つの「短いラベル」(近傍要約・`Ref`/`Term` のテキストなし参照先表示に使う)。

use strata_core::{CellValue, Figure, Graph, NodeId, NodePayload, Scalar};

use crate::inline::plain_text;

const MAX_LABEL_CHARS: usize = 60;

/// ノードの種類ごとに、それ単体を指し示すのに十分な短いテキストを作る。
/// サブツリーへは展開しない(D36 スコープ2「ノード自体」の定義に合わせる)。
pub(crate) fn node_short_label(graph: &Graph, id: NodeId) -> String {
    let Some(node) = graph.nodes.get(&id) else {
        return format!("(不明なノード {})", id.0);
    };
    match &node.payload {
        NodePayload::Section(s) => truncate(&plain_text(&s.heading)),
        NodePayload::Para(p) => truncate(&plain_text(&p.inline)),
        NodePayload::List(l) => {
            if l.ordered { "順序リスト".to_string() } else { "箇条書きリスト".to_string() }
        }
        NodePayload::Table(t) => match &t.caption {
            Some(c) => truncate(&plain_text(c)),
            None => "表".to_string(),
        },
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
        NodePayload::Value(v) => {
            let s = match &v.scalar {
                Scalar::Number(n) => n.to_string(),
                Scalar::Text(s) => s.clone(),
                Scalar::Bool(b) => b.to_string(),
            };
            match &v.unit {
                Some(u) => format!("{s}{u}"),
                None => s,
            }
        }
        NodePayload::Document(d) => d.title.clone().unwrap_or_else(|| "文書".to_string()),
        NodePayload::Record(r) => {
            let first = r.entries.first().map(|e| format!("{}: {}", e.key, cell_value_text(&e.value)));
            match first {
                Some(f) => format!("record({}件, 先頭: {})", r.entries.len(), truncate(&f)),
                None => "record(空)".to_string(),
            }
        }
    }
}

/// `CellValue` のプレーンテキスト表現(表/record で共有)。
pub(crate) fn cell_value_text(v: &CellValue) -> String {
    match v {
        CellValue::Number { v } => v.to_string(),
        CellValue::Text { v } => v.clone(),
        CellValue::Ref { to } => format!("→ {}", to.0),
        CellValue::Empty => String::new(),
        CellValue::Quantity { v, unit } => format!("{v} {unit}"),
        CellValue::Date(d) => format_date(d),
        CellValue::Period { from, to } => match to {
            Some(t) => format!("{} 〜 {}", format_date(from), format_date(t)),
            None => format!("{} 〜 現在", format_date(from)),
        },
    }
}

fn format_date(d: &strata_core::DateValue) -> String {
    match d.d {
        Some(day) => format!("{:04}-{:02}-{:02}", d.y, d.m, day),
        None => format!("{:04}-{:02}", d.y, d.m),
    }
}

/// 短いラベルは常に1行に収める前提(エッジ一覧・近傍要約はどれも「1項目=1行」の
/// 体裁のため)。改行・連続空白を単一の半角スペースに畳んでから文字数で切り詰める。
fn truncate(s: &str) -> String {
    let normalized = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = normalized.chars().count();
    if count <= MAX_LABEL_CHARS {
        return normalized;
    }
    let head: String = normalized.chars().take(MAX_LABEL_CHARS).collect();
    format!("{head}…")
}
