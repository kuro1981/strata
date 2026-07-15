//! `strata view --check`(D33)の dry-run 診断。
//!
//! 「未充足スロット」: マニフェストが要求するファイル/フィールドが、ビュー定義側に
//! 宣言されていない。「未使用ノード」: グラフのノードのうち、どのファイル・
//! どの profile の評価によっても一度も参照(セレクタで解決)されなかったもの
//! (粗い定義。裁量、docs/view-v1-handoff.md 参照)。
use crate::def::ViewDef;
use crate::eval::{self, EvalContext};
use crate::manifest::Manifest;
use std::collections::BTreeSet;
use strata_build::BuildOutput;
use strata_core::NodePayload;

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
    let ctx = EvalContext::new(&build.graph);
    for f in &view.files {
        let scope = eval::Scope::default();
        if let Err(e) = eval::eval(&ctx, &scope, &f.content) {
            report.eval_errors.push(format!("file '{}': 評価エラー: {e}", f.path));
        }
    }
    report.warnings.extend(ctx.warnings.borrow().iter().cloned());

    // 3. 未使用ノード: 「粗い定義」(裁量、docs/view-v1-handoff.md)として、
    //    alias付きの Table/Record/List だけを対象にする。alias はビュー定義から
    //    引ける「アドレス契約」(D26)なので、alias が付いているのに一度も参照
    //    されないのは「宣言したのに使い忘れたスロット」として実用的に意味がある
    //    診断になる。一方、見出し(Section)や地の文(Para)は組版上の構造・narrative
    //    であることが多く、alias の有無に関わらず「参照されない」こと自体は
    //    ありふれている(例: 「詳細は work_history.sml を参照」という道案内の
    //    段落)ため対象から外す — ここを含めると常時ノイズになり「ゼロ診断」の
    //    実用性が失われる。
    let touched: BTreeSet<_> = ctx.touched.borrow().iter().copied().collect();
    for (id, node) in &build.graph.nodes {
        let Some(alias) = &node.alias else { continue };
        let is_structured =
            matches!(node.payload, NodePayload::Table(_) | NodePayload::Record(_) | NodePayload::List(_));
        if !is_structured {
            continue;
        }
        if !touched.contains(id) {
            report.unused_nodes.push(format!(
                "node '{alias}'(type={}) はどのファイルからも参照されていません",
                eval::node_type_name(&node.payload)
            ));
        }
    }
    report.unused_nodes.sort();

    report
}
