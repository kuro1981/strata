//! セレクタ解決とコンビネータ評価(実行器)。
//!
//! 決定性(WP-W1 要件: 同一入力・同一定義 → バイト同一出力)は、この評価器が
//! グラフの反復順序として「表: 宣言順(row_order)」「contains: ord 昇順
//! (graph.children_of が既に保証)」だけを使い、HashMap 反復や乱択に依存しない
//! ことで担保する。
use crate::ast::{AsType, Combinator, ExtendPath, RowSource, Selector};
use crate::value::YValue;
use std::collections::{BTreeMap, HashMap, HashSet};
use strata_core::{CellValue, DateValue, Graph, Node, NodeId, NodePayload};

pub struct EvalContext<'g> {
    pub graph: &'g Graph,
    /// 無修飾 alias の解決表。単一文書モードでは全ノードの alias をそのまま
    /// フラットに写す(従来どおり — 1文書内では build 時点で alias 一意性が
    /// 保証済み)。ワークスペースモード(`new_workspace`)では**ワークスペース全体で
    /// 一意な alias だけ**を載せる(WP-W3: 複数文書が同名の alias を持つのは
    /// 正当 — D42「無修飾 alias = 同一文書」— なので、フラットな1個のグローバル
    /// 表に押し込めない alias は `ambiguous` 側へ回す)。
    alias_index: HashMap<&'g str, NodeId>,
    /// ワークスペースモードで、複数文書にまたがって定義されている alias 名
    /// (無修飾では解決できない — `doc:` で明示するよう案内する)。単一文書モードでは
    /// 常に空。
    ambiguous: HashSet<&'g str>,
    /// ワークスペースモード専用: 文書 alias → (ブロック alias → NodeId)。
    /// `strata_build::WorkspaceBuildOutput::doc_aliases` をそのまま借用する
    /// (WP-W3: `{ alias: X, doc: Y }` の解決に使う)。単一文書モードでは `None`。
    doc_alias_index: Option<&'g BTreeMap<String, BTreeMap<String, NodeId>>>,
    /// D46: `contains` の子→親逆引き表(strata_core::parent_index)。`join` の
    /// `include-only-class`/`exclude-class` を実効 class(自身+祖先の和集合)で
    /// 判定するために使う(render --hide / context --class と同じ定義に統一)。
    parents: HashMap<NodeId, NodeId>,
    pub touched: std::cell::RefCell<HashSet<NodeId>>,
    /// `touched` の部分集合: `{ alias: X, doc: Y }` の明示的な doc 修飾で解決した
    /// ノード(WP-W3)。`check_workspace` の「未使用ノード」走査(WP-W4 で追加)が、
    /// 「1フィールドだけ doc: で借りてきた文書」を、その文書全体が使われた
    /// ことにして丸ごとスキャン対象へ広げてしまわないようにするための印
    /// (crate::check のドキュメント参照)。単一文書モードでは常に空。
    pub doc_qualified_touched: std::cell::RefCell<HashSet<NodeId>>,
    pub warnings: std::cell::RefCell<Vec<String>>,
}

impl<'g> EvalContext<'g> {
    pub fn new(graph: &'g Graph) -> Self {
        let mut alias_index = HashMap::new();
        for (id, node) in &graph.nodes {
            if let Some(a) = &node.alias {
                alias_index.insert(a.as_str(), *id);
            }
        }
        EvalContext {
            graph,
            alias_index,
            ambiguous: HashSet::new(),
            doc_alias_index: None,
            parents: strata_core::parent_index(graph),
            touched: std::cell::RefCell::new(HashSet::new()),
            doc_qualified_touched: std::cell::RefCell::new(HashSet::new()),
            warnings: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// ワークスペースモード用のコンストラクタ(WP-W3)。`doc_aliases` は
    /// `strata_build::build_workspace` が返す文書 alias → ブロック alias 表
    /// (`WorkspaceBuildOutput::doc_aliases`)。無修飾 alias の解決規則は
    /// 「ワークスペース全体で一意なら解決・複数文書にまたがれば `doc:` を要求する
    /// エラー」(裁量、docs/view-def-v1.md 参照)。
    pub fn new_workspace(graph: &'g Graph, doc_aliases: &'g BTreeMap<String, BTreeMap<String, NodeId>>) -> Self {
        let mut counts: HashMap<&str, u32> = HashMap::new();
        let mut first: HashMap<&str, NodeId> = HashMap::new();
        for (id, node) in &graph.nodes {
            if let Some(a) = &node.alias {
                *counts.entry(a.as_str()).or_insert(0) += 1;
                first.entry(a.as_str()).or_insert(*id);
            }
        }
        let alias_index: HashMap<&str, NodeId> =
            first.into_iter().filter(|(a, _)| counts[a] == 1).collect();
        let ambiguous: HashSet<&str> = counts.into_iter().filter(|(_, c)| *c > 1).map(|(a, _)| a).collect();
        EvalContext {
            graph,
            alias_index,
            ambiguous,
            doc_alias_index: Some(doc_aliases),
            parents: strata_core::parent_index(graph),
            touched: std::cell::RefCell::new(HashSet::new()),
            doc_qualified_touched: std::cell::RefCell::new(HashSet::new()),
            warnings: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn node(&self, id: NodeId) -> &'g Node {
        self.graph.nodes.get(&id).expect("dangling NodeId in graph")
    }

    fn mark(&self, id: NodeId) {
        self.touched.borrow_mut().insert(id);
    }

    /// `mark` に加え、`doc_qualified_touched` にも記録する(WP-W3 の `doc:` 修飾
    /// alias セレクタ専用)。
    fn mark_doc_qualified(&self, id: NodeId) {
        self.touched.borrow_mut().insert(id);
        self.doc_qualified_touched.borrow_mut().insert(id);
    }

    fn warn(&self, msg: String) {
        self.warnings.borrow_mut().push(msg);
    }
}

#[derive(Clone, Default)]
pub struct Scope {
    pub row_path: Vec<String>,
    pub current_table: Option<NodeId>,
    pub current_node: Option<NodeId>,
}

pub enum Resolved {
    Node(NodeId),
    Value(CellValue),
}

pub type EvalResult<T> = Result<T, String>;

// --------------------------------------------------------------------------
// セレクタ解決
// --------------------------------------------------------------------------

pub fn resolve_selector(ctx: &EvalContext, scope: &Scope, sel: &Selector) -> EvalResult<Resolved> {
    match sel {
        // WP-W3(D43): `doc` が指定されていれば文書スコープ修飾解決(ワークスペース
        // モード専用)。無指定なら従来どおりの無修飾解決(単一文書モードはそのまま、
        // ワークスペースモードは「全体で一意なら解決・複数文書にまたがれば
        // エラー」— `new_workspace` が事前に計算した `ambiguous` を見る)。
        Selector::Alias { alias: name, doc: Some(d) } => {
            let idx = ctx.doc_alias_index.ok_or_else(|| {
                format!("alias '{name}' に doc: '{d}' が指定されていますが、ワークスペースモードではありません(`--workspace` を使ってください)")
            })?;
            let doc_table = idx
                .get(d.as_str())
                .ok_or_else(|| format!("doc '{d}' が見つかりません(ワークスペースに frontmatter alias '{d}' を持つ文書がありません)"))?;
            let id = *doc_table
                .get(name.as_str())
                .ok_or_else(|| format!("文書 '{d}' に alias '{name}' が見つかりません"))?;
            ctx.mark_doc_qualified(id);
            Ok(Resolved::Node(id))
        }
        Selector::Alias { alias: name, doc: None } => {
            if ctx.ambiguous.contains(name.as_str()) {
                return Err(format!(
                    "alias '{name}' は複数の文書に存在するため無修飾では解決できません(`doc:` で文書を指定してください)"
                ));
            }
            let id = *ctx
                .alias_index
                .get(name.as_str())
                .ok_or_else(|| format!("alias '{name}' が見つかりません"))?;
            ctx.mark(id);
            Ok(Resolved::Node(id))
        }
        Selector::Class(name) => {
            let mut matches: Vec<NodeId> = ctx
                .graph
                .nodes
                .iter()
                .filter(|(_, n)| n.classes.iter().any(|c| c == name))
                .map(|(id, _)| *id)
                .collect();
            matches.sort();
            match matches.len() {
                0 => Err(format!("class '{name}' を持つノードが見つかりません")),
                1 => {
                    ctx.mark(matches[0]);
                    Ok(Resolved::Node(matches[0]))
                }
                n => {
                    ctx.warn(format!(
                        "class '{name}' に一致するノードが{n}件あります。最初の1件を使います"
                    ));
                    ctx.mark(matches[0]);
                    Ok(Resolved::Node(matches[0]))
                }
            }
        }
        Selector::HeadingText(text) => {
            ctx.warn(format!(
                "heading-text セレクタ '{text}' は見出しテキスト一致(頑健性が低い、D31)を使っています。可能なら alias に置き換えてください"
            ));
            let mut found = None;
            for (id, n) in &ctx.graph.nodes {
                if let NodePayload::Section(s) = &n.payload
                    && flatten_inline(&s.heading) == *text
                {
                    found = Some(*id);
                    break;
                }
            }
            let id = found.ok_or_else(|| format!("heading-text '{text}' に一致する見出しが見つかりません"))?;
            ctx.mark(id);
            Ok(Resolved::Node(id))
        }
        Selector::RecordField { of, key } => {
            let Resolved::Node(id) = resolve_selector(ctx, scope, of)? else {
                return Err("record-field の of は値ではなくノードを指す必要があります".to_string());
            };
            let node = ctx.node(id);
            let NodePayload::Record(rec) = &node.payload else {
                return Err(format!("record-field の対象ノードが record ではありません(id={id:?})"));
            };
            let entry = rec
                .entries
                .iter()
                .find(|e| e.key == *key)
                .ok_or_else(|| format!("record に key '{key}' が見つかりません"))?;
            Ok(Resolved::Value(entry.value.clone()))
        }
        Selector::Cell { of, col, row } => {
            let table_id = match of {
                Some(sel) => match resolve_selector(ctx, scope, sel)? {
                    Resolved::Node(id) => id,
                    Resolved::Value(_) => return Err("cell の of はノードを指す必要があります".to_string()),
                },
                None => scope
                    .current_table
                    .ok_or_else(|| "cell に of が無く、現在の表(rows: table)も確立していません".to_string())?,
            };
            let node = ctx.node(table_id);
            let NodePayload::Table(t) = &node.payload else {
                return Err(format!("cell の対象ノードが table ではありません(id={table_id:?})"));
            };
            let row_path: &[String] = row.as_deref().unwrap_or(&scope.row_path);
            let cell = t
                .cells
                .iter()
                .find(|c| c.row_path == row_path && c.col_path.first().map(|s| s.as_str()) == Some(col.as_str()))
                .ok_or_else(|| format!("table に row_path={row_path:?} col='{}' のセルが見つかりません", col))?;
            Ok(Resolved::Value(cell.value.clone()))
        }
        Selector::AliasFromRow { prefix, segment } => {
            let seg = scope
                .row_path
                .get(*segment)
                .ok_or_else(|| format!("row_path に segment {segment} がありません(row_path={:?})", scope.row_path))?;
            let alias = format!("{prefix}{seg}");
            let id = *ctx
                .alias_index
                .get(alias.as_str())
                .ok_or_else(|| format!("alias-from-row: alias '{alias}' が見つかりません"))?;
            ctx.mark(id);
            Ok(Resolved::Node(id))
        }
        Selector::FirstChildOfType { of, node_type } => {
            let Resolved::Node(parent) = resolve_selector(ctx, scope, of)? else {
                return Err("first-child-of-type の of はノードを指す必要があります".to_string());
            };
            for child in ctx.graph.children_of(parent) {
                if node_type_name(&ctx.node(child).payload) == node_type {
                    ctx.mark(child);
                    return Ok(Resolved::Node(child));
                }
            }
            Err(format!("type='{node_type}' の子ノードが見つかりません(parent={parent:?})"))
        }
        Selector::SelfNode => {
            let id = scope.current_node.ok_or_else(|| "self セレクタですが現在のスコープノードがありません".to_string())?;
            Ok(Resolved::Node(id))
        }
    }
}

pub fn node_type_name(p: &NodePayload) -> &'static str {
    match p {
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
        // M6(D40)。
        NodePayload::Quote(_) => "quote",
        NodePayload::ThematicBreak(_) => "thematic-break",
    }
}

fn flatten_inline(inline: &[strata_core::Inline]) -> String {
    use strata_core::Inline as I;
    let mut out = String::new();
    for seg in inline {
        match seg {
            I::Text { s } => out.push_str(s),
            I::Emph { children, .. } => out.push_str(&flatten_inline(children)),
            I::Ref { text, .. } => out.push_str(text),
            I::Term { text, .. } => out.push_str(text),
            // M6(D40): 外部リンクは表示テキスト、画像は alt。
            I::Link { text, .. } => out.push_str(text),
            I::Image { alt, .. } => out.push_str(alt),
            I::Anchor { .. } => {}
            I::Math { .. } => {}
        }
    }
    out
}

/// ノードの「読める本文テキスト」(見出し/段落/アンカー/用語)。
pub fn text_of_node(node: &Node) -> String {
    match &node.payload {
        NodePayload::Section(s) => flatten_inline(&s.heading),
        NodePayload::Para(p) => flatten_inline(&p.inline),
        NodePayload::Anchor(a) => flatten_inline(&a.inline),
        NodePayload::Term(t) => t.name.clone(),
        NodePayload::Document(d) => d.title.clone().unwrap_or_default(),
        _ => String::new(),
    }
}

/// CellValue の素朴な文字列化(v0 の cellvalue_text 相当)。
pub fn cellvalue_text(v: &CellValue) -> String {
    match v {
        CellValue::Text { v } => v.clone(),
        CellValue::Number { v } => format_f64(*v),
        CellValue::Quantity { v, unit } => format!("{}{}", format_f64(*v), unit),
        CellValue::Empty => String::new(),
        CellValue::Ref { .. } => String::new(),
        CellValue::Date(d) => format_date(d, "YYYY-MM-DD"),
        CellValue::Period { from, to } => match to {
            Some(t) => format!("{} ~ {}", format_date(from, "YYYY-MM-DD"), format_date(t, "YYYY-MM-DD")),
            None => format!("{} ~", format_date(from, "YYYY-MM-DD")),
        },
    }
}

fn format_f64(v: f64) -> String {
    if v.fract() == 0.0 { format!("{}", v as i64) } else { v.to_string() }
}

/// date コンビネータの書式トークン展開(YYYY/YY/M/MM/D/DD、他はリテラル通過)。
pub fn format_date(d: &DateValue, fmt: &str) -> String {
    let chars: Vec<char> = fmt.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if matches(&chars, i, "YYYY") {
            out.push_str(&format!("{:04}", d.y));
            i += 4;
        } else if matches(&chars, i, "YY") {
            out.push_str(&format!("{:02}", d.y.rem_euclid(100)));
            i += 2;
        } else if matches(&chars, i, "MM") {
            out.push_str(&format!("{:02}", d.m));
            i += 2;
        } else if chars[i] == 'M' {
            out.push_str(&d.m.to_string());
            i += 1;
        } else if matches(&chars, i, "DD") {
            out.push_str(&format!("{:02}", d.d.unwrap_or(0)));
            i += 2;
        } else if chars[i] == 'D' {
            if let Some(day) = d.d {
                out.push_str(&day.to_string());
            }
            i += 1;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn matches(chars: &[char], i: usize, pat: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    if i + p.len() > chars.len() {
        return false;
    }
    chars[i..i + p.len()] == p[..]
}

// --------------------------------------------------------------------------
// 表の行順・セル索引(v0 の walk_dim/row_order/cell_grid 相当)
// --------------------------------------------------------------------------

fn walk_dim(dim: &strata_core::Dim, prefix: &[String], out: &mut Vec<Vec<String>>) {
    for m in &dim.members {
        let mut path = prefix.to_vec();
        path.push(m.key.clone());
        if m.children.is_empty() {
            out.push(path);
        } else {
            for child in &m.children {
                walk_dim(child, &path, out);
            }
        }
    }
}

pub fn row_order(t: &strata_core::Table) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    for d in &t.rows {
        walk_dim(d, &[], &mut out);
    }
    out
}

// --------------------------------------------------------------------------
// コンビネータ評価
// --------------------------------------------------------------------------

pub fn eval(ctx: &EvalContext, scope: &Scope, comb: &Combinator) -> EvalResult<YValue> {
    match comb {
        Combinator::Fields(entries) => {
            let mut out = Vec::with_capacity(entries.len());
            for (k, c) in entries {
                let v = eval(ctx, scope, c).map_err(|e| format!("{k}: {e}"))?;
                out.push((k.clone(), v));
            }
            Ok(YValue::Map(out))
        }
        Combinator::Rows { source, item } => eval_rows(ctx, scope, source, item),
        Combinator::Join { of, separator, nested_prefix, include_only_class, exclude_class, keys } => {
            eval_join(ctx, scope, of, separator, nested_prefix.as_deref(), include_only_class.as_deref(), exclude_class.as_deref(), keys.as_deref())
        }
        Combinator::Date { of, format, period_separator, period_open, as_type } => {
            let resolved = resolve_selector(ctx, scope, of)?;
            let Resolved::Value(cv) = resolved else {
                return Err("date の of は値(セル/record フィールド)を指す必要があります".to_string());
            };
            let text = match &cv {
                CellValue::Date(d) => format_date(d, format),
                CellValue::Period { from, to } => {
                    let sep = period_separator
                        .as_deref()
                        .ok_or_else(|| "period 値には period-separator が必要です".to_string())?;
                    let from_s = format_date(from, format);
                    let to_s = match to {
                        Some(t) => format_date(t, format),
                        None => period_open
                            .clone()
                            .ok_or_else(|| "終了日の無い period には period-open が必要です".to_string())?,
                    };
                    format!("{from_s}{sep}{to_s}")
                }
                CellValue::Text { v } => v.clone(),
                CellValue::Empty => String::new(),
                other => return Err(format!("date の of が Date/Period/Text ではありません({other:?})")),
            };
            cast(text, *as_type)
        }
        Combinator::Age { birth, as_of, as_type } => {
            let b = expect_date(resolve_selector(ctx, scope, birth)?, "age.birth")?;
            let a = expect_date(resolve_selector(ctx, scope, as_of)?, "age.as-of")?;
            let mut age = a.y - b.y;
            if (a.m, a.d.unwrap_or(0)) < (b.m, b.d.unwrap_or(0)) {
                age -= 1;
            }
            cast(age.to_string(), *as_type)
        }
        Combinator::Literal(v) => Ok(v.clone()),
        Combinator::Pick { of, as_type } => {
            let resolved = resolve_selector(ctx, scope, of)?;
            let text = match resolved {
                Resolved::Value(v) => cellvalue_text(&v),
                Resolved::Node(id) => text_of_node(ctx.node(id)),
            };
            cast(text, *as_type)
        }
        // D45: parts を順に評価し、YValue::as_text() で文字列化してから separator で
        // 連結する(実需2件: cv 氏名の姓+名結合、tech-stack の details+level 結合)。
        Combinator::Concat { parts, separator } => {
            let mut strs = Vec::with_capacity(parts.len());
            for (i, part) in parts.iter().enumerate() {
                let v = eval(ctx, scope, part).map_err(|e| format!("concat.parts[{i}]: {e}"))?;
                strs.push(v.as_text());
            }
            Ok(YValue::Str(strs.join(separator)))
        }
    }
}

fn expect_date(r: Resolved, ctx_msg: &str) -> EvalResult<DateValue> {
    match r {
        Resolved::Value(CellValue::Date(d)) => Ok(d),
        Resolved::Value(CellValue::Period { from, .. }) => Ok(from),
        _ => Err(format!("{ctx_msg} は Date 値である必要があります")),
    }
}

fn cast(text: String, as_type: AsType) -> EvalResult<YValue> {
    match as_type {
        AsType::Text => Ok(YValue::Str(text)),
        AsType::Int => text
            .trim()
            .parse::<i64>()
            .map(YValue::Int)
            .map_err(|_| format!("as: int が指定されましたが '{text}' は整数として解釈できません")),
    }
}

fn eval_rows(ctx: &EvalContext, scope: &Scope, source: &RowSource, item: &Combinator) -> EvalResult<YValue> {
    let mut out = Vec::new();
    match source {
        RowSource::Table(sel) => {
            let Resolved::Node(table_id) = resolve_selector(ctx, scope, sel)? else {
                return Err("rows.table は table ノードを指す必要があります".to_string());
            };
            let NodePayload::Table(t) = &ctx.node(table_id).payload else {
                return Err("rows.table の対象が table ノードではありません".to_string());
            };
            for row_path in row_order(t) {
                let row_scope = Scope { row_path, current_table: Some(table_id), current_node: None };
                out.push(eval(ctx, &row_scope, item)?);
            }
        }
        RowSource::Contains { of, node_type, extend_path, include_only_class, exclude_class } => {
            let Resolved::Node(parent) = resolve_selector(ctx, scope, of)? else {
                return Err("rows.contains は親ノードを指す必要があります".to_string());
            };
            for child in ctx.graph.children_of(parent) {
                if let Some(t) = node_type
                    && node_type_name(&ctx.node(child).payload) != t
                {
                    continue;
                }
                ctx.mark(child);
                // D58(sml-spec.md §1.17): `join` と同一の実効 class(D46: 自身+祖先の
                // 和集合)判定でフィルタし、落ちた子はその行ごとスキップする
                // (サブツリーの行が生成されない)。
                if let Some(want) = include_only_class.as_deref() {
                    let tags: HashSet<&str> = std::iter::once(want).collect();
                    if !strata_core::has_effective_class(ctx.graph, &ctx.parents, child, &tags) {
                        continue;
                    }
                }
                if let Some(deny) = exclude_class.as_deref() {
                    let tags: HashSet<&str> = std::iter::once(deny).collect();
                    if strata_core::has_effective_class(ctx.graph, &ctx.parents, child, &tags) {
                        continue;
                    }
                }
                let mut child_scope = scope.clone();
                child_scope.current_node = Some(child);
                if let Some(ExtendPath::AliasSuffix { prefix }) = extend_path {
                    let alias = ctx.node(child).alias.as_deref().ok_or_else(|| {
                        format!("extend-path: alias-suffix ですが子ノードに alias がありません(id={child:?})")
                    })?;
                    let suffix = alias.strip_prefix(prefix.as_str()).ok_or_else(|| {
                        format!("extend-path: alias '{alias}' が接頭辞 '{prefix}' で始まっていません")
                    })?;
                    child_scope.row_path.push(suffix.to_string());
                }
                out.push(eval(ctx, &child_scope, item)?);
            }
        }
    }
    Ok(YValue::Seq(out))
}

#[allow(clippy::too_many_arguments)]
fn eval_join(
    ctx: &EvalContext,
    scope: &Scope,
    of: &Selector,
    separator: &str,
    nested_prefix: Option<&str>,
    include_only_class: Option<&str>,
    exclude_class: Option<&str>,
    keys: Option<&[String]>,
) -> EvalResult<YValue> {
    let Resolved::Node(node_id) = resolve_selector(ctx, scope, of)? else {
        return Err("join の of はノードを指す必要があります".to_string());
    };
    let node = ctx.node(node_id);

    if let Some(keys) = keys {
        let NodePayload::Record(rec) = &node.payload else {
            return Err("join.keys が指定されていますが対象が record ノードではありません".to_string());
        };
        let mut lines = Vec::new();
        for key in keys {
            let entry = rec.entries.iter().find(|e| e.key == *key);
            let text = entry.map(|e| cellvalue_text(&e.value)).unwrap_or_default();
            if !text.is_empty() {
                lines.push(format!("{key}: {text}"));
            }
        }
        return Ok(YValue::Str(lines.join(separator)));
    }

    if include_only_class.is_some() && exclude_class.is_some() {
        return Err("join: include-only-class と exclude-class は併用できません".to_string());
    }

    let mut lines = Vec::new();
    for child in ctx.graph.children_of(node_id) {
        ctx.mark(child);
        // D46: 実効 class(自身+祖先の和集合)で判定する — render --hide /
        // context --class と同じ定義(strata_core::has_effective_class)に統一。
        // コンテナ(見出し・リスト・引用)に class を1回書けば、その子ノードの
        // join にも継承される。
        if let Some(want) = include_only_class {
            let tags: HashSet<&str> = std::iter::once(want).collect();
            if !strata_core::has_effective_class(ctx.graph, &ctx.parents, child, &tags) {
                continue;
            }
        }
        if let Some(deny) = exclude_class {
            let tags: HashSet<&str> = std::iter::once(deny).collect();
            if strata_core::has_effective_class(ctx.graph, &ctx.parents, child, &tags) {
                continue;
            }
        }
        push_join_lines(ctx, child, nested_prefix, &mut lines);
    }
    Ok(YValue::Str(lines.join(separator)))
}

/// `join` 木モードの1ノード分をテキスト行へ再帰的に展開する。
///
/// D46(sml-spec.md §1.11)対応: 対象が `Section`(見出し。note コンテナ等)の場合、
/// 見出し自身のテキストを1行足したうえで、配下の子へ**同じロジックで再帰**する
/// (List は既存のアウトライン化、Section はさらに再帰、それ以外は1行)。
///
/// 背景: D46 前は note が段落ごとに `[class=note]` を繰り返す形だったため、
/// `join`(`include-only-class: note` 等)は「対象ノードの直接の子を1行ずつ」で
/// 全内容を拾えていた。D46 のコンテナ形式リライト(WP-Z4)後は、複数ブロックの
/// note が「見出し+リスト+段落」という**1つの Section のサブツリー**にまとまる。
/// この関数が無いと `text_of_node(Section)` が見出しの平文だけを返し、配下の
/// 内容が join の出力から消える(work_history.sml の HUMABUILD note 群で実際に
/// 発生を確認、最終報告参照)。再帰時は class フィルタを掛け直さない —
/// コンテナが一度マッチしたら、その配下は「同じ note の一部」として丸ごと拾う
/// (D46「コンテナに1回書けばよい」の join 版)。
fn push_join_lines(ctx: &EvalContext, id: NodeId, nested_prefix: Option<&str>, lines: &mut Vec<String>) {
    let node = ctx.node(id);
    match &node.payload {
        NodePayload::List(_) => {
            for item in ctx.graph.children_of(id) {
                ctx.mark(item);
                lines.push(text_of_node(ctx.node(item)));
                for sub in ctx.graph.children_of(item) {
                    ctx.mark(sub);
                    if let NodePayload::List(_) = &ctx.node(sub).payload {
                        for subitem in ctx.graph.children_of(sub) {
                            ctx.mark(subitem);
                            let prefix = nested_prefix.unwrap_or("");
                            lines.push(format!("{prefix}{}", text_of_node(ctx.node(subitem))));
                        }
                    }
                }
            }
        }
        NodePayload::Section(_) => {
            lines.push(text_of_node(node));
            for child in ctx.graph.children_of(id) {
                ctx.mark(child);
                push_join_lines(ctx, child, nested_prefix, lines);
            }
        }
        _ => lines.push(text_of_node(node)),
    }
}

/// `Combinator::Fields`(または `Rows` の中の `Fields`)からフィールド名の列を取り出す。
/// マニフェスト突合(--check の「未充足スロット」判定)専用。グラフの評価はしない。
pub fn declared_field_names(comb: &Combinator) -> Option<Vec<String>> {
    match comb {
        Combinator::Fields(entries) => Some(entries.iter().map(|(k, _)| k.clone()).collect()),
        Combinator::Rows { item, .. } => declared_field_names(item),
        _ => None,
    }
}

/// テストヘルパ: BTreeMap への変換(未使用ノード集計などで使いやすい形)。
#[allow(dead_code)]
pub fn touched_sorted(ctx: &EvalContext) -> BTreeMap<NodeId, ()> {
    ctx.touched.borrow().iter().map(|id| (*id, ())).collect()
}
