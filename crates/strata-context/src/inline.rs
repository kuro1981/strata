//! インライン列(`Vec<Inline>`)→ プレーンテキスト/Markdown への変換。
//!
//! strata-typst の `plain_text`/`render_inlines` と同型だが、Typst エスケープではなく
//! Markdown の軽い強調記法を使う。参照(`Ref`/`Term`)は表示テキストがあればそれを、
//! 無ければ参照先の短いラベルを埋め込む(リンクは張らない — 引用は「エッジ」節/
//! アドレスタグの ULID/alias で行う設計のため、本文中は読みやすさ優先)。

use strata_core::{EmphKind, Graph, Inline, MathNode};

use crate::label::node_short_label;

/// 整形を全て捨てたプレーンテキスト(見出し・位置文脈・短いラベルに使う)。
pub(crate) fn plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { s } => out.push_str(s),
            Inline::Emph { children, .. } => out.push_str(&plain_text(children)),
            Inline::Ref { text, .. } => out.push_str(text),
            Inline::Term { text, .. } => out.push_str(text),
            // M6(D40): 外部リンクは表示テキスト、画像は alt をプレーンテキストとする。
            Inline::Link { text, .. } => out.push_str(text),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::Math { .. } | Inline::Anchor { .. } => {}
        }
    }
    out
}

/// Markdown 本文用のインライン描画。`graph` は `Ref`/`Term`/`Anchor` の表示テキストが
/// 空の場合に参照先のラベルを引くために使う。
pub(crate) fn render_inlines_md(graph: &Graph, inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { s } => out.push_str(&md_escape(s)),
            Inline::Emph { kind, children } => {
                let inner = render_inlines_md(graph, children);
                match kind {
                    EmphKind::Strong => out.push_str(&format!("**{}**", inner)),
                    EmphKind::Em => out.push_str(&format!("_{}_", inner)),
                    EmphKind::Code => out.push_str(&format!("`{}`", inner)),
                    // M6(D40 Tier2): 取消線(GFM 記法)。
                    EmphKind::Strike => out.push_str(&format!("~~{}~~", inner)),
                }
            }
            Inline::Math { tree } => out.push_str(&format!("${}$", render_math_text(tree))),
            Inline::Ref { to, text, .. } => {
                if !text.is_empty() {
                    out.push_str(&md_escape(text));
                } else {
                    out.push_str(&node_short_label(graph, *to));
                }
            }
            Inline::Term { to, text } => {
                if !text.is_empty() {
                    out.push_str(&md_escape(text));
                } else {
                    out.push_str(&node_short_label(graph, *to));
                }
            }
            Inline::Anchor { to } => {
                if let Some(strata_core::NodePayload::Anchor(a)) = graph.nodes.get(to).map(|n| &n.payload) {
                    out.push_str(&render_inlines_md(graph, &a.inline));
                }
            }
            // M6(D40): 外部リンク/画像は Markdown 記法そのままで出す(context ビューは
            // Markdown なので情報を失わず自然に表現できる)。
            Inline::Link { url, text } => {
                if text == url {
                    out.push_str(&format!("<{url}>"));
                } else {
                    out.push_str(&format!("[{}]({})", md_escape(text), url));
                }
            }
            Inline::Image { url, alt } => {
                out.push_str(&format!("![{}]({})", md_escape(alt), url));
            }
        }
    }
    out
}

/// `MathNode` の可読プレーンテキスト表現(組版はしない。LLM が式の構造を読めれば十分)。
pub(crate) fn render_math_text(node: &MathNode) -> String {
    match node {
        MathNode::Num { v } | MathNode::Ident { v } | MathNode::Op { v } => v.clone(),
        MathNode::Row { items } => items.iter().map(render_math_text).collect::<Vec<_>>().join(" "),
        MathNode::Frac { num, den } => format!("({})/({})", render_math_text(num), render_math_text(den)),
        MathNode::Sup { base, sup } => format!("{}^({})", render_math_text(base), render_math_text(sup)),
        MathNode::Sub { base, sub } => format!("{}_({})", render_math_text(base), render_math_text(sub)),
        MathNode::SubSup { base, sub, sup } => {
            format!("{}_({})^({})", render_math_text(base), render_math_text(sub), render_math_text(sup))
        }
        MathNode::UnderOver { base, under, over } => {
            let mut s = render_math_text(base);
            if let Some(u) = under {
                s.push_str(&format!("_({})", render_math_text(u)));
            }
            if let Some(o) = over {
                s.push_str(&format!("^({})", render_math_text(o)));
            }
            s
        }
        MathNode::Sqrt { body } => format!("sqrt({})", render_math_text(body)),
        MathNode::Root { radicand, index } => {
            format!("root({}, {})", render_math_text(index), render_math_text(radicand))
        }
        MathNode::Fenced { open, close, body } => format!("{}{}{}", open, render_math_text(body), close),
        MathNode::Text { s } => format!("\"{}\"", s),
    }
}

/// Markdown の軽い特殊文字だけをエスケープする(見出し記号の `#`、強調の `*`/`_`、
/// インラインコードの `` ` ``)。SML の元テキストは通常散文なので過剰なエスケープは
/// 避け、可読性を優先する(裁量)。
fn md_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('*', "\\*").replace('_', "\\_").replace('`', "\\`")
}
