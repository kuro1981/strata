//! Term ノードの安定 ID 導出(sml-build-m3-handoff.md D-B5、sml-spec.md D9)。
//!
//! 導出式はハンドオフで凍結済み: `Sha256(f"strata:term:v0:{name}")` の先頭16バイトを
//! ビッグエンディアン u128 として読み、それを ULID の内部表現にする。名前は書かれた
//! ままの UTF-8 文字列で、Unicode 正規化はしない(既知の制約。ハンドオフの「既知の
//! 注意点」参照: `予測精度` と NFD 分解形は別 ID になる)。
//!
//! 同名の term は必ず同じ ID になるため、build はグラフに1回だけ Term ノードを
//! insert すればよい(呼び出し側 `Builder::term_id` がキャッシュして重複 insert を避ける)。

use sha2::{Digest, Sha256};
use strata_core::NodeId;
use ulid::Ulid;

pub(crate) fn derive_term_id(name: &str) -> NodeId {
    let mut hasher = Sha256::new();
    hasher.update(format!("strata:term:v0:{name}"));
    let hash = hasher.finalize();
    let bytes: [u8; 16] = hash[..16].try_into().expect("SHA-256 digest is 32 bytes, [..16] always succeeds");
    NodeId(Ulid(u128::from_be_bytes(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 導出式のハードコード固定(WP-B4 完了チェックリスト)。値は
    /// `Sha256("strata:term:v0:予測精度")[..16]` を実際に計算して得たもの。
    #[test]
    fn derivation_formula_is_frozen() {
        assert_eq!(derive_term_id("予測精度").0, Ulid::from_string("6MNYNRE7W9QBQSW9JGQS9R2CT6").unwrap());
        assert_eq!(derive_term_id("予測モデル").0, Ulid::from_string("0S0P70CAD95REZJ1ZFZW4RDKHT").unwrap());
        assert_eq!(derive_term_id("推論速度").0, Ulid::from_string("6S7KKGH307X3QA0PTPYPQ7HVD3").unwrap());
        assert_eq!(derive_term_id("foo").0, Ulid::from_string("0QHH0WKXWKFDZ8ZRJ4AMFZEEDW").unwrap());
    }

    #[test]
    fn same_name_derives_same_id() {
        assert_eq!(derive_term_id("同じ名前"), derive_term_id("同じ名前"));
    }

    #[test]
    fn different_names_derive_different_ids() {
        assert_ne!(derive_term_id("a"), derive_term_id("b"));
    }
}
