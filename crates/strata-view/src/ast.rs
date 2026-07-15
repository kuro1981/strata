//! strata-view の内部 AST(セレクタ・コンビネータ)。
//!
//! `def.rs` が YAML(素の serde 表現)からここへ変換する。実行(`eval.rs`)は
//! この AST だけを見る。
use crate::value::YValue;

#[derive(Debug, Clone)]
pub enum Selector {
    /// alias から直接ノードを引く(D31 一級セレクタ)。
    Alias(String),
    /// class を持つ唯一のノードを引く(D31 一級セレクタ)。複数該当は Warning。
    Class(String),
    /// 見出しテキスト一致(D31: Warning 付きエスケープハッチ)。
    HeadingText(String),
    /// `::record` のキーで値を引く。
    RecordField { of: Box<Selector>, key: String },
    /// 表セルをキーで引く(D31: セル座標)。`of` を省略すると直近の `rows: table`
    /// が確立した「現在の表」を使う。row 座標は既定で「現在の row_path」
    /// (rows/extend-path が積む文脈)を使うが、`row` を明示すればその row_path
    /// (リテラル)で上書きできる(rows のスコープ外から特定行を直接指す場合。
    /// 例: tech-stack 表の "languages" 行を rows ループ無しで引く)。
    Cell { of: Option<Box<Selector>>, col: String, row: Option<Vec<String>> },
    /// 「現在の row_path の segment 番目」を接頭辞と連結した alias でノードを引く
    /// (D31 のセル座標セレクタを alias 解決に流用する拡張。裁量。最終報告参照)。
    AliasFromRow { prefix: String, segment: usize },
    /// `of` の直接の子のうち、指定 type の最初の1つ。
    FirstChildOfType { of: Box<Selector>, node_type: String },
    /// 現在のスコープノード自身(`rows: contains` の item 内で使う)。
    SelfNode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsType {
    Text,
    Int,
}

#[derive(Debug, Clone)]
pub enum RowSource {
    /// 表の葉行を宣言順に辿る。
    Table(Selector),
    /// ノードの contains 子(文書順)を辿る。
    Contains {
        of: Selector,
        node_type: Option<String>,
        extend_path: Option<ExtendPath>,
    },
}

#[derive(Debug, Clone)]
pub enum ExtendPath {
    /// 子ノード自身の alias から `prefix` を取り除いた残りを row_path に追加する。
    AliasSuffix { prefix: String },
}

#[derive(Debug, Clone)]
pub enum Combinator {
    /// 名前付きサブコンビネータを Map にまとめる(pick の「まとめ役」)。
    Fields(Vec<(String, Combinator)>),
    Rows {
        source: RowSource,
        item: Box<Combinator>,
    },
    /// 子ノード列(または record のエントリ列)を区切り文字で連結する。
    Join {
        of: Selector,
        separator: String,
        /// リスト項目がさらにリストを子に持つ場合、その孫項目に前置する文字列
        /// (v0 convert_sml.py の flatten_summary_list 相当。省略時は前置なし)。
        nested_prefix: Option<String>,
        /// このクラスを持つ子だけを対象にする。
        include_only_class: Option<String>,
        /// このクラスを持つ子を除外する。
        exclude_class: Option<String>,
        /// `of` が record ノードの場合、対象にするキーの列(宣言順)。各エントリは
        /// `"<key>: <value>"` の行として出力し、値が空のキーは省略する
        /// (v0 の motivation_parts 相当。裁量。最終報告参照)。
        keys: Option<Vec<String>>,
    },
    Date {
        of: Selector,
        /// トークン: YYYY(4桁年) YY(2桁年) M(月) MM(0埋め月) D(日) DD(0埋め日)。
        /// それ以外の文字はリテラル通過(例: "YYYY年M月")。
        format: String,
        /// Period 値のとき from/to の間に挟む文字列(例: "〜" "～" " ~ ")。
        period_separator: Option<String>,
        /// Period 値で to が無い(継続中)ときの表示(例: "現在")。
        period_open: Option<String>,
        as_type: AsType,
    },
    Age {
        birth: Selector,
        as_of: Selector,
        as_type: AsType,
    },
    Literal(YValue),
    /// pick: 値をそのまま(型変換のみ)取り出す(旧名 rename。D35 で改名 —
    /// 実態は値の抽出でありリネームではない)。
    Pick { of: Selector, as_type: AsType },
}
