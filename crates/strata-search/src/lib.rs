//! strata-search — ブロック単位の全文検索+構造述語(D56、sml-spec.md §1.16)。
//!
//! 入力は `strata_build::build`/`build_workspace` の出力(canonical グラフ)。v0 は
//! **インメモリ・部分文字列一致**の素直な実装(転置索引も永続化もしない)。ただし
//! 「index 実装を差し替え可能」(D56)という要件があるため、クエリを受けて
//! ヒットを返す部分は [`SearchIndex`] トレイトの背後に切り出してある —
//! 将来 index を持ち込む場合は、このトレイトを実装する別の型(例:
//! `PersistentIndex`)を用意すればよく、`Query`/`Hit`/`Snippet` 等の公開型・
//! CLI 側は変更不要になる想定。[`InMemoryIndex`] が v0 で唯一の実装。
//!
//! ## クエリ構文(D56)
//! 空白区切りのトークン列、AND 結合。各トークンは以下のいずれか:
//! - 素のテキスト(部分文字列一致、大小無視。CJK は普通の部分文字列一致で足りる)
//! - `class:<tag>` — 実効 class(D46、自身+祖先の和集合)が `tag` を含むこと(完全一致)
//! - `term:<用語>` — そのノードが Term ノードでその名前を含む、またはそのブロックが
//!   インライン `term:` 参照でその用語を使っている(部分一致)
//! - `alias:<接頭辞>` — ノード alias がその接頭辞で始まる
//!
//! 述語の構文は最小(値のクォート・エスケープ・OR/NOT は無い)。拡張余地は
//! crate ルートのモジュールコメント末尾および最終報告を参照。
//!
//! ## クイックスイッチャー([`SearchIndex::switcher`])
//! 全文検索とは別の軽量フィルタ。文書(Document ノード)・見出し(Section ノード)・
//! alias を持つ任意のノードだけを対象に、素の部分文字列でマッチする(述語構文は無い)。
//! エディタの Ctrl+O 相当が呼ぶ想定(D54: エディタは Rust を直接呼ぶ)。

mod query;
mod text;

#[cfg(test)]
mod tests;

use std::collections::HashMap;

pub use query::Query;

use serde::{Deserialize, Serialize};
use strata_build::{BuildOutput, WorkspaceBuildOutput};
use strata_core::{Graph, Node, NodeId, NodePayload, Rel};

/// ヒット・スイッチャー結果に添える「所属文書」。単一ファイル build では呼び出し側が
/// 明示的に1つだけ渡す(ファイル名を `path` に)。ワークスペースでは
/// `strata_build::doc_ownership` + `DocRoot::alias` から組み立てる([`InMemoryIndex::from_workspace`])。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocRef {
    /// ファイルパス(表示用。ワークスペースでは `strata.toml` からの相対パス)。
    pub path: String,
    /// フロントマターの文書 alias(D41)。あれば表示にはこちらを優先する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

impl DocRef {
    /// 人間向けの短い表示形(alias があればそちら、無ければ path)。
    pub fn display(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.path)
    }
}

/// マッチ箇所前後のスニペット。3分割(前後+マッチ本体)で持つことで、CLI 人間可読出力・
/// `--json`(エディタ・AI 向け)のどちらでも「マーキング」を表現できる(座標計算を
/// 消費者側に持ち込まない)。述語のみのクエリ(素のテキスト条件が無い)では
/// `matched` が空文字列になり、`after` にブロック先頭からの単純な切り詰めが入る。
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Snippet {
    pub before: String,
    pub matched: String,
    pub after: String,
}

/// 全文検索のヒット(D56: ヒット単位=ブロック=ノード)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    pub node_id: NodeId,
    /// snake_case のノード種別名(strata-core の `NodePayload` serde tag と同じ字句、
    /// 例: `"section"`/`"para"`/`"table"`)。
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    /// 人間可読ラベル(G1.7 のフォールバック順: 明示テキスト → alias → ULID 短縮)。
    pub label: String,
    pub snippet: Snippet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<DocRef>,
    /// 素朴なランキングスコア(見出し一致>本文一致。裁量、絶対値に意味は無い —
    /// 相対順序の決定にのみ使う)。
    pub score: i64,
}

/// クイックスイッチャーのヒット(D56/D57: 文書・見出し・alias に限定)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SwitcherHit {
    pub node_id: NodeId,
    pub node_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<DocRef>,
    pub score: i64,
}

/// index 実装の差し替え境界(D56)。v0 は [`InMemoryIndex`] のみ実装。将来の永続化・
/// 転置索引の実装はこのトレイトを実装する別の型として追加すればよい設計。
pub trait SearchIndex {
    /// 全文検索(構造述語混在可、§ crate ルートのクエリ構文)。`limit` 件までスコア降順
    /// (同点は `NodeId` 昇順、決定的)。`query` が無条件(トークン0個)なら空を返す。
    fn search(&self, query: &Query, limit: usize) -> Vec<Hit>;

    /// クイックスイッチャー(文書・見出し・alias のみを対象にした軽量フィルタ、述語構文は無い)。
    fn switcher(&self, query: &str, limit: usize) -> Vec<SwitcherHit>;
}

// --- インメモリ実装(v0) -----------------------------------------------------------

struct IndexedBlock {
    id: NodeId,
    node_type: &'static str,
    alias: Option<String>,
    label: String,
    /// 見出し的テキスト(ランキングの一次シグナル)。小文字化済みを併せて持つ。
    heading_lower: Option<String>,
    body: String,
    body_lower: String,
    classes_effective: std::collections::HashSet<String>,
    /// 用語名(自身が Term ならその name、それ以外はインライン `term:` 参照先の name 群)。
    /// 小文字化済み。
    term_names_lower: Vec<String>,
    doc: Option<DocRef>,
}

impl IndexedBlock {
    fn matches(&self, q: &Query) -> bool {
        for cls in &q.class {
            if !self.classes_effective.contains(cls) {
                return false;
            }
        }
        for prefix in &q.alias_prefix {
            match &self.alias {
                Some(a) if a.starts_with(prefix.as_str()) => {}
                _ => return false,
            }
        }
        for name in &q.term {
            let nl = name.to_lowercase();
            if !self.term_names_lower.iter().any(|n| n.contains(&nl)) {
                return false;
            }
        }
        for t in &q.text {
            let tl = t.to_lowercase();
            if !self.body_lower.contains(&tl) {
                return false;
            }
        }
        true
    }
}

/// v0 の唯一の [`SearchIndex`] 実装。「使い捨てのインメモリ表」(D42 と同じ思想 —
/// build のたびに作り直してよい派生物であり、正本性を持たない)。
pub struct InMemoryIndex {
    blocks: Vec<IndexedBlock>,
}

impl InMemoryIndex {
    /// コア構築関数: canonical グラフ + ノード→所属文書の対応表から索引を組む。
    /// `doc_of` に無いノード(所属文書が特定できない — 例: `contains` で辿れない
    /// 孤立ノード)は `Hit::doc == None` になる。
    pub fn build(graph: &Graph, doc_of: &HashMap<NodeId, DocRef>) -> Self {
        let parents = strata_core::parent_index(graph);

        // インラインの `term:` 参照(Rel::TermRef)を「ブロック → 使用した用語名」の
        // 表に畳む(D56: term: 述語がブロックの term-ref 使用も拾えるようにするため。
        // 用語ノード自身の name はここに含めない — それは下のループで別途足す)。
        let term_name_of: HashMap<NodeId, String> = graph
            .nodes
            .iter()
            .filter_map(|(id, n)| match &n.payload {
                NodePayload::Term(t) => Some((*id, t.name.clone())),
                _ => None,
            })
            .collect();
        let mut term_refs_of_block: HashMap<NodeId, Vec<String>> = HashMap::new();
        for e in &graph.edges {
            if e.rel == Rel::TermRef
                && let Some(name) = term_name_of.get(&e.to)
            {
                term_refs_of_block.entry(e.from).or_default().push(name.clone());
            }
        }

        let mut blocks = Vec::with_capacity(graph.nodes.len());
        for (id, node) in &graph.nodes {
            let texts = text::node_texts(&node.payload);
            let label = derive_label(node, &texts);
            let classes_effective = strata_core::effective_classes(graph, &parents, *id);

            let mut term_names_lower: Vec<String> = term_refs_of_block
                .get(id)
                .map(|v| v.iter().map(|s| s.to_lowercase()).collect())
                .unwrap_or_default();
            if let NodePayload::Term(t) = &node.payload {
                term_names_lower.push(t.name.to_lowercase());
            }

            blocks.push(IndexedBlock {
                id: *id,
                node_type: text::node_type_name(&node.payload),
                alias: node.alias.clone(),
                label,
                heading_lower: texts.heading.as_ref().map(|h| h.to_lowercase()),
                body_lower: texts.body.to_lowercase(),
                body: texts.body,
                classes_effective,
                term_names_lower,
                doc: doc_of.get(id).cloned(),
            });
        }

        InMemoryIndex { blocks }
    }

    /// 単一ファイル build から索引を組む。`doc` は呼び出し側が渡す所属文書
    /// (通常は入力ファイルのパス。単一ファイル build には `WorkspaceBuildOutput` の
    /// ような複数 `DocRoot` が無いため、この情報を build 結果から自動導出できない)。
    /// フロントマターが無い(`root: None`)場合、全ノードの `doc` は `None` になる。
    pub fn from_build(build: &BuildOutput, doc: DocRef) -> Self {
        let mut doc_of = HashMap::new();
        if let Some(root) = build.root {
            for id in subtree_ids(&build.graph, root) {
                doc_of.insert(id, doc.clone());
            }
        }
        Self::build(&build.graph, &doc_of)
    }

    /// ワークスペース build から索引を組む。所属文書は `strata_build::doc_ownership`
    /// (`contains` サブツリーの逆引き)+ 各メンバーのフロントマター alias から組み立てる。
    pub fn from_workspace(ws: &WorkspaceBuildOutput) -> Self {
        let owner = strata_build::doc_ownership(&ws.graph, &ws.roots);
        let alias_by_path: HashMap<&str, &str> =
            ws.roots.iter().filter_map(|r| r.alias.as_deref().map(|a| (r.path.as_str(), a))).collect();
        let doc_of: HashMap<NodeId, DocRef> = owner
            .into_iter()
            .map(|(id, path)| {
                let alias = alias_by_path.get(path.as_str()).map(|s| s.to_string());
                (id, DocRef { path, alias })
            })
            .collect();
        Self::build(&ws.graph, &doc_of)
    }
}

impl SearchIndex for InMemoryIndex {
    fn search(&self, query: &Query, limit: usize) -> Vec<Hit> {
        if query.is_empty() {
            return Vec::new();
        }
        let mut scored: Vec<(i64, &IndexedBlock)> =
            self.blocks.iter().filter(|b| b.matches(query)).map(|b| (score_block(b, query), b)).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
        scored.truncate(limit);
        scored
            .into_iter()
            .map(|(score, b)| Hit {
                node_id: b.id,
                node_type: b.node_type.to_string(),
                alias: b.alias.clone(),
                label: b.label.clone(),
                snippet: build_snippet(&b.body, query),
                doc: b.doc.clone(),
                score,
            })
            .collect()
    }

    fn switcher(&self, query: &str, limit: usize) -> Vec<SwitcherHit> {
        let q = query.trim();
        if q.is_empty() {
            return Vec::new();
        }
        let ql = q.to_lowercase();
        let mut scored: Vec<(i64, &IndexedBlock)> = self
            .blocks
            .iter()
            .filter(|b| b.node_type == "document" || b.node_type == "section" || b.alias.is_some())
            .filter_map(|b| {
                let label_l = b.label.to_lowercase();
                let alias_l = b.alias.as_ref().map(|a| a.to_lowercase());
                let score = if alias_l.as_deref() == Some(ql.as_str()) {
                    400
                } else if label_l.starts_with(&ql) {
                    300
                } else if alias_l.as_deref().is_some_and(|a| a.contains(&ql)) {
                    200
                } else if label_l.contains(&ql) {
                    100
                } else {
                    return None;
                };
                Some((score, b))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));
        scored.truncate(limit);
        scored
            .into_iter()
            .map(|(score, b)| SwitcherHit {
                node_id: b.id,
                node_type: b.node_type.to_string(),
                alias: b.alias.clone(),
                label: b.label.clone(),
                doc: b.doc.clone(),
                score,
            })
            .collect()
    }
}

// --- ランキング・スニペット・ラベル --------------------------------------------------

/// 素朴なランキング(裁量、D56「見出し一致>本文一致 程度」): 見出し的テキストに
/// ヒットしたテキスト条件の数を主シグナルに、alias ヒットに定数ボーナス、本文の長さ
/// (短いほど的を絞っているとみなす、上限 500 文字でキャップ)を弱いペナルティにする。
/// 述語のみのクエリ(text が空)では常に0基点+長さペナルティだけになり、短いブロック
/// ほど上位に来る。
fn score_block(b: &IndexedBlock, q: &Query) -> i64 {
    let mut heading_hits = 0i64;
    if let Some(h) = &b.heading_lower {
        for t in &q.text {
            if h.contains(&t.to_lowercase()) {
                heading_hits += 1;
            }
        }
    }
    let alias_hit = b.alias.as_ref().is_some_and(|a| {
        let al = a.to_lowercase();
        q.text.iter().any(|t| al.contains(&t.to_lowercase()))
    });

    let mut score = heading_hits * 1000;
    if alias_hit {
        score += 200;
    }
    score -= (b.body.chars().count() as i64).min(500);
    score
}

/// マッチ箇所前後 `CTX` 文字を切り出したスニペットを組む(文字境界基準、CJK 安全)。
const SNIPPET_CTX: usize = 24;
const SNIPPET_FALLBACK_LEN: usize = 80;

fn build_snippet(body: &str, q: &Query) -> Snippet {
    for t in &q.text {
        if let Some((start, len)) = find_ci_char_range(body, t) {
            return window_snippet(body, start, len);
        }
    }
    Snippet { before: String::new(), matched: String::new(), after: truncate_chars(body, SNIPPET_FALLBACK_LEN) }
}

/// 大小無視の部分文字列検索(文字インデックス基準)。各文字を個別に小文字化して
/// 1文字 1文字の対応を保つ(トルコ語 İ 等、`to_lowercase()` が複数文字を返すケースは
/// 先頭の1文字だけを採用する近似 — v0 の CJK/ASCII/日本語混在という用途では十分、
/// 最終報告に明記)。
fn find_ci_char_range(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    let need_chars: Vec<char> = needle.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    let need_len = need_chars.len();
    if need_len == 0 {
        return None;
    }
    let hay_lower: Vec<char> = haystack.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect();
    if hay_lower.len() < need_len {
        return None;
    }
    for start in 0..=(hay_lower.len() - need_len) {
        if hay_lower[start..start + need_len] == need_chars[..] {
            return Some((start, need_len));
        }
    }
    None
}

fn window_snippet(text: &str, match_char_idx: usize, match_len_chars: usize) -> Snippet {
    let chars: Vec<char> = text.chars().collect();
    let match_end = (match_char_idx + match_len_chars).min(chars.len());
    let start = match_char_idx.saturating_sub(SNIPPET_CTX);
    let end = (match_end + SNIPPET_CTX).min(chars.len());

    let before: String = chars[start..match_char_idx].iter().collect();
    let matched: String = chars[match_char_idx..match_end].iter().collect();
    let after: String = chars[match_end..end].iter().collect();

    Snippet {
        before: if start > 0 { format!("…{before}") } else { before },
        matched,
        after: if end < chars.len() { format!("{after}…") } else { after },
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    let count = s.chars().count();
    if count <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}

const MAX_LABEL_CHARS: usize = 60;

/// G1.7 の表示ラベルポリシー(ui/src/lib/label.ts と同型): 明示テキスト(見出し的
/// テキスト、無ければブロック自身の本文)→ alias → ULID 短縮、の順でフォールバックする。
fn derive_label(node: &Node, texts: &text::NodeTexts) -> String {
    if let Some(h) = &texts.heading {
        let t = h.trim();
        if !t.is_empty() {
            return truncate_label(t);
        }
    }
    let body_trim = texts.body.trim();
    if !body_trim.is_empty() {
        return truncate_label(body_trim);
    }
    if let Some(alias) = &node.alias {
        return alias.clone();
    }
    short_id(node.id)
}

fn truncate_label(s: &str) -> String {
    let normalized = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let count = normalized.chars().count();
    if count <= MAX_LABEL_CHARS {
        return normalized;
    }
    let head: String = normalized.chars().take(MAX_LABEL_CHARS).collect();
    format!("{head}…")
}

/// ULID 短縮(末尾6文字。ui/src/lib/label.ts の `shortId` と同じ裁量 — タイムスタンプ
/// 相当の先頭よりランダム性の高い末尾の方が同時期生成ノードを見分けやすい)。
fn short_id(id: NodeId) -> String {
    let s = id.0.to_string();
    if s.len() > 6 { s[s.len() - 6..].to_string() } else { s }
}

/// `contains` を辿って `root` のサブツリー(自身含む)の `NodeId` を集める
/// (strata-context::subtree_ids と同型、依存を避けるための小さな重複)。
fn subtree_ids(graph: &Graph, root: NodeId) -> Vec<NodeId> {
    fn walk(graph: &Graph, id: NodeId, out: &mut Vec<NodeId>) {
        out.push(id);
        for child in graph.children_of(id) {
            walk(graph, child, out);
        }
    }
    let mut out = Vec::new();
    walk(graph, root, &mut out);
    out
}
