# 副次バグ修正ハンドオフ

Typst 括弧チェーンバグ修正(コミット `1e903ab`)の検証中に見つかった、
無関係な2件の既存不具合を修正する。

## バグ1: `Inline::Anchor` のラベル対象化が機能していない

`crates/strata-typst/src/lib.rs` で `Inline::Anchor` を描画している箇所を
確認せよ。意図としては段落より細かいスパンを昇格させたノード(被参照・
トランスクルージョン可、`strata-core` の doc comment 参照)を Typst 上でも
ラベル付き要素として機能させたいはずだが、現状は単なる `[...]` の content
block になっており、Typst のラベル構文(`<label>` を要素に付ける)として
機能していない(=このノードへの `Ref` が正しく解決しない可能性がある)。

- 現状の実装と、他のブロックノード(Section 等)がどうラベルを付与している
  か(`<ULID>` を要素直後に置く既存パターン)を比較し、Anchor にも同じ
  パターンを適用する
- Anchor への `Ref`(`ref:<anchor-ulid>`)が実際に解決してリンクとして機能
  することを、最小再現(SML で Anchor を作り、別ブロックから参照する)を
  `typst compile` して確認する回帰テストを追加

## バグ2: 一部ハンドオフ `.typ` がインラインコード内エスケープで `typst compile` 失敗

`docs/handoffs/graph-ui-g1-handoff.sml`・`sml-fmt-m2-handoff.sml` 等(実際に
`render --format typst` → `typst compile` で再現し、対象を確定させること)で、
バッククォート(インラインコード)`` ` `` の中にバックスラッシュ+記号
(`\*` 等)が含まれる箇所が、Typst 側で不正なエスケープとして扱われ
コンパイルエラーになる。

- `crates/strata-typst/src/lib.rs` のインラインコード(`Code` inline)描画
  箇所を確認。Typst の raw text(`` `...` ``)構文の中では通常バックスラッシュは
  リテラルのはずだが、実際にどう出力されているか確認せよ
- 原因を特定し(SML 側のインラインコード内容がそのまま Typst の `` ` `` の
  中に埋め込まれる際に、Typst 側で特別な意味を持つ文字(バッククォート
  自体等)のエスケープが必要な場合の処理漏れの可能性が高い)、修正する
- 対象ファイル全部で `typst compile` が通ることを確認。回帰テストを追加

## ルール

git 操作禁止。fixture(`docs/sml_example_*`)は変更しない(バグ2の修正で
fixture 自体に影響が出る場合は、変更内容と理由を明記した上でgolden更新は
可、ただし最小限に)。曖昧な点は裁量として最終報告に明記。

## 完了の定義

両バグ修正+回帰テスト追加、`cargo test --workspace` 全通過、clippy 新規
警告ゼロ(strata-html 除く)。`docs/handoffs/*.sml` 全ファイルが
`typst compile` を通ることを確認。コミットしない。

最終報告: 各バグの原因 / 修正内容 / 影響ファイル / 追加テスト / 検証結果 /
裁量箇所
