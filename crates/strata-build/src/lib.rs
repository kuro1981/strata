//! strata-build — SML(層1)→ canonical グラフ(層2)への変換(Milestone 3)。
//!
//! スコープは docs/sml-build-m3-handoff.md D-B1/D-B5 が定義する範囲まで:
//! パース → エイリアス解決(2パス) → ノード/エッジ構築 → `invariants::validate`。
//! HTML/Typst レンダラへの接続は行わない。
//!
//! エラー方針(D13、全か無か): `build` はパース診断・解決エラーを1件でも検出したら
//! グラフを返さず、収集した全 `BuildError` を返す。D17(2026-07-14 裁定)でこの
//! 「全か無か」の対象を **`strata_sml::Severity::Error` の診断のみ**に絞った:
//! `Warning`(`DuplicateFrontmatterKey` / `UnknownAttrKey`)だけなら build は成功し、
//! `BuildOutput::warnings` に詰めて返す。

mod convert;
mod error;
mod list_child;
mod math;
mod resolve;
mod term;
mod workspace;

pub use error::BuildError;
pub use workspace::{build_workspace, doc_ownership, DocRoot, FileDiag, Member, WorkspaceBuildOutput, WorkspaceError};

use serde::{Deserialize, Serialize};
use strata_core::{Graph, NodeId};

/// `build` の成功時の戻り値。`Graph` 単体とは形が違う(D-B1)ため、CLI(WP-B5)は
/// この構造体ごと `serde_json` でシリアライズ/ラウンドトリップする。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BuildOutput {
    pub graph: Graph,
    /// フロントマター(Document ノード)があればその ID。レンダラ接続時のルート。
    pub root: Option<NodeId>,
    /// パース診断のうち `Warning` のもの(D17)。「全か無か」は `Error` にのみ適用する
    /// ため、`Warning` だけの入力でも build は成功し、ここに詰めて呼び出し側へ返す。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<strata_sml::Diag>,
}

/// 公開 API: SML ソースから canonical グラフを構築する(sml-build-m3-handoff.md D-B1)。
///
/// 全か無か(D13、D17で対象を `Error` のみに絞った): パース診断のうち `Error` の
/// もの、および解決エラー(MissingId/UnresolvedAlias/DuplicateAlias/BadFigure/Math/
/// RefTypeMismatch。これらは常に `Error` 相当)が1件でもあれば `Err` で全件を返す。
/// パース診断の `Warning` はエラー扱いせず `BuildOutput::warnings` に集める。
/// エラーが無ければ `strata_core::invariants::validate` を実行し、違反があればそれも
/// `BuildError` として返す(build 自体のバグ検出網)。
pub fn build(src: &str) -> Result<BuildOutput, Vec<BuildError>> {
    let parsed = strata_sml::parse(src);
    let (parse_errors, warnings): (Vec<_>, Vec<_>) =
        parsed.diags.into_iter().partition(strata_sml::Diag::is_error);
    let mut errors: Vec<BuildError> = parse_errors.into_iter().map(BuildError::Parse).collect();

    let registry = resolve::build_registry(&parsed.doc, &mut errors);
    let mut shared = convert::SharedState::new();
    // 単一ファイル build には cross_doc/doc_index が無い(`None`)。doc 修飾参照
    // (`<文書alias>/<ブロックalias>`)に遭遇すると `BuildError::CrossDocRef` になる
    // (WP-W1.3、`--workspace` の必要性を案内)。`doc:` 参照(D53)は自文書 alias だけは
    // それでも解決できる(`resolve_doc_ref_target` が `reg.alias_table` を先に見る)。
    let (root, pass2_errors) = convert::run(src, &parsed.doc, registry, &mut shared, None, None);
    errors.extend(pass2_errors);

    if !errors.is_empty() {
        return Err(errors);
    }

    let violations = strata_core::invariants::validate(&shared.graph);
    if !violations.is_empty() {
        return Err(violations.into_iter().map(BuildError::Invariant).collect());
    }

    Ok(BuildOutput { graph: shared.graph, root, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_source_builds_empty_graph() {
        let out = build("").expect("empty source has no diags/errors");
        assert!(out.graph.nodes.is_empty());
        assert!(out.graph.edges.is_empty());
        assert_eq!(out.root, None);
    }

    /// D-B6(WP-B5 の CLI)は `BuildOutput` を `serde_json` でラウンドトリップする
    /// 想定(sml-build-m3-handoff.md「既知の注意点」)。ここでその前提が壊れないことを
    /// 固定しておく。
    #[test]
    fn build_output_roundtrips_through_json() {
        let src = "# Title {#01ARZ3NDEKTSV4RRFFQ69G5FAV}\n";
        let out = build(src).unwrap();
        let json = serde_json::to_string_pretty(&out).unwrap();
        let back: BuildOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(out, back);
    }
}
