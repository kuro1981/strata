//! 子 List ノードの決定的 ID 導出(sml-spec.md D27、2026-07-15 裁定)。
//!
//! ネストしたリスト項目(D24)の子リスト全体には SML 上に ID を書く場所が無い
//! (前置属性行を置ける位置が存在しない — 親項目の行に埋め込まれるため)。従来は
//! build のたびに `NodeId::new()` で乱数採番していたため、同一入力を2回 build しても
//! 子 List ノードの ID が一致しない(ID 安定の不変条件と矛盾する)問題があった。
//!
//! D27 の裁定: **親リスト項目の ULID + 位置**から決定的に導出する。導出式は
//! Term ノードの安定 ID(D9/D15、`term.rs`)と同型: 名前(ここでは「親項目の ULID +
//! 位置」を文字列化したもの)を `Sha256("strata:list-child:v0:{...}")` に通し、
//! 先頭16バイトを ULID の内部表現として読む。
//!
//! `position` は「親項目が持つ何番目の子リストか」を表す。D24 時点では1項目につき
//! 子リストは高々1つ(「項目=段落1つ」制約により、ネストは項目直下の1本のみ)なので
//! 現状は常に `0` になるが、将来1項目が複数の子リストを持てるよう拡張された場合に
//! 備えて引数として残す(裁量、最終報告に記載)。
use sha2::{Digest, Sha256};
use strata_core::NodeId;
use ulid::Ulid;

pub(crate) fn derive_list_child_id(parent_item_id: NodeId, position: u32) -> NodeId {
    let mut hasher = Sha256::new();
    hasher.update(format!("strata:list-child:v0:{}:{position}", parent_item_id.0));
    let hash = hasher.finalize();
    let bytes: [u8; 16] = hash[..16].try_into().expect("SHA-256 digest is 32 bytes, [..16] always succeeds");
    NodeId(Ulid(u128::from_be_bytes(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 導出式のハードコード固定(term.rs の同種テストと同じ形式)。値は
    /// `Sha256("strata:list-child:v0:01ARZ3NDEKTSV4RRFFQ69G5FAV:0")[..16]` を
    /// 実際に計算して得たもの(`print_frozen_value_for_hardcoding` で確認済み)。
    #[test]
    fn derivation_formula_is_frozen() {
        let parent = NodeId(Ulid::from_string("01ARZ3NDEKTSV4RRFFQ69G5FAV").unwrap());
        assert_eq!(
            derive_list_child_id(parent, 0).0,
            Ulid::from_string("4ETFVD13EH5XC4TRMAKJ9A6Q1X").unwrap()
        );
    }

    #[test]
    fn same_parent_and_position_derives_same_id_across_builds() {
        let parent = NodeId::new();
        assert_eq!(derive_list_child_id(parent, 0), derive_list_child_id(parent, 0));
    }

    #[test]
    fn different_parents_derive_different_ids() {
        let a = NodeId::new();
        let b = NodeId::new();
        assert_ne!(derive_list_child_id(a, 0), derive_list_child_id(b, 0));
    }

    #[test]
    fn different_positions_under_same_parent_derive_different_ids() {
        let parent = NodeId::new();
        assert_ne!(derive_list_child_id(parent, 0), derive_list_child_id(parent, 1));
    }
}
