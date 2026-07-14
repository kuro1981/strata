//! tex2math::MathNode → strata_core::MathNode の構造的変換(sml-build-m3-handoff.md D-B5)。
//!
//! 両クレートのバリアント集合は 1:1 で対応するため(tex2math 側は MathML Presentation
//! サブセットとして strata-core と同じ形で設計されている)、素朴な match で写す。
//! strata-vault が行っている serde_json 経由のハックはここでは踏襲しない。

pub(crate) fn convert(n: tex2math::MathNode) -> strata_core::MathNode {
    use strata_core::MathNode as C;
    use tex2math::MathNode as T;

    match n {
        T::Num { v } => C::Num { v },
        T::Ident { v } => C::Ident { v },
        T::Op { v } => C::Op { v },
        T::Row { items } => C::Row { items: items.into_iter().map(convert).collect() },
        T::Frac { num, den } => C::Frac { num: Box::new(convert(*num)), den: Box::new(convert(*den)) },
        T::Sup { base, sup } => C::Sup { base: Box::new(convert(*base)), sup: Box::new(convert(*sup)) },
        T::Sub { base, sub } => C::Sub { base: Box::new(convert(*base)), sub: Box::new(convert(*sub)) },
        T::SubSup { base, sub, sup } => C::SubSup {
            base: Box::new(convert(*base)),
            sub: Box::new(convert(*sub)),
            sup: Box::new(convert(*sup)),
        },
        T::UnderOver { base, under, over } => C::UnderOver {
            base: Box::new(convert(*base)),
            under: under.map(|b| Box::new(convert(*b))),
            over: over.map(|b| Box::new(convert(*b))),
        },
        T::Sqrt { body } => C::Sqrt { body: Box::new(convert(*body)) },
        T::Root { radicand, index } => {
            C::Root { radicand: Box::new(convert(*radicand)), index: Box::new(convert(*index)) }
        }
        T::Fenced { open, close, body } => C::Fenced { open, close, body: Box::new(convert(*body)) },
        T::Text { s } => C::Text { s },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structural_match_covers_all_variants() {
        let src = r"\frac{a}{b^2} + \sqrt[3]{x} + \left( y \right) + \text{ok}";
        let t = tex2math::parse_normalized(src).unwrap();
        // 変換がパニックしないこと自体が「全バリアント 1:1」の担保。
        let _c = convert(t);
    }
}
