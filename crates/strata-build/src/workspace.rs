//! ワークスペース build(WP-W2、D43、sml-spec.md §1.10)。
//!
//! 複数の SML ファイル(`strata.toml` の `members`)を一括パースし、横断参照
//! (`<文書alias>/<ブロックalias>`、D42)を解決した**単一の統合グラフ**を作る。
//! 「store/index 分離」の index 側: `resolve::CrossDocIndex` は build のたびに
//! 作り直す使い捨てのインメモリ表であり、正本性を持たない(index の永続化・
//! インクリメンタル更新は §10 保留)。
//!
//! `strata.toml` 自体のパース・グロブ展開は呼び出し側(strata-cli)の責務 —
//! この関数はファイルパス+ソース文字列の列(`Member`)を受け取るだけで、ファイル
//! システムにも TOML にも触れない(strata-build の既存方針「パース済み文字列を
//! 受け取る」を踏襲)。

use std::collections::{BTreeMap, HashMap};

use serde::{Deserialize, Serialize};
use strata_core::{Graph, NodeId};
use strata_sml::Diag;

use crate::convert::{self, SharedState};
use crate::error::BuildError;
use crate::resolve::{self, CrossDocIndex, Registry};

/// ワークスペースの1メンバー。`path` は診断表示用のファイル名
/// (`strata.toml` からの相対パス。裁量: 「ファイル:行:列:」の「ファイル:」に使う)。
#[derive(Debug, Clone)]
pub struct Member {
    pub path: String,
    pub src: String,
}

/// 1メンバー文書のルート情報(WP-W2.4: 複数文書ぶんの roots の表現、裁量)。
/// `alias` はフロントマターに alias が無ければ `None`(D41: エラーではない —
/// その文書へは ULID 参照のみ可)。`root` はフロントマター自体が無ければ `None`
/// (単一ファイル build の `BuildOutput::root` と同じ意味、D12)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DocRoot {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<NodeId>,
}

/// ファイル名付きの警告(WP-W2.3: 診断にファイル名を含める)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileDiag {
    pub path: String,
    pub diag: Diag,
}

/// `build_workspace` の成功時の戻り値。`BuildOutput` とは形が違う(複数 root を
/// 持つため) — CLI(WP-W2.2)はこの構造体ごと `serde_json` でラウンドトリップする。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceBuildOutput {
    pub graph: Graph,
    pub roots: Vec<DocRoot>,
    /// 文書 alias → (ブロック alias → NodeId)。build 時に組んだ `CrossDocIndex`
    /// (使い捨てのインメモリ index、D42)をそのまま出力へ写したもの。
    /// `strata-view` の doc スコープ解決(WP-W3、`{ alias: X, doc: Y }`)が
    /// 再構築なしで直接使う。文書 alias を持たないメンバーはここに現れない(D41:
    /// エラーではなく、その文書へは ULID 参照のみ可)。`BTreeMap` はグラフ JSON の
    /// 決定的な出力順のため(`HashMap` だと反復順が不定)。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub doc_aliases: BTreeMap<String, BTreeMap<String, NodeId>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<FileDiag>,
}

/// ワークスペース build のエラー(WP-W2.3、全か無か・全ファイル集約)。
/// ファイル横断の構造エラー(文書 alias 重複・ファイル間 ULID 衝突)と、
/// 個々のファイルの build エラー(パース診断・解決エラー・doc 修飾未解決等、
/// `BuildError` をそのまま包む)の2系統。
#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceError {
    /// 2文書が同じ frontmatter alias を宣言している(WP-W2.3)。
    DuplicateDocAlias { alias: String, paths: Vec<String> },
    /// 2つ以上のファイルが同じ ULID を明示的に宣言している(コピペ事故、WP-W2.3)。
    UlidCollision { id: NodeId, paths: Vec<String> },
    /// 特定ファイルの build エラー。`path` が空文字列なら統合グラフ全体の
    /// `invariants::validate` 違反(個別ファイルに帰属しない)。
    Member { path: String, error: BuildError },
}

/// 公開 API: 複数の SML ソースから単一の統合 canonical グラフを構築する
/// (WP-W2.2、sml-spec.md §1.10 D43)。
///
/// 全か無か(D13 と同じ方針をワークスペース全体に拡張): 全ファイルを最後まで処理し、
/// 収集した全 `WorkspaceError` を返す(最初の1件で止めない)。
pub fn build_workspace(members: &[Member]) -> Result<WorkspaceBuildOutput, Vec<WorkspaceError>> {
    let mut errors: Vec<WorkspaceError> = Vec::new();
    let mut warnings: Vec<FileDiag> = Vec::new();

    struct FileCtx {
        idx: usize,
        doc: strata_sml::SmlDocument,
        registry: Registry,
    }

    let mut files: Vec<FileCtx> = Vec::with_capacity(members.len());
    for (idx, m) in members.iter().enumerate() {
        let parsed = strata_sml::parse(&m.src);
        let (parse_errors, parse_warnings): (Vec<_>, Vec<_>) = parsed.diags.into_iter().partition(Diag::is_error);
        for e in parse_errors {
            errors.push(WorkspaceError::Member { path: m.path.clone(), error: BuildError::Parse(e) });
        }
        for w in parse_warnings {
            warnings.push(FileDiag { path: m.path.clone(), diag: w });
        }
        let mut reg_errors = Vec::new();
        let registry = resolve::build_registry(&parsed.doc, &mut reg_errors);
        for e in reg_errors {
            errors.push(WorkspaceError::Member { path: m.path.clone(), error: e });
        }
        files.push(FileCtx { idx, doc: parsed.doc, registry });
    }

    // 文書 alias index + 重複検出(WP-W2.3)。
    let mut doc_alias_defs: HashMap<String, Vec<String>> = HashMap::new();
    for f in &files {
        if let Some(doc_id) = f.registry.document_id
            && let Some(alias) = f.registry.id_alias.get(&doc_id)
        {
            doc_alias_defs.entry(alias.clone()).or_default().push(members[f.idx].path.clone());
        }
    }
    let mut dup_alias_errors: Vec<(String, Vec<String>)> =
        doc_alias_defs.into_iter().filter(|(_, paths)| paths.len() > 1).collect();
    dup_alias_errors.sort_by(|a, b| a.0.cmp(&b.0));
    for (alias, paths) in dup_alias_errors {
        errors.push(WorkspaceError::DuplicateDocAlias { alias, paths });
    }

    // ファイル間 ULID 衝突検出(WP-W2.3、コピペ事故)。`node_kind` は Pass 1 が
    // 明示的な ID 宣言(フロントマター id / `{#ULID}` / `[id=ULID]`)のときだけ
    // 登録する表なので、build が内部生成する ID(水平線・子リスト・Term)は
    // 対象に含まれない(Term は D9 の安定 ID による自然合流が正しい挙動であり、
    // 衝突ではない)。
    let mut id_owner: HashMap<NodeId, String> = HashMap::new();
    let mut collision_paths: HashMap<NodeId, Vec<String>> = HashMap::new();
    for f in &files {
        let path = &members[f.idx].path;
        for id in f.registry.node_kind.keys() {
            match id_owner.get(id) {
                Some(prev) => {
                    let entry = collision_paths.entry(*id).or_insert_with(|| vec![prev.clone()]);
                    if !entry.contains(path) {
                        entry.push(path.clone());
                    }
                }
                None => {
                    id_owner.insert(*id, path.clone());
                }
            }
        }
    }
    let mut collisions: Vec<(NodeId, Vec<String>)> = collision_paths.into_iter().collect();
    collisions.sort_by_key(|(id, _)| *id);
    for (id, paths) in collisions {
        errors.push(WorkspaceError::UlidCollision { id, paths });
    }

    // cross-doc index(D42): 文書 alias → その文書のブロック alias 表。文書 alias が
    // 重複していれば(既に上で Err 化済み)最初に登録されたファイルの表を採用する
    // (どのみち全体は Err になるため、決定性さえあればよい)。
    let mut cross_doc: CrossDocIndex = HashMap::new();
    for f in &files {
        if let Some(doc_id) = f.registry.document_id
            && let Some(alias) = f.registry.id_alias.get(&doc_id)
        {
            cross_doc.entry(alias.clone()).or_insert_with(|| f.registry.alias_table.clone());
        }
    }

    let mut shared = SharedState::new();
    let mut roots: Vec<DocRoot> = Vec::with_capacity(files.len());
    for f in files {
        let path = members[f.idx].path.clone();
        let src = &members[f.idx].src;
        let doc_alias = f.registry.document_id.and_then(|id| f.registry.id_alias.get(&id).cloned());
        let (root, pass2_errors) = convert::run(src, &f.doc, f.registry, &mut shared, Some(&cross_doc));
        for e in pass2_errors {
            errors.push(WorkspaceError::Member { path: path.clone(), error: e });
        }
        roots.push(DocRoot { path, alias: doc_alias, root });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let violations = strata_core::invariants::validate(&shared.graph);
    if !violations.is_empty() {
        return Err(violations
            .into_iter()
            .map(|v| WorkspaceError::Member { path: String::new(), error: BuildError::Invariant(v) })
            .collect());
    }

    roots.sort_by(|a, b| a.path.cmp(&b.path));
    let doc_aliases: BTreeMap<String, BTreeMap<String, NodeId>> = cross_doc
        .into_iter()
        .map(|(doc, table)| (doc, table.into_iter().collect::<BTreeMap<_, _>>()))
        .collect();
    Ok(WorkspaceBuildOutput { graph: shared.graph, roots, doc_aliases, warnings })
}

/// 各文書の `contains` 部分木を根(`DocRoot::root`)から辿り、ノード → 所属文書
/// (`DocRoot::path`)の逆引き表を作る(D44: `render --workspace`/`context --workspace`
/// のクロスドキュメント判定、および WP-W4 の `strata-view::check` 内で個別実装
/// されていた同じロジックをここへ統一する — 「1箇所に定義」の対象は D46 の class に
/// 限らず、doc 所属判定も複数消費者が必要とする共有ロジックのため)。
/// `contains` で辿れない孤立ノード(Term 等)には所有者が付かない。
pub fn doc_ownership(graph: &Graph, roots: &[DocRoot]) -> HashMap<NodeId, String> {
    let mut owner: HashMap<NodeId, String> = HashMap::new();
    for r in roots {
        let Some(root_id) = r.root else { continue };
        owner.entry(root_id).or_insert_with(|| r.path.clone());
        let mut stack = vec![root_id];
        while let Some(id) = stack.pop() {
            for child in graph.children_of(id) {
                if owner.contains_key(&child) {
                    continue;
                }
                owner.insert(child, r.path.clone());
                stack.push(child);
            }
        }
    }
    owner
}
