//! 診断(Diag)。パーサは最初のエラーで止まらず収集する(sml-parser-design.md §6)。
//!
//! 「全か無か」(sml-spec §8.2)の裁定は呼び出し側(fmt/build)の仕事: `diags` が
//! 非空なら何もしない、という判断はここではしない。将来の LSP は部分 AST を使える。

use crate::span::Span;

/// 診断の種別。tex2math の `ParseError` 同様、型で分類する。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// 1件の診断。位置(スパン)と人間可読メッセージを持つ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diag {
    pub kind: DiagKind,
    pub span: Span,
    pub msg: String,
}

impl Diag {
    pub fn new(kind: DiagKind, span: Span, msg: impl Into<String>) -> Self {
        Diag { kind, span, msg: msg.into() }
    }
}
