//! strata-view — 宣言的ビュー定義(D30〜D34)のパースと canonical グラフへの適用。
//!
//! `strata_build::BuildOutput`(canonical グラフ)へビュー定義(YAML)を適用し、
//! テンプレート消費用のデータファイル群(YAML テキスト)を生成する。Typst を
//! 直接は知らない(D30: データ層に留める)。
//!
//! 公開 API: [`parse_view_def`] → [`apply`](ビュー適用) / [`check`](dry-run 検証)。

pub mod ast;
pub mod check;
pub mod def;
pub mod eval;
pub mod manifest;
pub mod value;

pub use check::CheckReport;
pub use def::ViewDef;
pub use manifest::Manifest;
pub use value::YValue;

use strata_build::BuildOutput;

/// 1つの出力ファイル(-o 起点の相対パス + 決定的にシリアライズされた YAML テキスト)。
#[derive(Debug, Clone, PartialEq)]
pub struct OutputFile {
    pub path: String,
    pub yaml: String,
}

/// ビュー定義 YAML をパースする。
pub fn parse_view_def(src: &str) -> Result<ViewDef, String> {
    def::parse(src)
}

/// テンプレート・マニフェスト YAML をパースする。
pub fn parse_manifest(src: &str) -> Result<Manifest, String> {
    manifest::parse(src)
}

/// ビュー定義をグラフへ適用する。`profile` が `Some` ならその profile を含む
/// ファイルだけ(`profiles:` 未指定のファイルは常に含む)、`None` なら全ファイルを
/// 生成する(D34: 1定義から profile 別の複数出力。`--profile` 省略時は全出力)。
///
/// 戻り値は (出力ファイル列(path 昇順で決定的), 警告メッセージ列)。
pub fn apply(build: &BuildOutput, view: &ViewDef, profile: Option<&str>) -> Result<(Vec<OutputFile>, Vec<String>), String> {
    if let Some(p) = profile
        && !view.profiles.iter().any(|x| x == p)
    {
        return Err(format!("未宣言の profile '{p}' です(宣言済み: {:?})", view.profiles));
    }

    let ctx = eval::EvalContext::new(&build.graph);
    let mut outputs = Vec::new();
    for f in &view.files {
        if let Some(p) = profile
            && let Some(fp) = &f.profiles
            && !fp.iter().any(|x| x == p)
        {
            continue;
        }
        let scope = eval::Scope::default();
        let value = eval::eval(&ctx, &scope, &f.content).map_err(|e| format!("{}: {e}", f.path))?;
        let yaml = value::to_yaml_string(&value).map_err(|e| format!("{}: YAML シリアライズ失敗: {e}", f.path))?;
        outputs.push(OutputFile { path: f.path.clone(), yaml });
    }
    outputs.sort_by(|a, b| a.path.cmp(&b.path));
    let warnings = ctx.warnings.into_inner();
    Ok((outputs, warnings))
}

/// dry-run 検証(D33)。
pub fn check(build: &BuildOutput, view: &ViewDef, manifest: &Manifest) -> CheckReport {
    check::check(build, view, manifest)
}
