//! 層B: `::record` フェンス本体パース(sml-spec.md §1.5・§6、D28)。
//!
//! 本体は「キー: 値」の行の列。キー = 行頭から最初の `:` までの自由テキスト
//! (日本語可、前後空白トリム)。値は `crate::value`(D4/D29)の型付きパースを表と
//! 共有する。フェンス内 `#` コメント・空行は既存(`table.rs`)を踏襲する。
//!
//! ## 実装上の裁量(報告対象)
//!
//! - 空キー・`:` 無し行は `Error` 重大度(構造そのものが特定できないため、
//!   全か無かで弾く)。重複キーは `Warning`(値は失わず全件を順序保存列に残す —
//!   フロントマターの「後勝ち」とは異なり、record は複数値を許すデータモデルの
//!   ため「捨てない」方を選んだ)
//! - 重複キー検出時、先勝ち/後勝ちのどちらを「代表値」とするかは決めない
//!   (両方とも `entries` にそのまま残る。消費側=ビューの仕事)

use std::collections::HashSet;

use crate::ast::{RecordBody, RecordEntry};
use crate::error::{Diag, DiagKind};
use crate::scan::split_lines_range;
use crate::span::Span;
use crate::value::{parse_typed_value, DateFormat};

/// `::record` フェンスの本体(フェンス内属性行を除いた残り)をパースする。
pub fn parse_record_body(
    src: &str,
    body: Span,
    date_format: Option<DateFormat>,
    diags: &mut Vec<Diag>,
) -> RecordBody {
    let mut entries = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for pl in split_lines_range(src, body) {
        let raw = pl.content.slice(src);
        let content = raw.trim();
        if content.is_empty() || content.starts_with('#') {
            continue;
        }

        let Some(colon_idx) = content.find(':') else {
            diags.push(Diag::new(
                DiagKind::RecordMissingColon,
                pl.content,
                "record 本体の行に ':' がありません(構文: 'キー: 値')".to_string(),
            ));
            continue;
        };
        let key = content[..colon_idx].trim();
        if key.is_empty() {
            diags.push(Diag::new(
                DiagKind::RecordEmptyKey,
                pl.content,
                "record のキーが空です".to_string(),
            ));
            continue;
        }
        let value_raw = content[colon_idx + 1..].trim();
        let value = parse_typed_value(value_raw, date_format, pl.content, diags);

        if !seen.insert(key.to_string()) {
            diags.push(Diag::new(
                DiagKind::DuplicateRecordKey,
                pl.content,
                format!("record キー '{key}' が複数回宣言されています(値はすべて保持されます)"),
            ));
        }

        entries.push(RecordEntry { key: key.to_string(), value, span: pl.content });
    }

    RecordBody { entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{CellRaw, DateRaw};

    fn body(src: &str) -> (RecordBody, Vec<Diag>) {
        let mut diags = Vec::new();
        let rb = parse_record_body(src, Span::new(0, src.len()), None, &mut diags);
        (rb, diags)
    }

    #[test]
    fn empty_body_yields_empty_record() {
        let (rb, diags) = body("");
        assert!(rb.entries.is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn keys_are_free_text_and_japanese() {
        let (rb, diags) = body("姓: 山田\n名: 太郎\n");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(rb.entries.len(), 2);
        assert_eq!(rb.entries[0].key, "姓");
        assert_eq!(rb.entries[0].value, CellRaw::Text("山田".to_string()));
        assert_eq!(rb.entries[1].key, "名");
        assert_eq!(rb.entries[1].value, CellRaw::Text("太郎".to_string()));
    }

    #[test]
    fn value_only_splits_on_first_colon() {
        // 値の中に ':' が含まれても、キーは最初の ':' までで確定する。
        let (rb, diags) = body("時刻: 10:30\n");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(rb.entries[0].key, "時刻");
        assert_eq!(rb.entries[0].value, CellRaw::Text("10:30".to_string()));
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let (rb, diags) = body("# comment\n姓: 山田\n\n# another\n名: 太郎\n");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(rb.entries.len(), 2);
    }

    #[test]
    fn date_value_field() {
        let (rb, diags) = body("生年月日: 1997-03-15\n");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(rb.entries[0].value, CellRaw::Date(DateRaw { y: 1997, m: 3, d: Some(15) }));
    }

    #[test]
    fn phone_number_like_value_is_text_without_diagnostic() {
        let (rb, diags) = body("電話番号: 080-1234-5678\n");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(rb.entries[0].value, CellRaw::Text("080-1234-5678".to_string()));
    }

    #[test]
    fn missing_colon_is_diagnosed_and_line_skipped() {
        let (rb, diags) = body("キーだけの行\n姓: 山田\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::RecordMissingColon), "{diags:?}");
        assert_eq!(rb.entries.len(), 1);
        assert_eq!(rb.entries[0].key, "姓");
    }

    #[test]
    fn empty_key_is_diagnosed_and_line_skipped() {
        let (rb, diags) = body(": 値だけ\n姓: 山田\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::RecordEmptyKey), "{diags:?}");
        assert_eq!(rb.entries.len(), 1);
    }

    #[test]
    fn duplicate_key_is_warning_and_both_values_kept() {
        let (rb, diags) = body("姓: 山田\n姓: 田中\n");
        assert_eq!(rb.entries.len(), 2, "duplicate entries must both be retained");
        let dup = diags.iter().find(|d| d.kind == DiagKind::DuplicateRecordKey).expect("expected DuplicateRecordKey");
        assert_eq!(dup.severity, crate::error::Severity::Warning);
    }

    #[test]
    fn date_format_is_threaded_through() {
        let mut diags = Vec::new();
        let src = "生年月日: 1997年3月15日\n";
        let rb = parse_record_body(
            src,
            Span::new(0, src.len()),
            Some(DateFormat::YearMonthDayJa),
            &mut diags,
        );
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(rb.entries[0].value, CellRaw::Date(DateRaw { y: 1997, m: 3, d: Some(15) }));
    }
}
