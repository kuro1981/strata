//! テンプレート・マニフェスト(D33)。テンプレートが実際に読むスロット(ファイル+
//! フィールド)を宣言する YAML。`strata view --check` の突合対象。
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
pub struct RawManifest {
    pub version: u32,
    pub files: BTreeMap<String, RawFileManifest>,
    #[serde(default)]
    #[allow(dead_code)]
    pub notes: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawFileManifest {
    /// "fields"(ファイル全体が1つの dict)または "rows"(ファイルが行の配列。
    /// 各行が dict なら fields はその必須キー、行がスカラなら fields は空)。
    pub shape: String,
    #[serde(default)]
    pub fields: Vec<String>,
}

pub struct Manifest {
    pub files: Vec<(String, RawFileManifest)>,
}

pub fn parse(src: &str) -> Result<Manifest, String> {
    let raw: RawManifest = serde_yaml_ng::from_str(src).map_err(|e| format!("manifest YAML: {e}"))?;
    if raw.version != 1 {
        return Err(format!("unsupported manifest version: {}", raw.version));
    }
    for (path, f) in &raw.files {
        if f.shape != "fields" && f.shape != "rows" {
            return Err(format!("files.{path}.shape は 'fields' か 'rows' である必要があります(got '{}')", f.shape));
        }
    }
    Ok(Manifest { files: raw.files.into_iter().collect() })
}
