# グラフの役割再設計 + 発見しやすさ改善 ハンドオフ(strata本体・ui/)

実機ドッグフーディング(`~/dev/strata-notes/zettel/editor-dogfood-2026-07-29.sml`
参照、198文書規模のvault統合後)で見つかった摩擦のうち、ユーザーが今回対処すると
裁定した2件をまとめて実装する。

## 背景(前提として読むこと)

- グラフ表現(local/overview/outline)が198文書規模になると小さくなりすぎて
  可読性を失う、という報告があった。裁定: **「探す」はスイッチャー/全文検索に
  完全委譲し、グラフは「選択ノードの近傍を辿る」専任にする**という役割分担の
  再設計を採用する。
- `ClassToggle`(class:チップの列)が `App.tsx` で GraphPane・DocumentPane
  両方を包む `<main>` の外側・上に置かれた全幅ツールバー行にあり、かつ
  `isHidden()` 経由で**両ペイン(グラフ・文書)から消える**設計になっている
  (`crates/../ui/src/components/blocks/BlockTree.tsx` 29行目・61行目で
  `isHidden` を消費している)。Obsidianのグラフフィルタはグラフ描画だけに
  効き、ノート本文表示は変えない。裁定: **Obsidian同型にする**(グラフのみに
  効く・グラフペインの隅に付属する専用パネルとして配置し直す)。

## 作業1: グラフの役割再設計

現状の3モード(local/overview/outline)のうち、**local を唯一の主要モードとして
確立**する。overview/outline は「大規模データの中から探す」用途としては
スイッチャー/全文検索に完全に道を譲り、あくまで「ローカルな構造を俯瞰したい
ときの補助」という位置づけに格下げする(裁量: モードボタン自体を消すか、
残しつつ既定を強くlocalに寄せるか、UIコピー(ラベル文言)で位置づけを明示する
かは実装時に判断してよい。最終報告に明記)。

- `ui/src/state/GraphContext.tsx` の `graphMode` 既定値は既に `"local"`
  なので変更不要のはず(確認すること)。
- local モードが実際に「探す」の代替にならずに済んでいるか(=常に小さく
  留まるか)を、198文書規模の `~/dev/strata-notes` を使って実地確認する。
  特に **contains 隣接(親・子・兄弟)の表示件数**(`lib/localLayout.ts`)が、
  子・兄弟の多い文書(例: 大きな記事・長いリスト)で肥大化してラベル過密に
  ならないか確認し、なるようなら「先頭N件+『他M件』」のような打ち切り表示を
  検討する(裁量、必要と判断した場合のみ実装。過剰設計しないこと)。
- overview/outline モードの切替ボタン自体(`GraphPane.tsx` 冒頭のモード
  トグル)は残してよいが、「探す」目的で誤って使われないよう、ラベルや
  tooltip文言を「構造の俯瞰(補助)」的な位置づけに調整することを検討する
  (裁量)。

## 作業2: ClassToggle をグラフ専用パネルへ

1. **適用範囲をグラフのみに絞る**: `GraphContext` の `hiddenClasses`/`isHidden`
   を、GraphPane 用と DocumentPane 用で分離する。具体的には
   `hiddenClasses`(既存、グラフ用として維持)とは別に、`isHidden` の消費箇所を
   `GraphPane.tsx`/`LocalGraph` 等のグラフ描画専用に限定し、
   `BlockTree.tsx` からは `isHidden` 呼び出しを削除する(文書ペインは常に
   全ブロックを表示する、Obsidianのノートエディタと同じ挙動に合わせる)。
   `GraphContext` の型・API(`isHidden`/`hiddenClasses`/`toggleClass`)自体は
   そのまま維持してよく、呼び出し側を絞るだけで足りるはず(要確認)。
2. **配置をグラフペインの隅へ**: `ClassToggle` コンポーネントを `App.tsx` の
   全幅ツールバー行から外し、`GraphPane.tsx` 内(既存の
   local/俯瞰/全展開モードボタンがある行、またはグラフ描画エリアの右上隅の
   オーバーレイ)に移動する。多数のclassがある場合(実データで30種類以上)に
   全幅ツールバーのようにレイアウトを圧迫しないよう、折りたたみ可能な
   ポップオーバー/ドロワー形式にすることを検討する(裁量、既存の
   `components/ui` のプリミティブで実現できる形を選ぶこと)。
3. `App.tsx` 側で `<ClassToggle />` を呼んでいた箇所を削除し、代わりに
   `GraphPane` が内部で呼ぶ形にする(`ui/` は strata-editor にも
   `strata site` にも共有されるコンポーネントなので、`ui/` 側だけで完結
   させ、strata-editor の `App.tsx` 側の変更は不要なはず — 確認すること)。

## 検証

- `~/dev/strata-notes`(198文書)を使い、local モードのラベル過密が実害
  レベルでないことを確認(視覚的な目視確認がこの環境で難しい場合は、
  `computeLocalLayout` の出力ノード数・重なり検出等、既存のテスト手法に
  倣った回帰テストで代替可)
- class を1つOFFにしたとき、グラフからは消えるが文書ペインの内容は変わらない
  ことを確認する回帰テスト(既存の `hiddenClasses`/`isHidden` 関連テストが
  あれば参考にすること)
- `cargo test --workspace`(ui/には無関係だが strata 全体の regression 用)・
  `pnpm build`(ui/)通過
- fixture(`docs/sml_example_*`)は無関係のはずだが変更しないこと

## ルール

git操作禁止。曖昧な点(overview/outlineモードボタンの扱い、ClassToggleの
UI形式)は裁量として最終報告に明記。過剰設計しないこと(198文書規模で実際に
困る具体的な症状を解消することが目的で、汎用フレームワーク化は不要)。

## 完了の定義

作業1・作業2実装完了、検証項目通過。コミットしない。

最終報告: 各作業の実装内容 / class フィルタの適用範囲分離の実装方式 /
ClassToggleのUI形式(採用した形とその理由) / overview/outlineモードの
扱いをどうしたか / 裁量箇所
