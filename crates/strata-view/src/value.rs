//! strata-view の中間値表現(YValue)。
//!
//! Typst の `yaml()` が読む出力 YAML と、view 定義内の `literal` の両方をこの型で
//! 表す。フィールド順は挿入順を保持する(Vec ベース)。決定的出力(WP-W1 要件)の
//! ため、`serde_yaml_ng::Value::Mapping` へ変換する際も挿入順のまま渡す
//! (serde_yaml の Mapping は IndexMap ベースで順序保存)。
#[derive(Debug, Clone, PartialEq)]
pub enum YValue {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Seq(Vec<YValue>),
    Map(Vec<(String, YValue)>),
}

impl YValue {
    pub fn str(s: impl Into<String>) -> Self {
        YValue::Str(s.into())
    }

    /// 文字列表現(内部の比較・キャストに使う。Map/Seq は空文字列扱い)。
    pub fn as_text(&self) -> String {
        match self {
            YValue::Null => String::new(),
            YValue::Bool(b) => b.to_string(),
            YValue::Int(i) => i.to_string(),
            YValue::Str(s) => s.clone(),
            YValue::Seq(_) | YValue::Map(_) => String::new(),
        }
    }

    pub fn is_empty_text(&self) -> bool {
        matches!(self, YValue::Null) || self.as_text().is_empty()
    }
}

impl From<&YValue> for serde_yaml_ng::Value {
    fn from(v: &YValue) -> Self {
        use serde_yaml_ng::Value as V;
        match v {
            YValue::Null => V::Null,
            YValue::Bool(b) => V::Bool(*b),
            YValue::Int(i) => V::Number((*i).into()),
            YValue::Str(s) => V::String(s.clone()),
            YValue::Seq(items) => V::Sequence(items.iter().map(V::from).collect()),
            YValue::Map(entries) => {
                let mut m = serde_yaml_ng::Mapping::new();
                for (k, val) in entries {
                    m.insert(V::String(k.clone()), V::from(val));
                }
                V::Mapping(m)
            }
        }
    }
}

/// 決定的な YAML テキストへシリアライズする。
pub fn to_yaml_string(v: &YValue) -> Result<String, String> {
    let value: serde_yaml_ng::Value = v.into();
    serde_yaml_ng::to_string(&value).map_err(|e| e.to_string())
}

// `literal:` の値(view 定義 YAML 内)を YValue へ変換。
pub fn from_def_value(v: &serde_yaml_ng::Value) -> YValue {
    use serde_yaml_ng::Value as V;
    match v {
        V::Null => YValue::Null,
        V::Bool(b) => YValue::Bool(*b),
        V::Number(n) => {
            if let Some(i) = n.as_i64() {
                YValue::Int(i)
            } else {
                YValue::Str(n.to_string())
            }
        }
        V::String(s) => YValue::Str(s.clone()),
        V::Sequence(items) => YValue::Seq(items.iter().map(from_def_value).collect()),
        V::Mapping(m) => YValue::Map(
            m.iter()
                .map(|(k, val)| (k.as_str().unwrap_or_default().to_string(), from_def_value(val)))
                .collect(),
        ),
        V::Tagged(t) => from_def_value(&t.value),
    }
}

/// デバッグ用: JSON へ変換(テストでの比較を書きやすくする)。
#[allow(dead_code)]
pub fn to_json(v: &YValue) -> serde_json::Value {
    match v {
        YValue::Null => serde_json::Value::Null,
        YValue::Bool(b) => serde_json::Value::Bool(*b),
        YValue::Int(i) => serde_json::Value::Number((*i).into()),
        YValue::Str(s) => serde_json::Value::String(s.clone()),
        YValue::Seq(items) => serde_json::Value::Array(items.iter().map(to_json).collect()),
        YValue::Map(entries) => {
            let mut m = serde_json::Map::new();
            for (k, val) in entries {
                m.insert(k.clone(), to_json(val));
            }
            serde_json::Value::Object(m)
        }
    }
}
