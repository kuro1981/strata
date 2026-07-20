//! クエリ構文(D56): 素のテキスト(空白区切り、AND)+構造述語プレフィックス
//! `class:<tag>` / `term:<用語>` / `alias:<接頭辞>`。述語の構文は最小
//! (プレフィックスは小文字固定・`:` の後ろがそのまま値。値自体のクォート・
//! エスケープ・OR/NOT は無い)。拡張余地は最終報告に記載。

/// パース済みクエリ。全条件は AND(空白区切りの各トークンが1条件)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    /// 素のテキスト条件(部分文字列一致、大小無視)。
    pub text: Vec<String>,
    /// `class:<tag>` — 実効 class(祖先込み)が `tag` を含むこと(完全一致)。
    pub class: Vec<String>,
    /// `term:<用語>` — 用語名の部分一致(そのノード自身が Term か、そのブロックが
    /// term-ref エッジでその用語を使っていること)。
    pub term: Vec<String>,
    /// `alias:<接頭辞>` — ノード alias がこの接頭辞で始まること。
    pub alias_prefix: Vec<String>,
}

impl Query {
    /// クエリ文字列をパースする。空白(Unicode 空白类)区切りのトークン列を左から見て、
    /// `class:`/`term:`/`alias:` のいずれかで始まり値が非空ならその述語、それ以外は
    /// 素のテキスト条件として扱う(トークン中の他の位置の `:` は特別扱いしない —
    /// 例えば時刻表記 `12:30` はテキスト条件としてそのまま入る)。
    pub fn parse(raw: &str) -> Self {
        let mut q = Query::default();
        for tok in raw.split_whitespace() {
            if let Some(v) = tok.strip_prefix("class:").filter(|v| !v.is_empty()) {
                q.class.push(v.to_string());
            } else if let Some(v) = tok.strip_prefix("term:").filter(|v| !v.is_empty()) {
                q.term.push(v.to_string());
            } else if let Some(v) = tok.strip_prefix("alias:").filter(|v| !v.is_empty()) {
                q.alias_prefix.push(v.to_string());
            } else {
                q.text.push(tok.to_string());
            }
        }
        q
    }

    /// 何の条件も無い(空文字列や空白のみの入力)かどうか。
    pub fn is_empty(&self) -> bool {
        self.text.is_empty() && self.class.is_empty() && self.term.is_empty() && self.alias_prefix.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_terms() {
        let q = Query::parse("アジャイル 開発");
        assert_eq!(q.text, vec!["アジャイル", "開発"]);
        assert!(q.class.is_empty() && q.term.is_empty() && q.alias_prefix.is_empty());
    }

    #[test]
    fn parses_mixed_predicates_and_text() {
        let q = Query::parse("class:note term:アジャイル alias:proj- 予算");
        assert_eq!(q.class, vec!["note"]);
        assert_eq!(q.term, vec!["アジャイル"]);
        assert_eq!(q.alias_prefix, vec!["proj-"]);
        assert_eq!(q.text, vec!["予算"]);
    }

    #[test]
    fn empty_predicate_value_falls_back_to_text_token() {
        let q = Query::parse("class: term:");
        assert!(q.class.is_empty() && q.term.is_empty());
        assert_eq!(q.text, vec!["class:", "term:"]);
    }

    #[test]
    fn colon_inside_bare_text_is_not_special() {
        let q = Query::parse("12:30");
        assert_eq!(q.text, vec!["12:30"]);
    }

    #[test]
    fn empty_string_is_empty_query() {
        assert!(Query::parse("").is_empty());
        assert!(Query::parse("   ").is_empty());
    }
}
