//! view 定義 YAML の生の serde 表現 → `ast` への変換。
//!
//! 「kitchen sink struct」方式(全コンビネータ/セレクタの候補フィールドを1つの
//! struct に並べ、デシリアライズ後にどれか1つだけが Some であることを検証する)を
//! 採る。tagged enum より YAML が読みやすくなる(D34: 「読んで分かる」ことが v1 の
//! 品質基準)ための裁量。
//!
//! `RawCombinator`(コンビネータの値位置)だけは kitchen sink struct の外側に
//! もう1段、`untagged` enum を被せている: 裸文字列 `alias.キー` を
//! record フィールド抽出(`pick`)の糖衣として受理するため(D35)。`RawSelector`
//! が bare `"self"` を同じ手法で受理しているのと対称の設計だが、型が別
//! (Selector 位置 vs Combinator 位置)なので両者は衝突しない。
use crate::ast::{AsType, Combinator, ExtendPath, RowSource, Selector};
use crate::value::from_def_value;
use serde::Deserialize;
use std::collections::BTreeMap;

pub type DefResult<T> = Result<T, String>;

// --------------------------------------------------------------------------
// トップレベル
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct RawViewDef {
    pub version: u32,
    pub profiles: Vec<String>,
    /// テンプレート・マニフェスト(D33)への相対パス(この view 定義ファイル自身の
    /// ディレクトリが基準)。`strata view --check` が使う。省略可(--check 非使用の
    /// 定義には不要)。
    #[serde(default)]
    pub manifest: Option<String>,
    pub files: BTreeMap<String, RawFileDef>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawFileDef {
    #[serde(default)]
    pub profiles: Option<Vec<String>>,
    pub content: RawCombinator,
}

pub struct FileDef {
    pub path: String,
    pub profiles: Option<Vec<String>>,
    pub content: Combinator,
}

pub struct ViewDef {
    pub profiles: Vec<String>,
    pub manifest: Option<String>,
    pub files: Vec<FileDef>,
}

pub fn parse(src: &str) -> DefResult<ViewDef> {
    let raw: RawViewDef = serde_yaml_ng::from_str(src).map_err(|e| format!("view definition YAML: {e}"))?;
    if raw.version != 1 {
        return Err(format!("unsupported view definition version: {}", raw.version));
    }
    let mut files = Vec::new();
    for (path, f) in raw.files {
        if let Some(ps) = &f.profiles {
            for p in ps {
                if !raw.profiles.contains(p) {
                    return Err(format!(
                        "files.{path}.profiles に未宣言の profile '{p}' が指定されています"
                    ));
                }
            }
        }
        let content = f.content.into_ast().map_err(|e| format!("files.{path}: {e}"))?;
        files.push(FileDef { path, profiles: f.profiles, content });
    }
    Ok(ViewDef { profiles: raw.profiles, manifest: raw.manifest, files })
}

// --------------------------------------------------------------------------
// セレクタ(kitchen sink)
// --------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawSelector {
    /// bare string は "self" のみ許可(現在スコープのノード)。
    Keyword(String),
    Detailed(Box<RawSelectorFields>),
}

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RawSelectorFields {
    #[serde(default)]
    pub alias: Option<String>,
    #[serde(default)]
    pub class: Option<String>,
    #[serde(default, rename = "heading-text")]
    pub heading_text: Option<String>,
    #[serde(default, rename = "record-field")]
    pub record_field: Option<RawRecordField>,
    #[serde(default)]
    pub cell: Option<RawCell>,
    #[serde(default, rename = "alias-from-row")]
    pub alias_from_row: Option<RawAliasFromRow>,
    #[serde(default, rename = "first-child-of-type")]
    pub first_child_of_type: Option<RawFirstChild>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRecordField {
    pub of: RawSelector,
    pub key: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawCell {
    #[serde(default)]
    pub of: Option<RawSelector>,
    pub col: String,
    /// row_path を明示するリテラル(rows のスコープ外から特定行を直接指す場合。
    /// 省略時は現在の row_path(rows/extend-path が積む文脈)を使う)。
    #[serde(default)]
    pub row: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAliasFromRow {
    pub prefix: String,
    pub segment: usize,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawFirstChild {
    pub of: RawSelector,
    #[serde(rename = "type")]
    pub node_type: String,
}

impl RawSelector {
    pub fn into_ast(self) -> DefResult<Selector> {
        match self {
            RawSelector::Keyword(s) if s == "self" => Ok(Selector::SelfNode),
            RawSelector::Keyword(s) => Err(format!("未知のセレクタキーワード '{s}'(有効なのは 'self' のみ)")),
            RawSelector::Detailed(f) => f.into_ast(),
        }
    }
}

impl RawSelectorFields {
    fn into_ast(self) -> DefResult<Selector> {
        let mut count = 0;
        let mut result = None;
        macro_rules! take {
            ($opt:expr, $build:expr) => {
                if let Some(v) = $opt {
                    count += 1;
                    result = Some($build(v)?);
                }
            };
        }
        take!(self.alias, |v: String| -> DefResult<Selector> { Ok(Selector::Alias(v)) });
        take!(self.class, |v: String| -> DefResult<Selector> { Ok(Selector::Class(v)) });
        take!(self.heading_text, |v: String| -> DefResult<Selector> {
            Ok(Selector::HeadingText(v))
        });
        take!(self.record_field, |v: RawRecordField| -> DefResult<Selector> {
            Ok(Selector::RecordField { of: Box::new(v.of.into_ast()?), key: v.key })
        });
        take!(self.cell, |v: RawCell| -> DefResult<Selector> {
            let of = v.of.map(|s| s.into_ast()).transpose()?.map(Box::new);
            Ok(Selector::Cell { of, col: v.col, row: v.row })
        });
        take!(self.alias_from_row, |v: RawAliasFromRow| -> DefResult<Selector> {
            Ok(Selector::AliasFromRow { prefix: v.prefix, segment: v.segment })
        });
        take!(self.first_child_of_type, |v: RawFirstChild| -> DefResult<Selector> {
            Ok(Selector::FirstChildOfType { of: Box::new(v.of.into_ast()?), node_type: v.node_type })
        });
        match count {
            1 => Ok(result.unwrap()),
            0 => Err("セレクタが指定されていません".to_string()),
            _ => Err("セレクタに複数の種類が同時に指定されています".to_string()),
        }
    }
}

// --------------------------------------------------------------------------
// コンビネータ(kitchen sink)
// --------------------------------------------------------------------------

/// 値位置のコンビネータ。裸文字列 `alias.キー` は record フィールド抽出(pick)の
/// 糖衣構文(D35)として受理する — alias の字句は `[A-Za-z0-9_-]+`(ドットを含め
/// ない)なので、最初の `.` で安全に分割できる。セレクタ側の bare `self`
/// (`RawSelector::Keyword`)とは型が別(こちらはコンビネータ位置)なので衝突しない。
/// bare 文字列にドットが無い場合(例えば誤って `self` をここに書いた場合)は
/// 糖衣として解釈できないため、明示的なパースエラーにする。
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RawCombinator {
    Sugar(String),
    Detailed(Box<RawCombinatorFields>),
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawCombinatorFields {
    /// フィールド順(出力の列順)を保つため、生の `serde_yaml_ng::Value`(Mapping は
    /// 挿入順保持)として受け取り、`into_ast` で順に辿って `RawCombinator` へ
    /// 個別デシリアライズする。`BTreeMap` を使うとキーの辞書順に並び替わってしまう
    /// ため避けた(裁量: フィールド順=可読性・diff 安定性に直結するため)。
    #[serde(default)]
    pub fields: Option<serde_yaml_ng::Value>,
    #[serde(default)]
    pub rows: Option<RawRows>,
    #[serde(default)]
    pub join: Option<RawJoin>,
    #[serde(default)]
    pub date: Option<RawDate>,
    #[serde(default)]
    pub age: Option<RawAge>,
    /// `literal: null` を「literal コンビネータが null 値を持つ」と正しく読むため、
    /// `Option<Value>` ではなく専用の `LiteralSlot` を使う(裁量。最終報告参照:
    /// serde の `Option<T>` はどの形式でも YAML の `null` スカラを「フィールド
    /// 自体が無い」と衝突させて畳んでしまう既知の落とし穴があり、`literal: null`
    /// が「未指定」に化けてしまうのを避けるための回避策)。
    #[serde(default)]
    pub literal: LiteralSlot,
    /// D35: 旧名 `rename` を `pick` に改名(実態は値の抽出でありリネームではない)。
    #[serde(default)]
    pub pick: Option<RawPick>,
}

#[derive(Debug, Clone, Default)]
pub enum LiteralSlot {
    #[default]
    Absent,
    Present(serde_yaml_ng::Value),
}

/// 手書きの `Deserialize`: 常に `serde_yaml_ng::Value` として読み(`Value` 自身は
/// null を正当な値として表せるので `Option<T>` の null 畳み込みに引っかからない)、
/// `Present` として包む。「キー自体が無い」場合はこの関数は一切呼ばれず、struct 側の
/// `#[serde(default)]` が `Absent` を補う——これで「未指定」と「null という値」を
/// 区別できる。
impl<'de> Deserialize<'de> for LiteralSlot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        serde_yaml_ng::Value::deserialize(deserializer).map(LiteralSlot::Present)
    }
}

impl RawCombinator {
    pub fn into_ast(self) -> DefResult<Combinator> {
        match self {
            RawCombinator::Sugar(s) => parse_pick_sugar(&s),
            RawCombinator::Detailed(f) => f.into_ast(),
        }
    }
}

/// 糖衣構文 `alias.キー` を `{ pick: { of: { record-field: { of: { alias }, key } } } }`
/// へ機械的に脱糖する(D35)。alias は key 字句 `[A-Za-z0-9_-]+` なのでドットを
/// 含まず、最初の `.` で安全に分割できる。ドットが無ければ糖衣として無効(誤って
/// `self` のような bare キーワードをここに書いた場合など)なので明示的なエラーに
/// する — セレクタの `self` キーワードとは別物であることをエラーメッセージでも
/// 明確にする。
fn parse_pick_sugar(s: &str) -> DefResult<Combinator> {
    let (alias, key) = s.trim().split_once('.').ok_or_else(|| {
        format!(
            "裸文字列 '{s}' は 'alias.キー' の形式である必要があります(record フィールド抽出の糖衣、D35)。\
             セレクタの `self` はここでは使えません — フルの `{{ pick: {{ of: self }} }}` を書いてください"
        )
    })?;
    if alias.is_empty() || key.is_empty() {
        return Err(format!("裸文字列 '{s}' の alias またはキーが空です"));
    }
    Ok(Combinator::Pick {
        of: Selector::RecordField { of: Box::new(Selector::Alias(alias.to_string())), key: key.to_string() },
        as_type: AsType::Text,
    })
}

/// field 順は YAML 上の宣言順を保つ(出力の決定性・可読性のため、`fields` だけは
/// 順序付き Vec で保持したいが、serde_yaml の Mapping ではなく素朴な
/// `BTreeMap<String, _>` を素の受け皿にすると辞書順に並び替わってしまう。
/// そのため `fields` は生 YAML の `serde_yaml_ng::Value` を直接見て順序を復元する。
impl RawCombinatorFields {
    fn into_ast(self) -> DefResult<Combinator> {
        let mut count = 0;
        let mut result = None;
        macro_rules! take {
            ($opt:expr, $build:expr) => {
                if let Some(v) = $opt {
                    count += 1;
                    result = Some($build(v)?);
                }
            };
        }
        take!(self.fields, |v: serde_yaml_ng::Value| -> DefResult<Combinator> {
            let serde_yaml_ng::Value::Mapping(m) = v else {
                return Err("fields はマッピングである必要があります".to_string());
            };
            let mut entries = Vec::new();
            for (k, val) in m {
                let key = k.as_str().ok_or_else(|| "fields のキーは文字列である必要があります".to_string())?.to_string();
                let raw: RawCombinator = serde_yaml_ng::from_value(val)
                    .map_err(|e| format!("fields.{key}: {e}"))?;
                entries.push((key.clone(), raw.into_ast().map_err(|e| format!("fields.{key}: {e}"))?));
            }
            Ok(Combinator::Fields(entries))
        });
        take!(self.rows, |v: RawRows| -> DefResult<Combinator> { v.into_ast() });
        take!(self.join, |v: RawJoin| -> DefResult<Combinator> { v.into_ast() });
        take!(self.date, |v: RawDate| -> DefResult<Combinator> { v.into_ast() });
        take!(self.age, |v: RawAge| -> DefResult<Combinator> { v.into_ast() });
        if let LiteralSlot::Present(v) = self.literal {
            count += 1;
            result = Some(Combinator::Literal(from_def_value(&v)));
        }
        take!(self.pick, |v: RawPick| -> DefResult<Combinator> {
            Ok(Combinator::Pick { of: v.of.into_ast()?, as_type: parse_as(v.r#as)? })
        });
        match count {
            1 => Ok(result.unwrap()),
            0 => Err("コンビネータが指定されていません".to_string()),
            _ => Err("コンビネータに複数の種類が同時に指定されています".to_string()),
        }
    }
}

fn parse_as(v: Option<String>) -> DefResult<AsType> {
    match v.as_deref() {
        None | Some("text") => Ok(AsType::Text),
        Some("int") => Ok(AsType::Int),
        Some(other) => Err(format!("未知の as: '{other}'(text または int)")),
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRows {
    #[serde(default)]
    pub table: Option<RawSelector>,
    #[serde(default)]
    pub contains: Option<RawSelector>,
    #[serde(default, rename = "type")]
    pub node_type: Option<String>,
    #[serde(default, rename = "extend-path")]
    pub extend_path: Option<RawExtendPath>,
    pub item: Box<RawCombinator>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawExtendPath {
    #[serde(rename = "alias-suffix")]
    pub alias_suffix: RawAliasSuffix,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAliasSuffix {
    pub prefix: String,
}

impl RawRows {
    fn into_ast(self) -> DefResult<Combinator> {
        let item = Box::new(self.item.into_ast()?);
        let source = match (self.table, self.contains) {
            (Some(t), None) => {
                if self.node_type.is_some() || self.extend_path.is_some() {
                    return Err("rows.table と type/extend-path は併用できません".to_string());
                }
                RowSource::Table(t.into_ast()?)
            }
            (None, Some(c)) => RowSource::Contains {
                of: c.into_ast()?,
                node_type: self.node_type,
                extend_path: self
                    .extend_path
                    .map(|ep| ExtendPath::AliasSuffix { prefix: ep.alias_suffix.prefix }),
            },
            (None, None) => return Err("rows には table か contains のどちらかが必要です".to_string()),
            (Some(_), Some(_)) => return Err("rows.table と rows.contains は併用できません".to_string()),
        };
        Ok(Combinator::Rows { source, item })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawJoin {
    pub of: RawSelector,
    pub separator: String,
    #[serde(default, rename = "nested-prefix")]
    pub nested_prefix: Option<String>,
    #[serde(default, rename = "include-only-class")]
    pub include_only_class: Option<String>,
    #[serde(default, rename = "exclude-class")]
    pub exclude_class: Option<String>,
    #[serde(default)]
    pub keys: Option<Vec<String>>,
}

impl RawJoin {
    fn into_ast(self) -> DefResult<Combinator> {
        Ok(Combinator::Join {
            of: self.of.into_ast()?,
            separator: self.separator,
            nested_prefix: self.nested_prefix,
            include_only_class: self.include_only_class,
            exclude_class: self.exclude_class,
            keys: self.keys,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawDate {
    pub of: RawSelector,
    pub format: String,
    #[serde(default, rename = "period-separator")]
    pub period_separator: Option<String>,
    #[serde(default, rename = "period-open")]
    pub period_open: Option<String>,
    #[serde(default)]
    pub r#as: Option<String>,
}

impl RawDate {
    fn into_ast(self) -> DefResult<Combinator> {
        Ok(Combinator::Date {
            of: self.of.into_ast()?,
            format: self.format,
            period_separator: self.period_separator,
            period_open: self.period_open,
            as_type: parse_as(self.r#as)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawAge {
    pub birth: RawSelector,
    #[serde(rename = "as-of")]
    pub as_of: RawSelector,
    #[serde(default)]
    pub r#as: Option<String>,
}

impl RawAge {
    fn into_ast(self) -> DefResult<Combinator> {
        Ok(Combinator::Age {
            birth: self.birth.into_ast()?,
            as_of: self.as_of.into_ast()?,
            as_type: parse_as(self.r#as)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawPick {
    pub of: RawSelector,
    #[serde(default)]
    pub r#as: Option<String>,
}
