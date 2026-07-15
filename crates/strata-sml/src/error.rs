//! 診断(Diag)。パーサは最初のエラーで止まらず収集する(sml-parser-design.md §6)。
//!
//! 「全か無か」(sml-spec §8.2)の裁定は呼び出し側(fmt/build)の仕事: `diags` が
//! 非空なら何もしない、という判断はここではしない。将来の LSP は部分 AST を使える。
//!
//! D17(2026-07-14 裁定): 診断には severity(`Error` / `Warning`)がある。「全か無か」は
//! **`Error` にのみ適用**する — `Warning` だけの場合は fmt/build は成功し、`Warning` を
//! 結果と併せて呼び出し側に返す(`FmtOutput::warnings` / `BuildOutput::warnings`)。
//! 新設した2種別(`DuplicateFrontmatterKey` / `UnknownAttrKey`)のみ `Warning`。
//! 既存の種別はすべて `Error` のまま(挙動は変えない)。

use serde::{Deserialize, Serialize};

use crate::span::Span;

/// 診断の重大度(D17)。「全か無か」の判定基準は `Error` の有無だけを見る。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
}

/// 診断の種別。tex2math の `ParseError` 同様、型で分類する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagKind {
    /// `::`フェンス/コードフェンスが閉じられずファイル末尾に到達した。
    UnclosedFence,
    /// 属性行の直後に空行(またはファイル終端)があり、束縛先ブロックがない。
    OrphanAttrLine,
    /// 行型ブロックの `{#...}` とプローズ属性行の `id=` が同じブロックに併記された。
    DuplicateId,
    /// 行型ブロック(見出し・リスト・フェンス)の前置属性行に `id=` が書かれている
    /// (sml-spec §4: id を書けるのはプローズブロックの属性行だけ)。`{#...}` タグとの
    /// 併記は `DuplicateId` になるため、こちらは `{#...}` が**無い**ケース専用。
    IdNotAllowedHere,
    /// key / エイリアス / member key が `[A-Za-z0-9_-]+` の外の文字を含む(D5)。
    BadKeyCharset,
    /// 属性行の `id` の値が裸トークンでない(`[id="..."]` / `[id=[a, b]]`)。
    /// id は ULID または人間ラベルの単一トークンのみ(sml-spec §3.2。2026-07-13 裁定)。
    BadIdValue,
    /// `{#label alias=x}` — 非 ULID の id に alias が併記された。alias を書けるのは
    /// ULID の id だけで、ドラフト段階では `{#label}` とだけ書く(fmt がラベルを
    /// alias へ昇格させる。sml-spec §3.1。2026-07-13 裁定)。
    AliasWithoutUlid,
    /// セル座標(`path | path`)が字句として不正。`::table` 本体の `@cells` セル行
    /// (sml-spec §6.1)と、インライン `cell:` 参照の座標(sml-spec §5.3)の
    /// 両方でこの同じ variant を使う(どちらも座標の文法は §7 の path 規則で共通)。
    BadCellCoord,
    /// `::table` 本体のインデントが2スペース単位で揃っていない。
    InconsistentIndent,
    /// インライン参照のスキームが `ref/term/table/fig/math/cell` のいずれでもない。
    UnknownScheme,
    /// フロントマター(sml-spec §2.1、D12)の `key: value` 行のキーが `id` / `title`
    /// のいずれでもない(v0 は「出たら足す」方針)。
    UnknownFrontmatterKey,
    /// フロントマターの閉じ `---` 単独行が見つからずファイル末尾に到達した。
    UnclosedFrontmatter,
    /// フロントマターの同一キー(`id` / `title` 等)が複数行で宣言されている(D17)。
    /// 挙動は従来どおり後勝ち(最後の出現が採用される)のまま、`Warning`。
    DuplicateFrontmatterKey,
    /// 属性行のキーが `supports` / `depends-on` / `cites` / `id` / `alias` の
    /// いずれでもない(D17)。エッジが張られないタイポの検出用。挙動は従来どおり
    /// 無視のまま(`apply_block_attrs` は未知キーを黙って読み飛ばす)、`Warning`。
    UnknownAttrKey,
    /// `::record` 本体の行に `:` が無い(D28、sml-spec §1.5)。「キー: 値」の構文に
    /// 従っていない行はキーを特定できないため、その行はスキップして処理を続ける。
    RecordMissingColon,
    /// `::record` 本体の行のキーが空(`: 値` のように `:` の前が空白のみ、D28)。
    RecordEmptyKey,
    /// `::record` 本体で同一キーが複数回宣言されている(D28)。値は失わず全件を
    /// 順序保存列にそのまま残す(データを捨てない)ため `Warning`。
    DuplicateRecordKey,
    /// セル値 / record 値が ISO 日付らしき形(`YYYY-MM[-DD]`)、または宣言済み
    /// `date-format=` の形をしているが値レンジが不正(13月等)、あるいは期間の
    /// 片側だけが日付として読めない(D29)。値は Text へフォールバックする。
    BadDateValue,
    /// フェンス属性 `date-format=` の値が未対応(v0 は `"YYYY年M月"` /
    /// `"YYYY年M月D日"` のみ)、またはリスト値など裸トークンでない形(D29)。
    BadDateFormat,
}

impl DiagKind {
    /// D17: 種別ごとの既定 severity。`Error` が既定で、`Warning` は明示した2種別のみ。
    pub fn severity(self) -> Severity {
        match self {
            DiagKind::DuplicateFrontmatterKey | DiagKind::UnknownAttrKey | DiagKind::DuplicateRecordKey => {
                Severity::Warning
            }
            _ => Severity::Error,
        }
    }
}

/// 1件の診断。位置(スパン)と人間可読メッセージを持つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diag {
    pub kind: DiagKind,
    pub span: Span,
    pub msg: String,
    pub severity: Severity,
}

impl Diag {
    /// `kind` から severity を自動決定する(D17)。呼び出し側が severity を
    /// 意識せずに済むよう、既存の `Diag::new(kind, span, msg)` 呼び出し規約を温存する。
    pub fn new(kind: DiagKind, span: Span, msg: impl Into<String>) -> Self {
        Diag { kind, span, msg: msg.into(), severity: kind.severity() }
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }

    pub fn is_warning(&self) -> bool {
        self.severity == Severity::Warning
    }
}
