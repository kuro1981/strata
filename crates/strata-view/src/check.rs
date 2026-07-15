//! `strata view --check`(D33)の dry-run 診断。
//!
//! 「未充足スロット」: マニフェストが要求するファイル/フィールドが、ビュー定義側に
//! 宣言されていない。「未使用ノード」: グラフのノードのうち、どのファイル・
//! どの profile の評価によっても一度も参照(セレクタで解決)されなかったもの
//! (粗い定義。裁量、docs/view-v1-handoff.md 参照)。
use crate::def::ViewDef;
use crate::eval::{self, EvalContext};
use crate::manifest::Manifest;
use std::collections::{BTreeSet, HashMap, HashSet};
use strata_build::{BuildOutput, WorkspaceBuildOutput};
use strata_core::{Graph, NodeId, NodePayload};

#[derive(Debug, Default)]
pub struct CheckReport {
    /// "-:-: MissingSlot: <message>" 形式の本文だけを保持する(表示は呼び出し側)。
    pub missing_slots: Vec<String>,
    pub unused_nodes: Vec<String>,
    pub eval_errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl CheckReport {
    pub fn is_clean(&self) -> bool {
        self.missing_slots.is_empty() && self.unused_nodes.is_empty() && self.eval_errors.is_empty()
    }
}

pub fn check(build: &BuildOutput, view: &ViewDef, manifest: &Manifest) -> CheckReport {
    let ctx = EvalContext::new(&build.graph);
    let mut report = check_slots_and_eval(&ctx, view, manifest);
    // 単一文書モード: 従来どおりグラフ全体(=その1文書)を対象に未使用ノードを
    // 走査する(doc フィルタ無し)。
    scan_unused_nodes(&ctx, &mut report, None);
    report
}

/// `check` のワークスペース版(WP-W3)。doc スコープ(`{ alias, doc }`)を解決できる
/// `EvalContext::new_workspace` を使う点と、「未使用ノード」走査の対象を
/// **このビュー定義が実際に触れた文書だけ**に絞る点が `check` と異なる(裁量。
/// 理由: 統合グラフをそのまま `check` と同じ「グラフ全体走査」にすると、1つの
/// ビュー定義が意図的に対象としていない他文書の alias 付きノードが常に
/// 「未使用」として鳴ってしまい、「ゼロ診断」の実用性が失われる — 例えば
/// resume-jis.view.yaml は resume.sml だけを対象にしているが、統合グラフには
/// work_history.sml のノードも載っているため、フィルタ無しだと必ず数件の
/// UnusedNode が出る。「このビューが1つでも触れた文書」を単位に絞ることで、
/// 単一文書モードと同じ「そのビューの対象文書内で使い忘れが無いか」という
/// 診断の意図を保つ)。
pub fn check_workspace(ws: &WorkspaceBuildOutput, view: &ViewDef, manifest: &Manifest) -> CheckReport {
    let ctx = EvalContext::new_workspace(&ws.graph, &ws.doc_aliases);
    let mut report = check_slots_and_eval(&ctx, view, manifest);
    let doc_of = compute_doc_ownership(&ws.graph, &ws.roots);
    scan_unused_nodes(&ctx, &mut report, Some(&doc_of));
    report
}

/// 未充足スロット判定(§1)+ 全 profile の評価(§2、`ctx.touched`/`ctx.warnings` を
/// 埋める副作用込み)。「未使用ノード」判定(§3)は呼び出し側(`check`/
/// `check_workspace`)がスコープの違いに応じて別途行う。
fn check_slots_and_eval(ctx: &EvalContext, view: &ViewDef, manifest: &Manifest) -> CheckReport {
    let mut report = CheckReport::default();

    // 1. 未充足スロット: manifest が要求する file+field が view 定義に無い。
    for (path, fm) in &manifest.files {
        let Some(file_def) = view.files.iter().find(|f| &f.path == path) else {
            report.missing_slots.push(format!("file '{path}' がビュー定義に存在しません"));
            continue;
        };
        let declared = eval::declared_field_names(&file_def.content);
        match (fm.fields.is_empty(), &declared) {
            (_, None) if !fm.fields.is_empty() => {
                report.missing_slots.push(format!(
                    "file '{path}': マニフェストは fields {:?} を要求しますが、ビュー定義の内容は fields 構造ではありません",
                    fm.fields
                ));
            }
            (_, Some(have)) => {
                for want in &fm.fields {
                    if !have.contains(want) {
                        report.missing_slots.push(format!("file '{path}': 必須フィールド '{want}' が未定義です"));
                    }
                }
            }
            _ => {}
        }
    }

    // 2. 全 profile を評価してノード到達を記録する(profile フィルタなしで
    //    「どれか1つの profile ででも使われていれば used」とする)。
    for f in &view.files {
        let scope = eval::Scope::default();
        if let Err(e) = eval::eval(ctx, &scope, &f.content) {
            report.eval_errors.push(format!("file '{}': 評価エラー: {e}", f.path));
        }
    }
    report.warnings.extend(ctx.warnings.borrow().iter().cloned());

    report
}

/// 未使用ノード走査(§3)。「粗い定義」(裁量、docs/view-v1-handoff.md)として、
/// alias付きの Table/Record/List だけを対象にする。alias はビュー定義から
/// 引ける「アドレス契約」(D26)なので、alias が付いているのに一度も参照
/// されないのは「宣言したのに使い忘れたスロット」として実用的に意味がある
/// 診断になる。一方、見出し(Section)や地の文(Para)は組版上の構造・narrative
/// であることが多く、alias の有無に関わらず「参照されない」こと自体は
/// ありふれている(例: 「詳細は[職務経歴書](ref:work-history/top)を参照」の
/// ような道案内の段落)ため対象から外す — ここを含めると常時ノイズになり
/// 「ゼロ診断」の実用性が失われる。
///
/// `doc_of`: `Some(m)` なら(ワークスペースモード)、`ctx.touched` が実際に
/// 触れた文書の集合を求め、その文書に属するノードだけを走査対象にする
/// (`check_workspace` のドキュメント参照)。`None`(単一文書モード)なら
/// グラフ全体を対象にする(従来どおり)。
fn scan_unused_nodes(ctx: &EvalContext, report: &mut CheckReport, doc_of: Option<&HashMap<NodeId, String>>) {
    let touched: BTreeSet<_> = ctx.touched.borrow().iter().copied().collect();
    // WP-W4: `{ alias, doc }` で明示的に借りてきたノード(`doc_qualified_touched`)は
    // 「その文書全体がこのビューの対象」の根拠にしない — 1フィールドだけ他文書から
    // 引いたせいで、その文書の無関係な alias 付きノードまで「未使用」判定の対象に
    // 広がってしまうのを防ぐ(crate::check_workspace のドキュメント参照)。
    let doc_qualified = ctx.doc_qualified_touched.borrow();
    let touched_docs: Option<HashSet<&str>> = doc_of.map(|m| {
        touched
            .iter()
            .filter(|id| !doc_qualified.contains(id))
            .filter_map(|id| m.get(id).map(String::as_str))
            .collect()
    });

    for (id, node) in &ctx.graph.nodes {
        let Some(alias) = &node.alias else { continue };
        let is_structured =
            matches!(node.payload, NodePayload::Table(_) | NodePayload::Record(_) | NodePayload::List(_));
        if !is_structured || touched.contains(id) {
            continue;
        }
        if let (Some(m), Some(td)) = (doc_of, &touched_docs) {
            let owner_touched = m.get(id).is_some_and(|d| td.contains(d.as_str()));
            if !owner_touched {
                continue;
            }
        }
        report.unused_nodes.push(format!(
            "node '{alias}'(type={}) はどのファイルからも参照されていません",
            eval::node_type_name(&node.payload)
        ));
    }
    report.unused_nodes.sort();
}

/// 各文書の `contains` 部分木を根(`DocRoot::root`)から辿り、ノード → 所属文書
/// (`DocRoot::path`)の逆引き表を作る(ワークスペースモード専用)。alias の
/// 有無に関わらず全ノードを対象にする(`DocRoot::path` は常に存在するため、
/// D41 の「文書 alias 無しはエラーではない」文書も正しく分類できる — alias 表
/// (`doc_aliases`)だけを使うより頑健)。`contains` で辿れない孤立ノード
/// (Term 等)には所有者が付かない(この関数の呼び出し元は Table/Record/List
/// しか見ないため実害無し)。
fn compute_doc_ownership(graph: &Graph, roots: &[strata_build::DocRoot]) -> HashMap<NodeId, String> {
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
