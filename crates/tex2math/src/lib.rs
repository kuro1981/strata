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
}
