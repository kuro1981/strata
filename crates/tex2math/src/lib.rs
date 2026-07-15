//! tex2math — TeX 数式 → MathNode(MathML Presentation サブセット)
//!
//! Strata §6: 人は TeX で書き、ここで canonical の MathNode 木へロスレス変換する。
//! 手法は Pratt パース(演算子優先順位法)。`_` `^` `\frac` `\sqrt` `\sum` 等を扱う。
//! 足りない制御綴り/演算子が出たら、その都度ここに足す(§6 の運用)。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum MathNode {
    Num { v: String },
    Ident { v: String },
    Op { v: String },
    Row { items: Vec<MathNode> },
    Frac { num: Box<MathNode>, den: Box<MathNode> },
    Sup { base: Box<MathNode>, sup: Box<MathNode> },
    Sub { base: Box<MathNode>, sub: Box<MathNode> },
    SubSup { base: Box<MathNode>, sub: Box<MathNode>, sup: Box<MathNode> },
    UnderOver {
        base: Box<MathNode>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        under: Option<Box<MathNode>>,
        #[serde(skip_serializing_if = "Option::is_none", default)]
        over: Option<Box<MathNode>>,
    },
    Sqrt { body: Box<MathNode> },
    Root { radicand: Box<MathNode>, index: Box<MathNode> },
    Fenced { open: String, close: String, body: Box<MathNode> },
    Text { s: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Unexpected { pos: usize, found: char },
    UnexpectedEnd { context: &'static str },
    Unbalanced { pos: usize },
    UnknownCommand { name: String },
    ExpectedGroup { context: &'static str, pos: usize },
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Digit(char),
    Letter(char),
    Op(char),
    Caret,
    Underscore,
    LBrace,
    RBrace,
    Command(String),
}

struct Lexer {
    chars: Vec<(usize, char)>,
    i: usize,
}

impl Lexer {
    fn new(s: &str) -> Self {
        Lexer { chars: s.char_indices().collect(), i: 0 }
    }

    fn tokenize(mut self) -> Result<Vec<(usize, Tok)>, ParseError> {
        let mut out = Vec::new();
        while self.i < self.chars.len() {
            let (pos, c) = self.chars[self.i];
            match c {
                ' ' | '\t' | '\n' | '\r' => self.i += 1,
                '{' => { out.push((pos, Tok::LBrace)); self.i += 1; }
                '}' => { out.push((pos, Tok::RBrace)); self.i += 1; }
                '^' => { out.push((pos, Tok::Caret)); self.i += 1; }
                '_' => { out.push((pos, Tok::Underscore)); self.i += 1; }
                '\\' => {
                    self.i += 1;
                    if self.i >= self.chars.len() {
                        return Err(ParseError::UnexpectedEnd { context: "command" });
                    }
                    let (_, first) = self.chars[self.i];
                    if first.is_ascii_alphabetic() {
                        let mut name = String::new();
                        while self.i < self.chars.len() {
                            let (_, cc) = self.chars[self.i];
                            if cc.is_ascii_alphabetic() {
                                name.push(cc);
                                self.i += 1;
                            } else {
                                break;
                            }
                        }
                        out.push((pos, Tok::Command(name)));
                    } else {
                        out.push((pos, Tok::Command(first.to_string())));
                        self.i += 1;
                    }
                }
                '0'..='9' | '.' => { out.push((pos, Tok::Digit(c))); self.i += 1; }
                c if c.is_ascii_alphabetic() => { out.push((pos, Tok::Letter(c))); self.i += 1; }
                _ => { out.push((pos, Tok::Op(c))); self.i += 1; }
            }
        }
        Ok(out)
    }
}

pub struct Parser {
    toks: Vec<(usize, Tok)>,
    i: usize,
}

pub fn parse(tex: &str) -> Result<MathNode, ParseError> {
    let toks = Lexer::new(tex).tokenize()?;
    let mut p = Parser { toks, i: 0 };
    let node = p.parse_row(&[])?;
    if p.i != p.toks.len() {
        let (pos, _) = p.toks[p.i].clone();
        return Err(ParseError::Unbalanced { pos });
    }
    Ok(node)
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i).map(|(_, t)| t)
    }
    fn pos(&self) -> usize {
        self.toks.get(self.i).map(|(p, _)| *p).unwrap_or(usize::MAX)
    }
    fn bump(&mut self) -> Option<(usize, Tok)> {
        let t = self.toks.get(self.i).cloned();
        if t.is_some() {
            self.i += 1;
        }
        t
    }

    fn parse_row(&mut self, stop: &[Tok]) -> Result<MathNode, ParseError> {
        let mut items: Vec<MathNode> = Vec::new();
        while let Some(t) = self.peek() {
            if stop.contains(t) || matches!(t, Tok::RBrace) {
                break;
            }
            items.push(self.parse_postfix()?);
        }
        Ok(match items.len() {
            1 => items.pop().unwrap(),
            _ => MathNode::Row { items },
        })
    }

    fn parse_postfix(&mut self) -> Result<MathNode, ParseError> {
        let mut base = self.parse_atom()?;
        let mut sub: Option<MathNode> = None;
        let mut sup: Option<MathNode> = None;
        loop {
            match self.peek() {
                Some(Tok::Underscore) => {
                    self.bump();
                    sub = Some(self.parse_script_arg("subscript")?);
                }
                Some(Tok::Caret) => {
                    self.bump();
                    sup = Some(self.parse_script_arg("superscript")?);
                }
                _ => break,
            }
        }
        base = match (sub, sup) {
            (None, None) => base,
            (Some(sb), None) => MathNode::Sub { base: Box::new(base), sub: Box::new(sb) },
            (None, Some(sp)) => MathNode::Sup { base: Box::new(base), sup: Box::new(sp) },
            (Some(sb), Some(sp)) => MathNode::SubSup {
                base: Box::new(base),
                sub: Box::new(sb),
                sup: Box::new(sp),
            },
        };
        Ok(base)
    }

    fn parse_script_arg(&mut self, ctx: &'static str) -> Result<MathNode, ParseError> {
        match self.peek() {
            Some(Tok::LBrace) => self.parse_group(ctx),
            Some(_) => self.parse_atom(),
            None => Err(ParseError::UnexpectedEnd { context: ctx }),
        }
    }

    fn parse_group(&mut self, ctx: &'static str) -> Result<MathNode, ParseError> {
        match self.peek() {
            Some(Tok::LBrace) => {
                self.bump();
                let inner = self.parse_row(&[Tok::RBrace])?;
                match self.bump() {
                    Some((_, Tok::RBrace)) => Ok(inner),
                    _ => Err(ParseError::UnexpectedEnd { context: ctx }),
                }
            }
            _ => Err(ParseError::ExpectedGroup { context: ctx, pos: self.pos() }),
        }
    }

    fn parse_atom(&mut self) -> Result<MathNode, ParseError> {
        match self.peek().cloned() {
            Some(Tok::Digit(_)) => {
                let mut s = String::new();
                while let Some(Tok::Digit(c)) = self.peek() {
                    s.push(*c);
                    self.bump();
                }
                Ok(MathNode::Num { v: s })
            }
            Some(Tok::Letter(c)) => {
                self.bump();
                Ok(MathNode::Ident { v: c.to_string() })
            }
            Some(Tok::Op(c)) => {
                self.bump();
                Ok(MathNode::Op { v: c.to_string() })
            }
            Some(Tok::LBrace) => self.parse_group("group"),
            Some(Tok::Command(name)) => {
                self.bump();
                self.parse_command(&name)
            }
            Some(Tok::Underscore) | Some(Tok::Caret) => {
                Err(ParseError::Unexpected { pos: self.pos(), found: '^' })
            }
            Some(Tok::RBrace) => Err(ParseError::Unbalanced { pos: self.pos() }),
            None => Err(ParseError::UnexpectedEnd { context: "atom" }),
        }
    }

    fn parse_command(&mut self, name: &str) -> Result<MathNode, ParseError> {
        match name {
            "frac" => {
                let num = self.parse_group("\\frac numerator")?;
                let den = self.parse_group("\\frac denominator")?;
                Ok(MathNode::Frac { num: Box::new(num), den: Box::new(den) })
            }
            "sqrt" => {
                if matches!(self.peek(), Some(Tok::Op('['))) {
                    self.bump();
                    let index = self.parse_row(&[Tok::Op(']')])?;
                    match self.bump() {
                        Some((_, Tok::Op(']'))) => {}
                        _ => return Err(ParseError::UnexpectedEnd { context: "\\sqrt index" }),
                    }
                    let radicand = self.parse_group("\\sqrt radicand")?;
                    Ok(MathNode::Root { radicand: Box::new(radicand), index: Box::new(index) })
                } else {
                    let body = self.parse_group("\\sqrt body")?;
                    Ok(MathNode::Sqrt { body: Box::new(body) })
                }
            }
            "sum" => Ok(MathNode::Op { v: "∑".into() }),
            "prod" => Ok(MathNode::Op { v: "∏".into() }),
            "int" => Ok(MathNode::Op { v: "∫".into() }),
            "lim" => Ok(MathNode::Op { v: "lim".into() }),
            "text" => {
                let inner = self.collect_text("\\text")?;
                Ok(MathNode::Text { s: inner })
            }
            "left" => self.parse_left_right(),
            "hat" => {
                let base = self.parse_group("\\hat body")?;
                Ok(MathNode::UnderOver {
                    base: Box::new(base),
                    under: None,
                    over: Some(Box::new(MathNode::Op { v: "^".into() })),
                })
            }
            other => {
                if let Some(sym) = greek(other) {
                    Ok(MathNode::Ident { v: sym.to_string() })
                } else if let Some(sym) = named_op(other) {
                    Ok(MathNode::Op { v: sym.to_string() })
                } else {
                    Err(ParseError::UnknownCommand { name: other.to_string() })
                }
            }
        }
    }

    fn collect_text(&mut self, ctx: &'static str) -> Result<String, ParseError> {
        match self.bump() {
            Some((_, Tok::LBrace)) => {}
            _ => return Err(ParseError::ExpectedGroup { context: ctx, pos: self.pos() }),
        }
        let mut s = String::new();
        loop {
            match self.bump() {
                Some((_, Tok::RBrace)) => break,
                Some((_, tok)) => s.push(tok_to_char(&tok)),
                None => return Err(ParseError::UnexpectedEnd { context: ctx }),
            }
        }
        Ok(s)
    }

    fn parse_left_right(&mut self) -> Result<MathNode, ParseError> {
        let open = match self.bump() {
            Some((_, Tok::Op(c))) => c.to_string(),
            Some((_, Tok::Command(c))) => c,
            _ => return Err(ParseError::UnexpectedEnd { context: "\\left delimiter" }),
        };
        let body = self.parse_until_right()?;
        let close = match self.bump() {
            Some((_, Tok::Op(c))) => c.to_string(),
            Some((_, Tok::Command(c))) => c,
            _ => return Err(ParseError::UnexpectedEnd { context: "\\right delimiter" }),
        };
        Ok(MathNode::Fenced { open, close, body: Box::new(body) })
    }

    fn parse_until_right(&mut self) -> Result<MathNode, ParseError> {
        let mut items = Vec::new();
        loop {
            match self.peek() {
                Some(Tok::Command(c)) if c == "right" => {
                    self.bump();
                    break;
                }
                None => return Err(ParseError::UnexpectedEnd { context: "\\right" }),
                _ => items.push(self.parse_postfix()?),
            }
        }
        Ok(match items.len() {
            1 => items.pop().unwrap(),
            _ => MathNode::Row { items },
        })
    }
}

fn tok_to_char(t: &Tok) -> char {
    match t {
        Tok::Digit(c) | Tok::Letter(c) | Tok::Op(c) => *c,
        Tok::Caret => '^',
        Tok::Underscore => '_',
        Tok::LBrace => '{',
        Tok::RBrace => '}',
        Tok::Command(s) => s.chars().next().unwrap_or('?'),
    }
}

fn greek(name: &str) -> Option<&'static str> {
    Some(match name {
        "alpha" => "α", "beta" => "β", "gamma" => "γ", "delta" => "δ",
        "epsilon" => "ε", "theta" => "θ", "lambda" => "λ", "mu" => "μ",
        "pi" => "π", "sigma" => "σ", "phi" => "φ", "omega" => "ω",
        "Gamma" => "Γ", "Delta" => "Δ", "Theta" => "Θ", "Lambda" => "Λ",
        "Sigma" => "Σ", "Phi" => "Φ", "Omega" => "Ω",
        _ => return None,
    })
}

fn named_op(name: &str) -> Option<&'static str> {
    Some(match name {
        "times" => "×", "cdot" => "⋅", "div" => "÷", "pm" => "±",
        "leq" | "le" => "≤", "geq" | "ge" => "≥", "neq" | "ne" => "≠",
        "approx" => "≈", "to" => "→", "infty" => "∞",
        "partial" => "∂", "nabla" => "∇", "in" => "∈",
        _ => return None,
    })
}

/// 大型演算子(∑ ∏ ∫)に付いた Sub/Sup を UnderOver(munderover)に正規化する。
pub fn normalize(node: MathNode) -> MathNode {
    fn is_big(n: &MathNode) -> bool {
        matches!(n, MathNode::Op { v } if v == "∑" || v == "∏" || v == "∫")
    }
    match node {
        MathNode::Row { items } => MathNode::Row {
            items: items.into_iter().map(normalize).collect(),
        },
        MathNode::SubSup { base, sub, sup } if is_big(&base) => MathNode::UnderOver {
            base: Box::new(normalize(*base)),
            under: Some(Box::new(normalize(*sub))),
            over: Some(Box::new(normalize(*sup))),
        },
        MathNode::Sub { base, sub } if is_big(&base) => MathNode::UnderOver {
            base: Box::new(normalize(*base)),
            under: Some(Box::new(normalize(*sub))),
            over: None,
        },
        MathNode::Frac { num, den } => MathNode::Frac {
            num: Box::new(normalize(*num)),
            den: Box::new(normalize(*den)),
        },
        MathNode::Sup { base, sup } => MathNode::Sup {
            base: Box::new(normalize(*base)),
            sup: Box::new(normalize(*sup)),
        },
        MathNode::Sub { base, sub } => MathNode::Sub {
            base: Box::new(normalize(*base)),
            sub: Box::new(normalize(*sub)),
        },
        MathNode::SubSup { base, sub, sup } => MathNode::SubSup {
            base: Box::new(normalize(*base)),
            sub: Box::new(normalize(*sub)),
            sup: Box::new(normalize(*sup)),
        },
        MathNode::Sqrt { body } => MathNode::Sqrt { body: Box::new(normalize(*body)) },
        MathNode::Root { radicand, index } => MathNode::Root {
            radicand: Box::new(normalize(*radicand)),
            index: Box::new(normalize(*index)),
        },
        MathNode::Fenced { open, close, body } => MathNode::Fenced {
            open,
            close,
            body: Box::new(normalize(*body)),
        },
        MathNode::UnderOver { base, under, over } => MathNode::UnderOver {
            base: Box::new(normalize(*base)),
            under: under.map(|b| Box::new(normalize(*b))),
            over: over.map(|b| Box::new(normalize(*b))),
        },
        leaf => leaf,
    }
}

pub fn parse_normalized(tex: &str) -> Result<MathNode, ParseError> {
    parse(tex).map(normalize)
}

// --- WP-M1(sml-spec §1.8 D38): MathNode → TeX 逆直列化 -----------------------------
//
// `render --format md` の数式出力(`$...$`/`$$...$$`)のために MathNode 木を TeX
// テキストへ書き戻す。パーサの厳密な逆関数である必要はない(文字列の完全復元は
// 要求しない、sml-spec §1.8 WP-M1)。要求されるのは「TeX として正しく再パースでき、
// 元の MathNode と構造同値になる」こと(round-trip テストで固定する)。
//
// 括弧の方針(裁量): `Sub`/`Sup`/`SubSup`/`UnderOver` の `base` は常に `{...}` で
// くるむ。TeX のグルーピングは「透過」(`parse_group` は中身をそのまま返す。複数
// トークンの場合のみ `Row` になる)なので、余分な `{}` を足しても再パース結果は
// 変わらない。これにより「base が Row(複数トークンの積等)のときだけ波括弧が要る」
// という条件分岐を避け、実装を単純化した。
pub fn to_tex(node: &MathNode) -> String {
    match node {
        MathNode::Num { v } => v.clone(),
        MathNode::Ident { v } => ident_to_tex(v),
        MathNode::Op { v } => op_to_tex(v),
        MathNode::Row { items } => items.iter().map(to_tex).collect::<Vec<_>>().join(" "),
        MathNode::Frac { num, den } => format!("\\frac{{{}}}{{{}}}", to_tex(num), to_tex(den)),
        MathNode::Sup { base, sup } => format!("{}^{{{}}}", to_tex_base(base), to_tex(sup)),
        MathNode::Sub { base, sub } => format!("{}_{{{}}}", to_tex_base(base), to_tex(sub)),
        MathNode::SubSup { base, sub, sup } => {
            format!("{}_{{{}}}^{{{}}}", to_tex_base(base), to_tex(sub), to_tex(sup))
        }
        MathNode::UnderOver { base, under, over } => to_tex_underover(base, under.as_deref(), over.as_deref()),
        MathNode::Sqrt { body } => format!("\\sqrt{{{}}}", to_tex(body)),
        MathNode::Root { radicand, index } => format!("\\sqrt[{}]{{{}}}", to_tex(index), to_tex(radicand)),
        MathNode::Fenced { open, close, body } => {
            format!("\\left{} {} \\right{}", delim_to_tex(open), to_tex(body), delim_to_tex(close))
        }
        MathNode::Text { s } => format!("\\text{{{}}}", s),
    }
}

/// `\hat{x}` は専用の `UnderOver{ under: None, over: Some(Op("^")) }` 形で表現される
/// (`parse_command` の "hat" アーム)。この特殊形は `\hat{...}` として書き戻す。それ
/// 以外の一般形(`normalize` が大型演算子の Sub/Sup をたたみ込んだもの等)は
/// `base_{under}^{over}` として書き戻す(再パースは Sub/Sup/SubSup になり、
/// UnderOver への畳み込みは `normalize` 側の仕事なので、素の `parse` との構造同値は
/// 崩れない — round-trip テストは `parse_normalized` 側で確認する)。
fn to_tex_underover(base: &MathNode, under: Option<&MathNode>, over: Option<&MathNode>) -> String {
    if under.is_none() && matches!(over, Some(MathNode::Op { v }) if v == "^") {
        return format!("\\hat{{{}}}", to_tex(base));
    }
    let mut out = to_tex_base(base);
    if let Some(u) = under {
        out.push_str(&format!("_{{{}}}", to_tex(u)));
    }
    if let Some(o) = over {
        out.push_str(&format!("^{{{}}}", to_tex(o)));
    }
    out
}

/// `Sub`/`Sup`/`SubSup`/`UnderOver` の `base` の書き戻し。`Num`/`Ident`/`Op` は
/// それ自体が1トークンなので波括弧無しでそのまま出す(`y_i` のように素直な見た目に
/// なる。`render --format md` は人間向けビューなので読みやすさを優先する裁量)。
/// それ以外(`Row`/`Frac`/`Sqrt` 等の複合式)は常に `{...}` でくるむ — TeX の
/// グルーピングは透過(`parse_group` は中身をそのまま返す)なので、この判定を
/// 誤って複合式を裸で出しても構造は壊れない(安全側に倒す簡易ヒューリスティック)。
fn to_tex_base(base: &MathNode) -> String {
    match base {
        MathNode::Num { .. } | MathNode::Ident { .. } | MathNode::Op { .. } => to_tex(base),
        _ => format!("{{{}}}", to_tex(base)),
    }
}

/// `\left`/`\right` の区切り文字。1文字の記号(`(` 等)はそのまま、複数文字の
/// コマンド名(`langle` 等、`parse_left_right` が `Tok::Command` から先頭 `\` を
/// 落として保持したもの)は `\` を付け直す。
fn delim_to_tex(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic()) {
        format!("\\{s}")
    } else {
        s.to_string()
    }
}

/// `Ident` の書き戻し。1文字の ASCII 文字はそのまま、ギリシャ文字記号は
/// `greek()` の逆引きで `\alpha` 等に戻す。
fn ident_to_tex(v: &str) -> String {
    if let Some(name) = reverse_greek(v) {
        format!("\\{name} ")
    } else {
        v.to_string()
    }
}

/// `Op` の書き戻し。大型演算子(∑∏∫)・named_op の記号・`lim` はコマンド名へ逆変換、
/// それ以外(素の ASCII 記号 `+` `=` `(` 等)はそのまま出す。
fn op_to_tex(v: &str) -> String {
    match v {
        "∑" => "\\sum ".to_string(),
        "∏" => "\\prod ".to_string(),
        "∫" => "\\int ".to_string(),
        "lim" => "\\lim ".to_string(),
        _ => {
            if let Some(name) = reverse_named_op(v) {
                format!("\\{name} ")
            } else {
                v.to_string()
            }
        }
    }
}

fn reverse_greek(sym: &str) -> Option<&'static str> {
    Some(match sym {
        "α" => "alpha", "β" => "beta", "γ" => "gamma", "δ" => "delta",
        "ε" => "epsilon", "θ" => "theta", "λ" => "lambda", "μ" => "mu",
        "π" => "pi", "σ" => "sigma", "φ" => "phi", "ω" => "omega",
        "Γ" => "Gamma", "Δ" => "Delta", "Θ" => "Theta", "Λ" => "Lambda",
        "Σ" => "Sigma", "Φ" => "Phi", "Ω" => "Omega",
        _ => return None,
    })
}

/// `named_op()` の逆引き。複数コマンドが同じ記号にマップされる場合(`leq`/`le` 等)は
/// 短い正式名を代表として選ぶ(round-trip の構造同値には影響しない — どちらで
/// 書き戻しても再パース結果の `Op { v: "≤" }` は同じ)。
fn reverse_named_op(sym: &str) -> Option<&'static str> {
    Some(match sym {
        "×" => "times", "⋅" => "cdot", "÷" => "div", "±" => "pm",
        "≤" => "leq", "≥" => "geq", "≠" => "neq",
        "≈" => "approx", "→" => "to", "∞" => "infty",
        "∂" => "partial", "∇" => "nabla", "∈" => "in",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ident(c: &str) -> MathNode { MathNode::Ident { v: c.into() } }
    fn num(n: &str) -> MathNode { MathNode::Num { v: n.into() } }
    fn op(o: &str) -> MathNode { MathNode::Op { v: o.into() } }

    #[test]
    fn single_ident() {
        assert_eq!(parse("x").unwrap(), ident("x"));
    }

    #[test]
    fn multidigit_number() {
        assert_eq!(parse("314").unwrap(), num("314"));
    }

    #[test]
    fn superscript() {
        assert_eq!(
            parse("x^2").unwrap(),
            MathNode::Sup { base: Box::new(ident("x")), sup: Box::new(num("2")) }
        );
    }

    #[test]
    fn subscript_group() {
        assert_eq!(
            parse("x_{ij}").unwrap(),
            MathNode::Sub {
                base: Box::new(ident("x")),
                sub: Box::new(MathNode::Row { items: vec![ident("i"), ident("j")] }),
            }
        );
    }

    #[test]
    fn fraction() {
        assert_eq!(
            parse(r"\frac{a}{b}").unwrap(),
            MathNode::Frac { num: Box::new(ident("a")), den: Box::new(ident("b")) }
        );
    }

    #[test]
    fn nested_fraction() {
        let got = parse(r"\frac{1}{\frac{a}{b}}").unwrap();
        assert_eq!(
            got,
            MathNode::Frac {
                num: Box::new(num("1")),
                den: Box::new(MathNode::Frac {
                    num: Box::new(ident("a")),
                    den: Box::new(ident("b")),
                }),
            }
        );
    }

    #[test]
    fn sqrt_and_root() {
        assert_eq!(parse(r"\sqrt{2}").unwrap(), MathNode::Sqrt { body: Box::new(num("2")) });
        assert_eq!(
            parse(r"\sqrt[3]{x}").unwrap(),
            MathNode::Root { radicand: Box::new(ident("x")), index: Box::new(num("3")) }
        );
    }

    #[test]
    fn sum_normalizes_to_underover() {
        let got = parse_normalized(r"\sum_{i=1}^{n} x_i").unwrap();
        let expected = MathNode::Row {
            items: vec![
                MathNode::UnderOver {
                    base: Box::new(op("∑")),
                    under: Some(Box::new(MathNode::Row {
                        items: vec![ident("i"), op("="), num("1")],
                    })),
                    over: Some(Box::new(ident("n"))),
                },
                MathNode::Sub { base: Box::new(ident("x")), sub: Box::new(ident("i")) },
            ],
        };
        assert_eq!(got, expected);
    }

    #[test]
    fn greek_and_named_ops() {
        assert_eq!(parse(r"\alpha").unwrap(), ident("α"));
        assert_eq!(parse(r"\times").unwrap(), op("×"));
        assert_eq!(
            parse(r"a \times b").unwrap(),
            MathNode::Row { items: vec![ident("a"), op("×"), ident("b")] }
        );
    }

    #[test]
    fn quadratic_formula() {
        let got = parse_normalized(r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}").unwrap();
        match got {
            MathNode::Row { items } => {
                assert_eq!(items[0], ident("x"));
                assert_eq!(items[1], op("="));
                assert!(matches!(items[2], MathNode::Frac { .. }));
            }
            _ => panic!("expected row"),
        }
    }

    #[test]
    fn json_roundtrips() {
        let n = parse_normalized(r"\frac{a}{b}").unwrap();
        let j = serde_json::to_string(&n).unwrap();
        let back: MathNode = serde_json::from_str(&j).unwrap();
        assert_eq!(n, back);
    }

    #[test]
    fn unknown_command_is_reported() {
        match parse(r"\foobar") {
            Err(ParseError::UnknownCommand { name }) => assert_eq!(name, "foobar"),
            other => panic!("expected UnknownCommand, got {:?}", other),
        }
    }

    #[test]
    fn unbalanced_brace_is_reported() {
        // \frac{a} は分母の {...} が無い → ExpectedGroup(次が LBrace でない/尽きた)
        assert!(matches!(parse(r"\frac{a}"), Err(ParseError::ExpectedGroup { .. })));
        // 閉じられていない { → グループ内 parse_row が尽きて UnexpectedEnd
        assert!(matches!(parse(r"{a"), Err(ParseError::UnexpectedEnd { .. })));
    }

    #[test]
    fn hat_accent_wraps_base_in_underover() {
        assert_eq!(
            parse(r"\hat{y}").unwrap(),
            MathNode::UnderOver {
                base: Box::new(ident("y")),
                under: None,
                over: Some(Box::new(op("^"))),
            }
        );
    }

    #[test]
    fn hat_with_trailing_subscript() {
        // \hat{y}_i: \hat{y} は1個の atom として \hat が消費し、続く `_i` は
        // postfix として \hat の結果(UnderOver)に付く。normalize は UnderOver を
        // 素通り(構造を変えず内部を再帰的に正規化するだけ)することも併せて確認する。
        let got = parse_normalized(r"\hat{y}_i").unwrap();
        let expected = MathNode::Sub {
            base: Box::new(MathNode::UnderOver {
                base: Box::new(ident("y")),
                under: None,
                over: Some(Box::new(op("^"))),
            }),
            sub: Box::new(ident("i")),
        };
        assert_eq!(got, expected);
    }

    #[test]
    fn left_right_fenced() {
        let got = parse(r"\left( a + b \right)").unwrap();
        assert_eq!(
            got,
            MathNode::Fenced {
                open: "(".into(),
                close: ")".into(),
                body: Box::new(MathNode::Row { items: vec![ident("a"), op("+"), ident("b")] }),
            }
        );
    }

    // --- WP-M1(D38): to_tex round-trip ------------------------------------------
    //
    // `parse(to_tex(parse(s))) == parse(s)`(構造同値、文字列の完全一致は求めない)。
    // §6 の対応コマンド全種を1件ずつ、素の `parse`(非正規化)で確認する。

    fn assert_roundtrips(tex: &str) {
        let original = parse(tex).unwrap_or_else(|e| panic!("precondition parse({tex:?}) failed: {e:?}"));
        let rewritten = to_tex(&original);
        let reparsed = parse(&rewritten)
            .unwrap_or_else(|e| panic!("to_tex({tex:?}) -> {rewritten:?} failed to reparse: {e:?}"));
        assert_eq!(original, reparsed, "round-trip mismatch for {tex:?} (to_tex: {rewritten:?})");
    }

    #[test]
    fn roundtrip_ident_and_num() {
        assert_roundtrips("x");
        assert_roundtrips("314");
    }

    #[test]
    fn roundtrip_frac() {
        assert_roundtrips(r"\frac{a}{b}");
        assert_roundtrips(r"\frac{1}{\frac{a}{b}}");
    }

    #[test]
    fn roundtrip_sub_sup_subsup() {
        assert_roundtrips("x^2");
        assert_roundtrips("x_{ij}");
        assert_roundtrips("x_i^2");
    }

    #[test]
    fn roundtrip_sqrt_and_root() {
        assert_roundtrips(r"\sqrt{2}");
        assert_roundtrips(r"\sqrt[3]{x}");
    }

    #[test]
    fn roundtrip_big_operators_and_normalize() {
        assert_roundtrips(r"\sum_{i=1}^{n} x_i");
        assert_roundtrips(r"\prod_{i=1}^{n} x_i");
        assert_roundtrips(r"\int_{0}^{1} x");

        // normalize() 後(UnderOver 形)でも構造同値を保つこと。
        let original = parse_normalized(r"\sum_{i=1}^{n} x_i").unwrap();
        let rewritten = to_tex(&original);
        let reparsed = parse_normalized(&rewritten).unwrap();
        assert_eq!(original, reparsed);
    }

    #[test]
    fn roundtrip_text() {
        assert_roundtrips(r"\text{hello}");
    }

    #[test]
    fn roundtrip_left_right() {
        assert_roundtrips(r"\left( a + b \right)");
        assert_roundtrips(r"\left[ x \right]");
    }

    #[test]
    fn roundtrip_greek_letters() {
        for name in ["alpha", "beta", "gamma", "delta", "theta", "lambda", "mu", "pi", "sigma", "phi", "omega"] {
            assert_roundtrips(&format!(r"\{name}"));
        }
        for name in ["Gamma", "Delta", "Theta", "Lambda", "Sigma", "Phi", "Omega"] {
            assert_roundtrips(&format!(r"\{name}"));
        }
    }

    #[test]
    fn roundtrip_named_ops() {
        for name in ["times", "cdot", "div", "pm", "leq", "geq", "neq", "approx", "to", "infty", "partial", "nabla", "in"] {
            assert_roundtrips(&format!(r"a \{name} b"));
        }
    }

    #[test]
    fn roundtrip_hat_accent() {
        assert_roundtrips(r"\hat{y}");
        assert_roundtrips(r"\hat{y}_i");
    }

    #[test]
    fn roundtrip_quadratic_formula() {
        assert_roundtrips(r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}");
    }

    #[test]
    fn roundtrip_loss_formula_from_fixture() {
        // docs/sml_example_formatted.sml の ::math ブロックと同じ式(D38 ドッグ
        // フーディングで実際に描画される式そのもので固定する)。
        assert_roundtrips(r"L = \frac{1}{N} \sum_{i=1}^{N} (y_i - \hat{y}_i)^2");
    }
}
