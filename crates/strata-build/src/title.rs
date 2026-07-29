//! 文書タイトルの解決(D59、sml-spec.md §1.18)。
//!
//! `[[wikilink]]` の解決方式(D59 確定)は「ワークスペース内の文書**タイトル**との
//! 完全一致」。タイトルのフォールバック連鎖は:
//!
//!   1. フロントマター `title:`
//!   2. 無ければ最初の H1(レベル1見出し)のプレーンテキスト
//!   3. どちらも無ければファイル名 stem(拡張子を除いたファイル名)
//!
//! 3番目(ファイル名 stem)は**ファイルパスを知っている呼び出し側だけ**が付加できる
//! (`workspace.rs`)。strata-build の単一ファイル build(`lib.rs::build`)はソース文字列
//! しか受け取らずファイルパスの概念を持たない(D12: 「ファイル → 文書の対応管理は
//! 将来の vault 層の仕事」の既存方針どおり)ため、単一ファイル build での自己参照は
//! `title:` / H1 のみで判定する(裁量、最終報告参照)。

use strata_sml::{BlockKind, SmlDocument, SmlInline};
use unicode_normalization::UnicodeNormalization;

/// D59: wikilink のタイトル一致判定用の正規化。Term ID 正規化(D9/D15、`term.rs`)を
/// 踏襲して **NFC 正規化のみ**を行い、大文字小文字は区別しない変換(lowercase 等)は
/// **行わない**(裁量、最終報告参照 — D15 の Term 名正規化が大文字小文字を区別した
/// ままなのと同じ判断: 日本語タイトルには大文字小文字の概念が無く、英語タイトルの
/// 大文字小文字を勝手に畳むと意図的に大文字小文字で書き分けたタイトルが衝突しうる)。
pub(crate) fn normalize_title(s: &str) -> String {
    s.nfc().collect()
}

/// frontmatter `title:` → 最初の H1 の順でタイトルを決定する(ファイル名 stem への
/// フォールバックはここでは行わない — 上記モジュールコメント参照)。空文字列(または
/// 空白のみ)の `title:` は「無し」として扱い、次点へフォールバックする。
pub(crate) fn document_title(src: &str, doc: &SmlDocument) -> Option<String> {
    if let Some(fm) = &doc.frontmatter
        && let Some(title) = &fm.title
        && !title.trim().is_empty()
    {
        return Some(title.clone());
    }
    first_h1_text(src, doc)
}

fn first_h1_text(src: &str, doc: &SmlDocument) -> Option<String> {
    for block in &doc.blocks {
        if let BlockKind::Heading { level: 1, inline, .. } = &block.kind {
            let text = heading_plain_text(src, inline);
            if !text.trim().is_empty() {
                return Some(text);
            }
        }
    }
    None
}

/// 見出しインラインをプレーンテキストへ平坦化する(強調等の記法は無視)。wikilink の
/// ターゲットは書式なしの文字列なので、見出し側も書式を落として比較する。
fn heading_plain_text(src: &str, inlines: &[SmlInline]) -> String {
    let mut out = String::new();
    push_plain_text(src, inlines, &mut out);
    out
}

fn push_plain_text(src: &str, inlines: &[SmlInline], out: &mut String) {
    for i in inlines {
        match i {
            SmlInline::Text(span) => out.push_str(span.slice(src)),
            SmlInline::Escaped(span) => out.push_str(&span.slice(src)[1..]),
            SmlInline::Emph { children, .. } => push_plain_text(src, children, out),
            // 見出しに参照・数式・画像等が混ざるのは稀だが、タイトル一致の対象は
            // あくまで表示テキストなので表示用の text フィールドだけを拾う。
            SmlInline::Ref { text, .. } | SmlInline::TermRef { text, .. } | SmlInline::Link { text, .. } => {
                out.push_str(text.slice(src));
            }
            // image-support-handoff.md item 2: 画像embedも見出しテキストには寄与しない
            // (Image と同型)。
            SmlInline::Image { .. } | SmlInline::AssetEmbed { .. } | SmlInline::MathTex(_) => {}
        }
    }
}

/// ファイル名 stem(拡張子を除いたファイル名)をタイトルのフォールバックとして返す
/// (D59、ワークスペース build 専用 — `workspace.rs` がメンバーの `path` から呼ぶ)。
pub(crate) fn filename_stem(path: &str) -> Option<String> {
    let stem = std::path::Path::new(path).file_stem()?.to_str()?;
    if stem.is_empty() { None } else { Some(stem.to_string()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc_of(src: &str) -> SmlDocument {
        strata_sml::parse(src).doc
    }

    #[test]
    fn title_from_frontmatter_wins() {
        let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: フロントマターの題\n---\n# 見出し\n";
        let doc = doc_of(src);
        assert_eq!(document_title(src, &doc).as_deref(), Some("フロントマターの題"));
    }

    #[test]
    fn title_falls_back_to_first_h1_when_frontmatter_has_none() {
        let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n---\n# 見出しの題\n\n## 別の見出し\n";
        let doc = doc_of(src);
        assert_eq!(document_title(src, &doc).as_deref(), Some("見出しの題"));
    }

    #[test]
    fn blank_frontmatter_title_falls_back_to_h1() {
        let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\ntitle: \n---\n# H1の題\n";
        let doc = doc_of(src);
        assert_eq!(document_title(src, &doc).as_deref(), Some("H1の題"));
    }

    #[test]
    fn no_title_and_no_h1_yields_none() {
        let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n---\n本文だけ。\n";
        let doc = doc_of(src);
        assert_eq!(document_title(src, &doc), None);
    }

    #[test]
    fn h1_text_ignores_emphasis_markup() {
        let src = "---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAV\n---\n# **強調**された題\n";
        let doc = doc_of(src);
        assert_eq!(document_title(src, &doc).as_deref(), Some("強調された題"));
    }

    #[test]
    fn nfc_normalization_matches_nfd_input() {
        let nfc = "パイプラインが効率的";
        let nfd = "パイプラインか\u{3099}効率的";
        assert_ne!(nfc.as_bytes(), nfd.as_bytes());
        assert_eq!(normalize_title(nfc), normalize_title(nfd));
    }

    #[test]
    fn normalization_preserves_case() {
        assert_ne!(normalize_title("Foo"), normalize_title("foo"));
    }

    #[test]
    fn filename_stem_strips_directory_and_extension() {
        assert_eq!(filename_stem("notes/日記.sml").as_deref(), Some("日記"));
        assert_eq!(filename_stem("plain").as_deref(), Some("plain"));
    }
}
