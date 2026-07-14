//! Term ノードの安定 ID 導出(sml-build-m3-handoff.md D-B5、sml-spec.md D9、D15)。
//!
//! 導出式は v0 として凍結: 名前をまず **NFC 正規化**し、
//! `Sha256(f"strata:term:v0:{正規化後の name}")` の先頭16バイトをビッグエンディアン
//! u128 として読み、それを ULID の内部表現にする(2026-07-14 裁定、D15)。
//!
//! NFC 正規化を挟むのは、視覚的に同じ用語名が Unicode 正規化形式の違い(NFC/NFD)で
//! 別 ID になってしまう既知の問題(M3 実装時点のハンドオフ記載)を解消するため:
//! 「が」(NFC の合成済み1文字)と「か」+結合濁点(NFD の分解形)は正規化後は
//! どちらも同じ NFC 文字列になり、同一 term ID を得る。
//!
//! 同名(NFC 正規化後に同一)の term は必ず同じ ID になるため、build はグラフに
//! 1回だけ Term ノードを insert すればよい(呼び出し側 `Builder::term_id` がキャッシュ
//! して重複 insert を避ける)。

use sha2::{Digest, Sha256};
use strata_core::NodeId;
use ulid::Ulid;
use unicode_normalization::UnicodeNormalization;

pub(crate) fn derive_term_id(name: &str) -> NodeId {
    let normalized: String = name.nfc().collect();
    let mut hasher = Sha256::new();
    hasher.update(format!("strata:term:v0:{normalized}"));
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

    /// D15(2026-07-14 裁定): NFC 分解形と NFD 分解形は正規化後に同じ ID になる。
    /// 「が」(NFC: 1コードポイント U+304C)と「か」(U+304B)+ 結合濁点(U+3099、
    /// NFD 分解形)は視覚的に同じ文字だが、正規化しなければ別バイト列としてハッシュ
    /// される。ここでは実際にその2形("パイプライン" という語の「が」を NFC/NFD
    /// それぞれで書いた版)が同一 ID に写ることを固定する。
    #[test]
    fn nfd_decomposed_input_derives_same_id_as_nfc() {
        let nfc = "パイプラインが効率的"; // 「が」= U+304C(合成済み)
        let nfd = "パイプラインか\u{3099}効率的"; // 「か」(U+304B) + 結合濁点(U+3099)
        assert_ne!(nfc.as_bytes(), nfd.as_bytes(), "test precondition: inputs must differ byte-for-byte");
        assert_eq!(derive_term_id(nfc), derive_term_id(nfd));
    }
}
