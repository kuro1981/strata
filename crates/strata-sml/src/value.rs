//! セル値 / record 値の型付きパース(D4、sml-spec §6.1)+ 日付・期間拡張(D28/D29)。
//! `table.rs`(`::table` の `@cells`)と `record.rs`(`::record` の本体)の両方が使う
//! 共通ロジック(sml-spec §1.5「型付きパース(D29): 表セルと record 値で共通」)。
//!
//! **書式スニッフィングはしない**(D29): 既定で受理する日付書式は ISO
//! (`YYYY-MM-DD` / `YYYY-MM`)のみ。フェンス属性 `date-format=` が宣言された場合に
//! 限り、その1書式(v0 は `"YYYY年M月"` / `"YYYY年M月D日"` の最小語彙のみ)も追加で
//! 受理する。ISO っぽい形(`YYYY-MM-DD` の字句形)をしていて値レンジが不正
//! (13月等)なら `BadDateValue` を診断する。一方、日付の形をそもそもしていない
//! 値(電話番号 `080-...` 等)は診断を出さず黙って `Text` にフォールバックする
//! (sml-spec 「既知の注意点」)。

use crate::ast::{CellRaw, DateRaw};
use crate::block::parse_scoped_ref_target;
use crate::error::{Diag, DiagKind};
use crate::span::Span;

/// フェンス属性 `date-format=` で宣言できる追加入力書式(D29 v0、最小語彙)。
/// 書式言語そのものは実装せず、この2パターンのみをハードコードする(裁量。
/// 最終報告に記載)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    /// `YYYY年M月`(日を持たない)。
    YearMonthJa,
    /// `YYYY年M月D日`。
    YearMonthDayJa,
}

/// `date-format=` の属性値文字列をパースする。未対応の文字列は `None`
/// (呼び出し側が `BadDateFormat` を積む)。
pub fn parse_date_format(s: &str) -> Option<DateFormat> {
    match s {
        "YYYY年M月" => Some(DateFormat::YearMonthJa),
        "YYYY年M月D日" => Some(DateFormat::YearMonthDayJa),
        _ => None,
    }
}

/// D4 + D29: セル値 / record 値の型付きパース。`date_format` はそのフェンスで
/// 宣言された追加書式(無ければ ISO のみを試す)。`span` は診断位置(呼び出し側の
/// 行スパン)。
pub fn parse_typed_value(raw: &str, date_format: Option<DateFormat>, span: Span, diags: &mut Vec<Diag>) -> CellRaw {
    let s = raw.trim();
    if s.is_empty() || s == "~" {
        return CellRaw::Empty;
    }
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return CellRaw::Text(s[1..s.len() - 1].to_string());
    }
    if let Some(target) = s.strip_prefix("ref:") {
        return CellRaw::Ref(parse_scoped_ref_target(target.trim()));
    }

    if let Some(period) = try_period(s, date_format, span, diags) {
        return period;
    }

    if let Some(d) = try_parse_date(s, date_format) {
        return CellRaw::Date(d);
    }
    if looks_like_date_shape(s, date_format) {
        diags.push(Diag::new(
            DiagKind::BadDateValue,
            span,
            format!("日付 '{s}' の形式が不正です(既定は YYYY-MM-DD / YYYY-MM のみ受理)"),
        ));
        return CellRaw::Text(s.to_string());
    }

    let tokens: Vec<&str> = s.split_whitespace().collect();
    if tokens.len() == 2
        && let Ok(v) = tokens[0].parse::<f64>()
        && is_valid_unit_token(tokens[1])
    {
        return CellRaw::Quantity { v, unit: tokens[1].to_string() };
    }
    if tokens.len() == 1
        && let Ok(v) = tokens[0].parse::<f64>()
    {
        return CellRaw::Number(v);
    }

    // 裸の非数値テキストは寛容にフォールバック(D4)。日付の形をしていない値
    // (電話番号等)はここに来ても診断は出さない(sml-spec「既知の注意点」)。
    CellRaw::Text(s.to_string())
}

/// 期間「A 〜 B」「A 〜 現在」の試行(D29)。`〜`/`~` を区切りとして両辺を日付として
/// 読めた場合のみ `Some` を返す。`from` 側が日付として読めなければ、期間ではないと
/// みなして `None`(呼び出し側は通常のセル値パースへフォールスルーする)。
fn try_period(
    s: &str,
    date_format: Option<DateFormat>,
    span: Span,
    diags: &mut Vec<Diag>,
) -> Option<CellRaw> {
    let (from_str, to_str) = split_period(s)?;
    if from_str.is_empty() {
        return None;
    }
    let from = try_parse_date(from_str, date_format)?;

    if to_str.is_empty() || to_str == "現在" {
        return Some(CellRaw::Period { from, to: None });
    }
    if let Some(to) = try_parse_date(to_str, date_format) {
        return Some(CellRaw::Period { from, to: Some(to) });
    }
    // from は日付として読めたが to が読めない: to が日付らしい形をしているときだけ
    // 診断する(from 側が偶然日付形の非日付値だった、というケースの誤診断を避ける)。
    if looks_like_date_shape(to_str, date_format) {
        diags.push(Diag::new(
            DiagKind::BadDateValue,
            span,
            format!("期間の終端 '{to_str}' の日付形式が不正です"),
        ));
    }
    Some(CellRaw::Text(s.to_string()))
}

/// `〜`(全角波ダッシュ)または `~`(半角チルダ)で分割する。前後の空白は許容する
/// (sml-spec §1.5 D29)。見つからなければ `None`(期間ではない)。
fn split_period(s: &str) -> Option<(&str, &str)> {
    if let Some(idx) = s.find('〜') {
        return Some((s[..idx].trim(), s[idx + '〜'.len_utf8()..].trim()));
    }
    if let Some(idx) = s.find('~') {
        return Some((s[..idx].trim(), s[idx + 1..].trim()));
    }
    None
}

fn try_parse_date(s: &str, date_format: Option<DateFormat>) -> Option<DateRaw> {
    if let Some(d) = try_parse_iso_date(s) {
        return Some(d);
    }
    match date_format {
        Some(fmt) => try_parse_date_format(s, fmt),
        None => None,
    }
}

/// ISO(`YYYY-MM` / `YYYY-MM-DD`)の厳密パース(D29 既定書式)。
fn try_parse_iso_date(s: &str) -> Option<DateRaw> {
    if s.len() < 7 || !s.is_char_boundary(4) || !s.is_char_boundary(5) {
        return None;
    }
    let y_str = &s[..4];
    if !y_str.bytes().all(|b| b.is_ascii_digit()) || &s[4..5] != "-" {
        return None;
    }
    let rest = &s[5..];
    let parts: Vec<&str> = rest.split('-').collect();
    if parts.len() > 2 {
        return None;
    }
    if !parts.iter().all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let y: i32 = y_str.parse().ok()?;
    let m: u32 = parts[0].parse().ok()?;
    let d: Option<u32> = if parts.len() == 2 { Some(parts[1].parse().ok()?) } else { None };
    if !valid_date(y, m, d) {
        return None;
    }
    Some(DateRaw { y, m, d })
}

/// ISO の字句形(`YYYY-MM[-DD]`、各桁数固定)をしているかどうか(値レンジは見ない)。
/// `BadDateValue` を出すべきか(「日付のつもりで書いたが不正」)を判定するのに使う。
fn looks_like_iso_date_shape(s: &str) -> bool {
    if s.len() < 7 || !s.is_char_boundary(4) || !s.is_char_boundary(5) {
        return false;
    }
    let y_str = &s[..4];
    if !y_str.bytes().all(|b| b.is_ascii_digit()) || &s[4..5] != "-" {
        return false;
    }
    let parts: Vec<&str> = s[5..].split('-').collect();
    if parts.len() > 2 {
        return false;
    }
    parts.iter().all(|p| p.len() == 2 && p.bytes().all(|b| b.is_ascii_digit()))
}

fn looks_like_date_shape(s: &str, date_format: Option<DateFormat>) -> bool {
    if looks_like_iso_date_shape(s) {
        return true;
    }
    match date_format {
        Some(fmt) => looks_like_ja_shape(s, fmt),
        None => false,
    }
}

/// `YYYY年M月` / `YYYY年M月D日` の構造分割。年/月/(日) の生文字列を返す(値レンジは
/// 見ない)。
fn parse_ja_ymd_parts(s: &str) -> Option<(&str, &str, Option<&str>)> {
    let (y_str, rest) = s.split_once('年')?;
    let (m_str, rest2) = rest.split_once('月')?;
    if rest2.is_empty() {
        return Some((y_str, m_str, None));
    }
    let d_str = rest2.strip_suffix('日')?;
    Some((y_str, m_str, Some(d_str)))
}

fn looks_like_ja_shape(s: &str, fmt: DateFormat) -> bool {
    let Some((y, m, d_opt)) = parse_ja_ymd_parts(s) else { return false };
    if y.is_empty() || !y.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    if m.is_empty() || !m.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    match fmt {
        DateFormat::YearMonthJa => d_opt.is_none(),
        DateFormat::YearMonthDayJa => {
            matches!(d_opt, Some(d) if !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        }
    }
}

fn try_parse_date_format(s: &str, fmt: DateFormat) -> Option<DateRaw> {
    let (y_str, m_str, d_opt) = parse_ja_ymd_parts(s)?;
    match (fmt, &d_opt) {
        (DateFormat::YearMonthJa, None) => {}
        (DateFormat::YearMonthDayJa, Some(_)) => {}
        _ => return None,
    }
    if y_str.is_empty() || !y_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if m_str.is_empty() || !m_str.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let y: i32 = y_str.parse().ok()?;
    let m: u32 = m_str.parse().ok()?;
    let d: Option<u32> = match d_opt {
        Some(ds) => {
            if ds.is_empty() || !ds.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            Some(ds.parse().ok()?)
        }
        None => None,
    };
    if !valid_date(y, m, d) {
        return None;
    }
    Some(DateRaw { y, m, d })
}

fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(y) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

fn valid_date(y: i32, m: u32, d: Option<u32>) -> bool {
    if !(1..=12).contains(&m) {
        return false;
    }
    match d {
        Some(d) => d >= 1 && d <= days_in_month(y, m),
        None => true,
    }
}

/// 単位トークンの字句(D4、table.rs と同一。裁量: `[A-Za-zµ%°]` で始まり
/// `[A-Za-z0-9/^·%°-]*` が続く)。
fn is_valid_unit_token(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || matches!(c, 'µ' | '%' | '°') => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '^' | '·' | '%' | '°' | '-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::RefTarget;

    fn parse(raw: &str) -> (CellRaw, Vec<Diag>) {
        parse_with_format(raw, None)
    }

    fn parse_with_format(raw: &str, fmt: Option<DateFormat>) -> (CellRaw, Vec<Diag>) {
        let mut diags = Vec::new();
        let value = parse_typed_value(raw, fmt, Span::new(0, raw.len()), &mut diags);
        (value, diags)
    }

    #[test]
    fn existing_d4_types_still_work() {
        assert_eq!(parse("0.82").0, CellRaw::Number(0.82));
        assert_eq!(parse("45 ms").0, CellRaw::Quantity { v: 45.0, unit: "ms".to_string() });
        assert_eq!(parse("~").0, CellRaw::Empty);
        assert_eq!(parse("\"任意の テキスト\"").0, CellRaw::Text("任意の テキスト".to_string()));
        assert_eq!(parse("some text").0, CellRaw::Text("some text".to_string()));
    }

    #[test]
    fn iso_date_full_and_month_precision() {
        let (v, diags) = parse("1997-03-15");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(v, CellRaw::Date(DateRaw { y: 1997, m: 3, d: Some(15) }));

        let (v, diags) = parse("1997-03");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(v, CellRaw::Date(DateRaw { y: 1997, m: 3, d: None }));
    }

    #[test]
    fn phone_number_is_text_without_diagnostic() {
        // sml-spec「既知の注意点」: 氏名: 080-... のような非日付値は診断なしで Text。
        let (v, diags) = parse("080-1234-5678");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(v, CellRaw::Text("080-1234-5678".to_string()));
    }

    #[test]
    fn iso_looking_but_invalid_month_is_diagnosed() {
        let (v, diags) = parse("2026-13-01");
        assert_eq!(v, CellRaw::Text("2026-13-01".to_string()));
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadDateValue), "{diags:?}");
    }

    #[test]
    fn period_with_end_date() {
        let (v, diags) = parse("1997-03 〜 2020-10");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(
            v,
            CellRaw::Period {
                from: DateRaw { y: 1997, m: 3, d: None },
                to: Some(DateRaw { y: 2020, m: 10, d: None })
            }
        );
    }

    #[test]
    fn period_ongoing_to_present_both_separators() {
        for src in ["2020-10 〜 現在", "2020-10 ~ 現在"] {
            let (v, diags) = parse(src);
            assert!(diags.is_empty(), "{src}: {diags:?}");
            assert_eq!(v, CellRaw::Period { from: DateRaw { y: 2020, m: 10, d: None }, to: None });
        }
    }

    #[test]
    fn period_with_invalid_end_is_diagnosed() {
        let (v, diags) = parse("2020-10 〜 2026-13");
        assert_eq!(v, CellRaw::Text("2020-10 〜 2026-13".to_string()));
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadDateValue), "{diags:?}");
    }

    #[test]
    fn date_format_year_month_ja() {
        let (v, diags) = parse_with_format("1997年3月", Some(DateFormat::YearMonthJa));
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(v, CellRaw::Date(DateRaw { y: 1997, m: 3, d: None }));
    }

    #[test]
    fn date_format_year_month_day_ja() {
        let (v, diags) = parse_with_format("1997年3月15日", Some(DateFormat::YearMonthDayJa));
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(v, CellRaw::Date(DateRaw { y: 1997, m: 3, d: Some(15) }));
    }

    #[test]
    fn date_format_not_declared_leaves_ja_text_as_text_without_diag() {
        // date-format 未宣言なら「書式スニッフィングはしない」— 日本語形式は Text
        // フォールバックのまま、診断も出さない。
        let (v, diags) = parse("1997年3月");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(v, CellRaw::Text("1997年3月".to_string()));
    }

    #[test]
    fn iso_still_accepted_when_date_format_declared() {
        // date-format 宣言はあくまで「追加」書式(D29): ISO は常に既定で通る。
        let (v, diags) = parse_with_format("1997-03", Some(DateFormat::YearMonthJa));
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(v, CellRaw::Date(DateRaw { y: 1997, m: 3, d: None }));
    }

    #[test]
    fn ref_and_empty_take_priority_over_date_parsing() {
        assert_eq!(parse("ref:some-value").0, CellRaw::Ref(RefTarget::Label("some-value".to_string())));
        assert_eq!(parse("").0, CellRaw::Empty);
    }
}
