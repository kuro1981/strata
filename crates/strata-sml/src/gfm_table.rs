//! GFM パイプ表(M6 D40 Tier2)の本体パース。
//!
//! `::table`(sml-spec §6.1、多次元)とは別物: フラット2次元の単純表を素早く書ける
//! よう、ヘッダ行 + 区切り行(`---|---`)+ データ行の3部構成をそのまま読む。
//! セルの型付きパース(数量・日付等)は呼び出し側(strata-build)が `value.rs` の
//! `parse_typed_value` を通す(`::table`/`::record` と共通)。

use crate::ast::{CellRaw, GfmTableBody};
use crate::error::Diag;
use crate::span::Span;

/// ヘッダ行 + データ行群から `GfmTableBody` を組み立てる。列数はヘッダ基準
/// (データ行の列が足りなければ `CellRaw::Empty` で埋め、多ければ切り捨てる —
/// CommonMark の寛容な扱いに倣う)。セル値は `::table`/`::record` と同じ型付き
/// パース(D4)を通す。
pub(crate) fn parse_gfm_table_body(src: &str, header_span: Span, row_spans: &[Span], diags: &mut Vec<Diag>) -> GfmTableBody {
    let header = split_row_cells(src, header_span);
    let ncols = header.len();
    let rows = row_spans
        .iter()
        .map(|rs| {
            let cells = split_row_cells(src, *rs);
            (0..ncols)
                .map(|i| match cells.get(i) {
                    Some(span) if !span.is_empty() => crate::value::parse_typed_value(span.slice(src), None, *span, diags),
                    _ => CellRaw::Empty,
                })
                .collect()
        })
        .collect();
    GfmTableBody { header, rows }
}

/// `| a | b |` 形式の1行をセルスパンへ分割する。先頭/末尾の `|` は無視し、
/// `\|`(エスケープされたパイプ)はセル区切りとして扱わない。各セルは前後の
/// 空白を除いたスパンにする。
fn split_row_cells(src: &str, line_span: Span) -> Vec<Span> {
    let bytes = src.as_bytes();
    let mut start = line_span.start;
    let mut end = line_span.end;
    while start < end && bytes[start] == b' ' {
        start += 1;
    }
    while end > start && bytes[end - 1] == b' ' {
        end -= 1;
    }
    if start < end && bytes[start] == b'|' {
        start += 1;
    }
    if end > start && bytes[end - 1] == b'|' {
        end -= 1;
    }

    let mut cells = Vec::new();
    let mut cell_start = start;
    let mut i = start;
    while i < end {
        if bytes[i] == b'\\' && i + 1 < end {
            i += 2;
            continue;
        }
        if bytes[i] == b'|' {
            cells.push(trim_span(src, Span::new(cell_start, i)));
            i += 1;
            cell_start = i;
            continue;
        }
        i += 1;
    }
    cells.push(trim_span(src, Span::new(cell_start, end)));
    cells
}

fn trim_span(src: &str, span: Span) -> Span {
    let t = span.slice(src);
    let lead = t.len() - t.trim_start().len();
    let trimmed_len = t.trim().len();
    Span::new(span.start + lead, span.start + lead + trimmed_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_row() {
        let src = "| A | B |";
        let cells = split_row_cells(src, Span::new(0, src.len()));
        assert_eq!(cells.len(), 2);
        assert_eq!(cells[0].slice(src), "A");
        assert_eq!(cells[1].slice(src), "B");
    }

    #[test]
    fn pads_missing_and_truncates_extra_cells() {
        let src = "| A | B |\n| x |\n| y | z | w |";
        let header_span = Span::new(0, 9);
        let row1 = Span::new(10, 15);
        let row2 = Span::new(16, 29);
        let mut diags = Vec::new();
        let body = parse_gfm_table_body(src, header_span, &[row1, row2], &mut diags);
        assert_eq!(body.header.len(), 2);
        assert_eq!(body.rows[0].len(), 2);
        assert_eq!(body.rows[0][1], crate::ast::CellRaw::Empty);
        assert_eq!(body.rows[1].len(), 2);
    }
}
