//! 層B: `::table` 本体パース(sml-parser-design.md §3、sml-spec §6.1 D4)。
//!
//! `@rows:` / `@cols:` / `@cells:` の3セクションを、行頭 `#` コメントと空行を無視しつつ
//! 読む。次元⇄メンバーの交互ネストは2スペース単位のインデントで表現される
//! (`@rows:`/`@cols:` 直下が次元、次元の下がメンバー、メンバーが子を持てば
//! さらにその下が次元…という交互構造)。セル行は `path | path : value` で、
//! 値は D4 の6種類に型付けされる。
//!
//! 座標の葉パス実在検証(次元木とセルの突き合わせ)はここではやらない。build(M3)の
//! 仕事(sml-parser-design.md §10)。
//!
//! ## 実装上の裁量(報告対象、sml-parser-m1-handoff.md 参照)
//!
//! - 数量セル値(`45 ms`)の単位トークンの字句は仕様未確定。ここでは保守的に
//!   `[A-Za-zµ%°]` で始まり `[A-Za-z0-9/^·%°-]*` が続くトークン1個とした
//!   (`is_valid_unit_token`)。
//! - `@cells` の座標 path のセグメントが `[A-Za-z0-9_-]+` に違反する場合、
//!   および `|`/`:` を欠く不正なセル行は `BadCellCoord` を発火する。一方、
//!   `@rows`/`@cols` の次元名・member key の字句違反は `BadKeyCharset` を発火する
//!   (どちらも D5 の同じ字句制約だが、セル座標由来かどうかで診断種別を分けた)。
//! - 次元行 `- name: ...` がコロンを持たない、または `[...]` でも空でもない
//!   よく分からない形の場合、専用の DiagKind が用意されていないため
//!   `InconsistentIndent`(「期待した構造になっていない」の意)に倒して報告する。
//!   これは構造上のフォールバックであり、字句(charset)の問題ではない。
//! - フラット糖衣 `- name: [k1, k2]` の各メンバーには、個々のトークンの位置ではなく
//!   次元行全体のスパンを割り当てている(fmt がまだこの粒度を必要としないため)。

use ulid::Ulid;

use crate::ast::{CellEntry, CellRaw, DimNode, MemberNode, RefTarget, TableBody};
use crate::error::{Diag, DiagKind};
use crate::scan::split_lines_range;
use crate::span::Span;

/// `::table` フェンスの本体(フェンス内属性行を除いた残り)をパースする。
pub fn parse_table_body(src: &str, body: Span, diags: &mut Vec<Diag>) -> TableBody {
    let lines = collect_lines(src, body);

    let mut rows = Vec::new();
    let mut cols = Vec::new();
    let mut cells = Vec::new();

    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if line.indent != 0 {
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line.span,
                "セクション見出し(@rows:/@cols:/@cells:)はインデント無しで書いてください",
            ));
            i += 1;
            continue;
        }
        match line.text {
            "@rows:" => {
                i += 1;
                rows = parse_dim_list(&lines, &mut i, diags, 1);
            }
            "@cols:" => {
                i += 1;
                cols = parse_dim_list(&lines, &mut i, diags, 1);
            }
            "@cells:" => {
                i += 1;
                cells = parse_cells(&lines, &mut i, diags, 1);
            }
            other => {
                diags.push(Diag::new(
                    DiagKind::InconsistentIndent,
                    line.span,
                    format!("認識できないセクション見出しです: {other:?}"),
                ));
                i += 1;
            }
        }
    }

    TableBody { rows, cols, cells }
}

// ---- 行の前処理(空行・コメント除去、インデント計算) -------------------------

/// コメント・空行を除去した後の1行。`text` は先頭のインデント(半角スペース)を
/// 取り除き、末尾の空白もトリムした中身。
struct TLine<'a> {
    /// 元の行(インデントを含む、改行を含まない)のスパン。
    span: Span,
    /// 先頭の半角スペースの個数。
    indent: usize,
    text: &'a str,
}

fn collect_lines(src: &str, body: Span) -> Vec<TLine<'_>> {
    split_lines_range(src, body)
        .into_iter()
        .filter_map(|pl| {
            let raw = pl.content.slice(src);
            let stripped = raw.trim_start_matches(' ');
            let indent = raw.len() - stripped.len();
            let text = stripped.trim_end();
            if text.is_empty() || text.starts_with('#') {
                None
            } else {
                Some(TLine { span: pl.content, indent, text })
            }
        })
        .collect()
}

/// インデント(スペース数)を「2スペース単位のレベル」に変換する。
/// 戻り値の bool は「2の倍数だったか」(false なら `InconsistentIndent` 対象)。
fn level_of(indent: usize) -> (usize, bool) {
    (indent / 2, indent.is_multiple_of(2))
}

// ---- 次元 / メンバー ---------------------------------------------------------

/// `lines[*idx]` から、インデントレベルが `level` の次元(`- name: ...`)を
/// 連続して読み取る。レベルが `level` 未満になったら(親スコープに戻ったら)止まる。
fn parse_dim_list(lines: &[TLine], idx: &mut usize, diags: &mut Vec<Diag>, level: usize) -> Vec<DimNode> {
    let mut dims = Vec::new();
    while *idx < lines.len() {
        let line = &lines[*idx];
        let (line_level, aligned) = level_of(line.indent);
        if !aligned {
            if line_level < level {
                break;
            }
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line.span,
                "インデントが2スペース単位で揃っていません",
            ));
            *idx += 1;
            continue;
        }
        if line_level < level {
            break;
        }
        if line_level > level {
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line.span,
                "想定より深いインデントです",
            ));
            *idx += 1;
            continue;
        }
        let Some(rest) = line.text.strip_prefix("- ") else { break };
        *idx += 1;
        dims.push(parse_one_dim(line, rest, lines, idx, diags, level));
    }
    dims
}

fn parse_one_dim(
    line: &TLine,
    rest: &str,
    lines: &[TLine],
    idx: &mut usize,
    diags: &mut Vec<Diag>,
    level: usize,
) -> DimNode {
    let Some(colon_pos) = rest.find(':') else {
        diags.push(Diag::new(
            DiagKind::InconsistentIndent,
            line.span,
            "次元行に ':' がありません(構文: '- 次元名: [a, b]' または '- 次元名:')",
        ));
        return DimNode { name: rest.trim().to_string(), span: line.span, members: Vec::new() };
    };

    let name = rest[..colon_pos].trim().to_string();
    if !is_valid_key_charset(&name) {
        diags.push(Diag::new(
            DiagKind::BadKeyCharset,
            line.span,
            format!("次元名 '{name}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
        ));
    }

    let after_colon = rest[colon_pos + 1..].trim();
    let members = if after_colon.is_empty() {
        parse_member_list(lines, idx, diags, level + 1)
    } else if after_colon.starts_with('[') && after_colon.ends_with(']') {
        parse_flat_members(&after_colon[1..after_colon.len() - 1], line.span, diags)
    } else {
        diags.push(Diag::new(
            DiagKind::InconsistentIndent,
            line.span,
            "次元の宣言形式が不正です(フラット糖衣 '[...]' か、コロンのみで改行しネストするか)",
        ));
        Vec::new()
    };

    DimNode { name, span: line.span, members }
}

/// `lines[*idx]` から、インデントレベルが `level` のメンバー(`- key ["label"] [:]`)を
/// 連続して読み取る。
fn parse_member_list(lines: &[TLine], idx: &mut usize, diags: &mut Vec<Diag>, level: usize) -> Vec<MemberNode> {
    let mut members = Vec::new();
    while *idx < lines.len() {
        let line = &lines[*idx];
        let (line_level, aligned) = level_of(line.indent);
        if !aligned {
            if line_level < level {
                break;
            }
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line.span,
                "インデントが2スペース単位で揃っていません",
            ));
            *idx += 1;
            continue;
        }
        if line_level < level {
            break;
        }
        if line_level > level {
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line.span,
                "想定より深いインデントです",
            ));
            *idx += 1;
            continue;
        }
        let Some(rest) = line.text.strip_prefix("- ") else { break };
        *idx += 1;
        members.push(parse_one_member(line, rest, lines, idx, diags, level));
    }
    members
}

fn parse_one_member(
    line: &TLine,
    rest: &str,
    lines: &[TLine],
    idx: &mut usize,
    diags: &mut Vec<Diag>,
    level: usize,
) -> MemberNode {
    let rest_trimmed = rest.trim_end();
    let has_children_marker = rest_trimmed.ends_with(':');
    let core = if has_children_marker {
        rest_trimmed[..rest_trimmed.len() - 1].trim_end()
    } else {
        rest_trimmed
    };

    let (key, label) = split_key_label(core.trim());
    if !is_valid_key_charset(&key) {
        diags.push(Diag::new(
            DiagKind::BadKeyCharset,
            line.span,
            format!("member key '{key}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
        ));
    }

    let children = if has_children_marker { parse_dim_list(lines, idx, diags, level + 1) } else { Vec::new() };

    MemberNode { key, label, span: line.span, children }
}

/// フラット糖衣 `[k1, k2, "label" 付き k3, ...]` の中身をメンバー列にパースする。
fn parse_flat_members(inner: &str, line_span: Span, diags: &mut Vec<Diag>) -> Vec<MemberNode> {
    inner
        .split(',')
        .map(|raw| raw.trim())
        .filter(|s| !s.is_empty())
        .map(|tok| {
            let (key, label) = split_key_label(tok);
            if !is_valid_key_charset(&key) {
                diags.push(Diag::new(
                    DiagKind::BadKeyCharset,
                    line_span,
                    format!("member key '{key}' の字句が不正です([A-Za-z0-9_-]+ のみ許可)"),
                ));
            }
            MemberNode { key, label, span: line_span, children: Vec::new() }
        })
        .collect()
}

/// `key` または `key "label"` を `(key, label)` に分解する。
/// 引用符が閉じていない等、形が崩れている場合は全体をそのまま key 側に落とす
/// (charset 検証が自然に不正字句を検出する)。
fn split_key_label(tok: &str) -> (String, Option<String>) {
    if let Some(q) = tok.find('"') {
        let key_part = tok[..q].trim();
        let label_part = &tok[q..];
        if label_part.len() >= 2 && label_part.ends_with('"') {
            return (key_part.to_string(), Some(label_part[1..label_part.len() - 1].to_string()));
        }
        return (tok.trim().to_string(), None);
    }
    (tok.trim().to_string(), None)
}

// ---- セル --------------------------------------------------------------------

fn parse_cells(lines: &[TLine], idx: &mut usize, diags: &mut Vec<Diag>, level: usize) -> Vec<CellEntry> {
    let mut cells = Vec::new();
    while *idx < lines.len() {
        let line = &lines[*idx];
        let (line_level, aligned) = level_of(line.indent);
        if !aligned {
            if line_level < level {
                break;
            }
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line.span,
                "インデントが2スペース単位で揃っていません",
            ));
            *idx += 1;
            continue;
        }
        if line_level < level {
            break;
        }
        if line_level > level {
            diags.push(Diag::new(
                DiagKind::InconsistentIndent,
                line.span,
                "想定より深いインデントです",
            ));
            *idx += 1;
            continue;
        }
        *idx += 1;
        if let Some(cell) = parse_cell_line(line, diags) {
            cells.push(cell);
        }
    }
    cells
}

/// セル行 `<行path> | <列path> : <値>` をパースする(sml-spec §7)。
fn parse_cell_line(line: &TLine, diags: &mut Vec<Diag>) -> Option<CellEntry> {
    let text = line.text;

    let Some(pipe_pos) = text.find('|') else {
        diags.push(Diag::new(
            DiagKind::BadCellCoord,
            line.span,
            "セル行に '|' がありません(構文: '行path | 列path : 値')",
        ));
        return None;
    };
    let row_part = text[..pipe_pos].trim();
    let after_pipe = &text[pipe_pos + 1..];

    let Some(colon_pos) = after_pipe.find(':') else {
        diags.push(Diag::new(
            DiagKind::BadCellCoord,
            line.span,
            "セル行に ':' がありません(構文: '行path | 列path : 値')",
        ));
        return None;
    };
    let col_part = after_pipe[..colon_pos].trim();
    let value_part = after_pipe[colon_pos + 1..].trim();

    let row_path = parse_cell_path(row_part, line.span, diags)?;
    let col_path = parse_cell_path(col_part, line.span, diags)?;
    let value = parse_cell_value(value_part);

    Some(CellEntry { row_path, col_path, value, span: line.span })
}

/// `key ("." key)*` の座標パスをパースする(sml-spec §7)。
fn parse_cell_path(s: &str, line_span: Span, diags: &mut Vec<Diag>) -> Option<Vec<String>> {
    if s.is_empty() {
        diags.push(Diag::new(DiagKind::BadCellCoord, line_span, "座標が空です"));
        return None;
    }
    let parts: Vec<&str> = s.split('.').map(|p| p.trim()).collect();
    if parts.iter().any(|p| !is_valid_key_charset(p)) {
        diags.push(Diag::new(
            DiagKind::BadCellCoord,
            line_span,
            format!("座標 '{s}' の字句が不正です([A-Za-z0-9_-]+ を '.' で連結した形式のみ許可)"),
        ));
        return None;
    }
    Some(parts.into_iter().map(|p| p.to_string()).collect())
}

/// セル値の型付きパース(D4、sml-spec §6.1 の表)。
fn parse_cell_value(raw: &str) -> CellRaw {
    let s = raw.trim();
    if s.is_empty() || s == "~" {
        return CellRaw::Empty;
    }
    if s.len() >= 2 && s.starts_with('"') && s.ends_with('"') {
        return CellRaw::Text(s[1..s.len() - 1].to_string());
    }
    if let Some(target) = s.strip_prefix("ref:") {
        return CellRaw::Ref(parse_ref_target(target.trim()));
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

    // 裸の非数値テキストは寛容にフォールバック(D4)。
    CellRaw::Text(s.to_string())
}

/// 単位トークンの字句(裁量、モジュール冒頭のドキュメント参照)。
/// `[A-Za-zµ%°]` で始まり、`[A-Za-z0-9/^·%°-]*` が続く。
fn is_valid_unit_token(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || matches!(c, 'µ' | '%' | '°') => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '^' | '·' | '%' | '°' | '-'))
}

fn is_valid_key_charset(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn parse_ref_target(token: &str) -> RefTarget {
    match token.parse::<Ulid>() {
        Ok(u) => RefTarget::Ulid(u),
        Err(_) => RefTarget::Label(token.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(src: &str) -> (TableBody, Vec<Diag>) {
        let mut diags = Vec::new();
        let table = parse_table_body(src, Span::new(0, src.len()), &mut diags);
        (table, diags)
    }

    #[test]
    fn empty_body_yields_empty_table() {
        let (t, diags) = body("");
        assert!(t.rows.is_empty());
        assert!(t.cols.is_empty());
        assert!(t.cells.is_empty());
        assert!(diags.is_empty());
    }

    #[test]
    fn flat_sugar_dimension() {
        let (t, diags) = body("@rows:\n  - model: [Baseline-v1, Opt-v2]\n");
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(t.rows.len(), 1);
        assert_eq!(t.rows[0].name, "model");
        let keys: Vec<_> = t.rows[0].members.iter().map(|m| m.key.as_str()).collect();
        assert_eq!(keys, vec!["Baseline-v1", "Opt-v2"]);
    }

    #[test]
    fn nested_dimension() {
        let src = "@cols:\n  - dataset:\n    - Dataset-A:\n      - metric: [F1-Score, Latency]\n";
        let (t, diags) = body(src);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(t.cols.len(), 1);
        assert_eq!(t.cols[0].name, "dataset");
        assert_eq!(t.cols[0].members.len(), 1);
        let member = &t.cols[0].members[0];
        assert_eq!(member.key, "Dataset-A");
        assert_eq!(member.children.len(), 1);
        assert_eq!(member.children[0].name, "metric");
        let leaf_keys: Vec<_> = member.children[0].members.iter().map(|m| m.key.as_str()).collect();
        assert_eq!(leaf_keys, vec!["F1-Score", "Latency"]);
    }

    #[test]
    fn member_label() {
        let src = "@rows:\n  - quarter:\n    - q1 \"第1四半期\"\n";
        let (t, diags) = body(src);
        assert!(diags.is_empty(), "{diags:?}");
        let member = &t.rows[0].members[0];
        assert_eq!(member.key, "q1");
        assert_eq!(member.label.as_deref(), Some("第1四半期"));
        assert!(member.children.is_empty());
    }

    #[test]
    fn member_label_with_children() {
        let src = "@cols:\n  - dataset:\n    - a \"データセットA\":\n      - metric: [x]\n";
        let (t, diags) = body(src);
        assert!(diags.is_empty(), "{diags:?}");
        let member = &t.cols[0].members[0];
        assert_eq!(member.key, "a");
        assert_eq!(member.label.as_deref(), Some("データセットA"));
        assert_eq!(member.children.len(), 1);
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let src = "# comment\n@rows:\n  # another comment\n  - model: [a, b]\n\n@cells:\n  # cell comment\n  a | model.x : 1\n";
        let (t, diags) = body(src);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(t.rows.len(), 1);
        assert_eq!(t.rows[0].members.len(), 2);
        assert_eq!(t.cells.len(), 1);
        assert_eq!(t.cells[0].row_path, vec!["a".to_string()]);
        assert_eq!(t.cells[0].col_path, vec!["model".to_string(), "x".to_string()]);
    }

    #[test]
    fn cell_value_six_types() {
        assert_eq!(parse_cell_value("0.82"), CellRaw::Number(0.82));
        assert_eq!(parse_cell_value("-3"), CellRaw::Number(-3.0));
        assert_eq!(parse_cell_value("1e5"), CellRaw::Number(1e5));
        assert_eq!(parse_cell_value("45 ms"), CellRaw::Quantity { v: 45.0, unit: "ms".to_string() });
        assert_eq!(parse_cell_value("\"任意の テキスト\""), CellRaw::Text("任意の テキスト".to_string()));
        assert_eq!(parse_cell_value("some text"), CellRaw::Text("some text".to_string()));
        assert_eq!(parse_cell_value("~"), CellRaw::Empty);
        assert_eq!(parse_cell_value(""), CellRaw::Empty);
        assert_eq!(parse_cell_value("ref:eval-table"), CellRaw::Ref(RefTarget::Label("eval-table".to_string())));
    }

    #[test]
    fn bad_cell_coord_missing_pipe() {
        let (_, diags) = body("@cells:\n  no-pipe-here : 1\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadCellCoord), "{diags:?}");
    }

    #[test]
    fn bad_cell_coord_missing_colon() {
        let (_, diags) = body("@cells:\n  a | b\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadCellCoord), "{diags:?}");
    }

    #[test]
    fn bad_cell_coord_invalid_segment() {
        let (_, diags) = body("@cells:\n  a.b c | d : 1\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadCellCoord), "{diags:?}");
    }

    #[test]
    fn inconsistent_indent_odd_spaces() {
        let (_, diags) = body("@rows:\n   - model: [a]\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::InconsistentIndent), "{diags:?}");
    }

    #[test]
    fn inconsistent_indent_unexpected_jump() {
        let src = "@rows:\n      - model: [a]\n";
        let (_, diags) = body(src);
        assert!(diags.iter().any(|d| d.kind == DiagKind::InconsistentIndent), "{diags:?}");
    }

    #[test]
    fn bad_key_charset_on_dim_name() {
        let (_, diags) = body("@rows:\n  - bad name: [a]\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{diags:?}");
    }

    #[test]
    fn bad_key_charset_on_member_key() {
        let (_, diags) = body("@rows:\n  - model: [bad key]\n");
        assert!(diags.iter().any(|d| d.kind == DiagKind::BadKeyCharset), "{diags:?}");
    }

    #[test]
    fn cells_multiple_rows_parsed() {
        let src = "@cells:\n  Baseline-v1 | Dataset-A.F1-Score : 0.82\n  Opt-v2      | Dataset-A.Latency  : 12 ms\n";
        let (t, diags) = body(src);
        assert!(diags.is_empty(), "{diags:?}");
        assert_eq!(t.cells.len(), 2);
        assert_eq!(t.cells[0].row_path, vec!["Baseline-v1".to_string()]);
        assert_eq!(t.cells[0].col_path, vec!["Dataset-A".to_string(), "F1-Score".to_string()]);
        assert_eq!(t.cells[0].value, CellRaw::Number(0.82));
        assert_eq!(t.cells[1].value, CellRaw::Quantity { v: 12.0, unit: "ms".to_string() });
    }
}
