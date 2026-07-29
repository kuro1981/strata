# Typst レンダラバグ根本修正ハンドオフ

`docs/spec-sml/{grammar,decisions}.sml` の SML 化作業中に発見された
strata-typst の実バグの根本修正。現状は 6 箇所に手動で空白を挿入する
回避策が入っている(削除・恒久修正が目標)。

## 症状

Typst 出力で、リンク(`#link(...)[...]`)や取消線(`#strike[...]`)のような
「`#関数(...)  [...]` 形式で終わる」構文の直後に、ソース側で空白を挟まず
`(` が続くと(例: `[表示](ref:x)(→ D40 で…` のような SML/Markdown)、
Typst のパーサがこれを**関数呼び出しへの追加引数**と誤解釈し、コンパイル
エラーまたは意図しない出力になる。

## 再現

1. `docs/spec-sml/grammar.sml` の git 履歴(コミット `9009960` 直前の版、
   または `git log -p` で M8/D59 変換時の一時ハンドオフ)から、6箇所の
   手動空白挿入(`~~...~~(→ Dn...)` → `~~...~~ (→ Dn...)` のような)を
   特定する(`git log --all --oneline -- docs/spec-sml/grammar.sml` 等で
   変換過程のコミットを辿るか、下記の最小再現で十分)
2. 最小再現: strata の任意のスクラッチ SML ファイルに
   ```markdown
   [表示](ref:target-alias)(直後に括弧)
   ```
   と
   ```markdown
   ~~取消線~~(直後に括弧)
   ```
   を書き、`strata build` → `strata render --format typst` → 出力 `.typ` を
   `typst compile` に通してエラー/誤描画を確認する

## 修正方針

`crates/strata-typst/src/lib.rs` の Link・Strike(取消線)のレンダリング箇所
(および同じ「関数呼び出し+contentブロック」形式を使う他のインライン要素、
grep で洗い出す)を調査し、**直後に続くリテラルテキストの先頭文字に依らず
安全な出力**にする。候補(裁量で選択・検証):

- 関数呼び出し全体を Typst の code-mode グループ `#{ ... }` で包む
  (`#{link(url)[text]}` 形式にすると、閉じ `}` の後の `(` は markup 側の
  リテラルとして扱われ、呼び出しへの引数として吸収されなくなる — Typst の
  markup/code モード境界を使って曖昧性を断つ)
- または、後続リテラルの先頭が `(` の場合にのみ間に `#h(0pt)`(ゼロ幅の
  space)を挿入する(こちらは「次の文字を見る」実装が要るため上記より複雑)

前者(`#{...}` 包み)を推奨。実装後、`typst compile` で実際に検証すること。

## 対象範囲

- `crates/strata-typst/src/lib.rs` の Link・Strike 描画箇所が主。他に
  同型のパターン(`#関数(...)  [...]` で終わるインライン)が無いか grep で
  確認し、あれば同様に対処
- strata-md(GFM 出力)は関数呼び出し構文を持たないため対象外の見込み
  (念のため同種の隣接文字問題が無いか確認だけしておく)

## 検証

1. 上記の最小再現ケースが `typst compile` エラーなく正しい見た目になること
   (回帰テストとして `strata-typst` のテストに追加)
2. `docs/spec-sml/grammar.sml` と `decisions.sml` に残る手動空白挿入 6箇所を
   探し出し、**元の空白なし形に戻す**(回避策の撤去)。fmt 冪等・build 診断
   ゼロ・`render --format typst` → `typst compile` が通ることを確認
3. `cargo test --workspace` 全通過、clippy 新規警告ゼロ(strata-html 除く)

## ルール

git 操作禁止。fixture(`docs/sml_example_*`)は変更しない。曖昧な点は裁量として
最終報告に明記。

## 完了の定義

回避策 6 箇所の撤去+根本修正+回帰テスト追加。全テスト green。コミットしない。

最終報告: 根本原因の説明 / 修正方式 / 変更ファイル / 撤去した回避策の一覧と
撤去後の検証結果 / 追加テスト / 裁量箇所
