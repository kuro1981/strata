//! バイトオフセットスパン `[start, end)`。全 AST ノードの位置情報はこれを介して持つ。
//!
//! fmt(M2)のスパンパッチ方式(sml-spec D6)は「パーサがスパンを正確に持つこと」に
//! 全体重を掛けている。行/列への変換は都度計算する(sml-parser-m1-handoff.md の
//! 「実装中に裁量で決めてよいこと」に従い、キャッシュは持たない — エラー表示のためだけに
//! 使う低頻度の変換のため)。

use serde::{Deserialize, Serialize};

/// バイトオフセットの半開区間 `[start, end)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        debug_assert!(start <= end, "Span: start must be <= end ({start} > {end})");
        Span { start, end }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// この範囲に対応する元テキストの部分文字列。
    ///
    /// スパンは常に文字境界上にあるべきだが(スキャナが行境界・ASCII記号境界でのみ
    /// 切るため)、万一ずれた場合は panic するより空文字列側に倒す方が安全な用途もある。
    /// ここでは仕様上スパンが常に有効という前提を置き、素直にスライスする。
    pub fn slice<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }

    /// 1始まりの (行, 列) に変換する。都度スキャンで求める。
    pub fn line_col(&self, src: &str) -> (usize, usize) {
        let mut line = 1usize;
        let mut col = 1usize;
        for (i, ch) in src.char_indices() {
            if i >= self.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_roundtrips() {
        let src = "abcdef";
        let span = Span::new(2, 4);
        assert_eq!(span.slice(src), "cd");
    }

    #[test]
    fn line_col_counts_newlines() {
        let src = "abc\ndef\nghi";
        // "ghi" の 'g' は offset 8, 3行目1列目
        let span = Span::new(8, 9);
        assert_eq!(span.line_col(src), (3, 1));
    }
}
