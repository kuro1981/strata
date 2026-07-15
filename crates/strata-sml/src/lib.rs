//! strata-sml — SML(Strata Markup Language)パーサ(Milestone 1)。
//!
//! スコープは**パース(テキスト → スパン付き SML-AST)まで**。fmt(M2)と build(M3)は
//! 本パーサの出力を消費する別フェーズ(docs/sml-parser-design.md)。
//!
//! 依存方針: strata-core にも tex2math にも依存しない。SML-AST は canonical
//! (strata-core::Graph)とは別物であり、変換は build(M3)の仕事。インライン/ブロック
//! 数式は TeX ソース文字列+スパンのまま保持し、tex2math は build 時に呼ぶ(遅延パース)。
//!
//! アーキテクチャは二層:
//!   - 層A(`scan`): 行単位1パスでファイルをブロックスパン列に分割する
//!   - 層B(`block` / `inline` / `table`): 層Aのスパンを入力に、種別ごとに中身を解釈する
//!
//! 層Bのうち `inline.rs` / `table.rs` は M1 の WP1/WP2 時点ではプレースホルダ
//! (それぞれ WP4 / WP3 が実装を差し替える)。

pub mod ast;
pub mod block;
pub mod error;
pub mod fmt;
pub(crate) mod frontmatter;
pub mod inline;
pub mod record;
pub mod scan;
pub mod span;
pub mod table;
pub mod value;

pub use ast::*;
pub use error::{Diag, DiagKind, Severity};
pub use fmt::{format, format_with, FmtOutput, Patch};
pub use span::Span;

/// パース結果。パーサは最初のエラーで止まらず収集する(sml-spec §8.2 の
/// 「全か無か」判定は呼び出し側 = fmt/build の仕事)。
#[derive(Debug, Clone, PartialEq)]
pub struct ParseOutput {
    /// エラー箇所も含め、可能な限り構造化された AST。
    pub doc: SmlDocument,
    pub diags: Vec<Diag>,
}

/// 公開API: フロントマター(D12)→ 層A(スキャン)→ 層B(ブロック内パース)を統率する。
///
/// `inline`/`table` は層Bの中でもプレースホルダ経由(WP1/WP2時点)。
pub fn parse(src: &str) -> ParseOutput {
    let mut diags = Vec::new();
    let (frontmatter, body_start) = frontmatter::parse_frontmatter(src, &mut diags);
    let raw_blocks = scan::scan_from(src, body_start, &mut diags);
    let blocks = block::build_blocks(src, raw_blocks, &mut diags);
    let doc = SmlDocument { blocks, src_len: src.len(), frontmatter };
    ParseOutput { doc, diags }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_empty_source_yields_empty_document() {
        let out = parse("");
        assert!(out.doc.blocks.is_empty());
        assert_eq!(out.doc.src_len, 0);
        assert!(out.diags.is_empty());
    }

    #[test]
    fn parse_smoke_test_does_not_panic_on_minimal_document() {
        let src = "# Title\n\nA paragraph with **bold**? text.\n";
        let out = parse(src);
        assert_eq!(out.doc.src_len, src.len());
        assert!(out.diags.is_empty());
    }
}
