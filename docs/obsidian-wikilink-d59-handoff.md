# D59 実装ハンドオフ — Obsidian `[[wikilink]]` 対応

sml-spec §1.18(2026-07-29 確定)の実装指示。実 Obsidian vault
(`kurochanBrainPrivate`、304ファイル中100が wikilink 使用)の Strata 統合の
ブロッカー解消。

## 必読

1. `AGENTS.md`
2. `docs/sml-spec.md` §1.18(D59)・§5.2(既存インライン参照 `ref:`/`term:` 等)・
   §10(埋め込み `![[...]]` は対象外)
3. `crates/strata-sml/src/inline.rs`(既存 `[text](scheme:target)` パーサ)、
   `crates/strata-build/src/{resolve,convert}.rs`(参照解決)

## スコープ境界(やらないこと)

- 埋め込み `![[target]]`(§10 保留。今回は素通しのまま)
- Dataview インラインフィールド・callout(§10 保留、低頻度)
- 実 vault(`kurochanBrainPrivate`)への書き込み・移行作業そのもの(別タスク。
  今回は言語機能の実装のみ)
- fixture 改版禁止、既存コマンド非退行

## 実装内容

### WP-D59-1: パーサ

- `[[target]]` と `[[target|表示テキスト]]` の2形式をインライン記法として認識
  (`crates/strata-sml/src/inline.rs`)。`|` 前後の空白は許容
- 新しい `Inline` バリアント、または既存 `Ref` の特殊ケースとして扱うかは裁量
  (後続の解決フェーズがタイトル一致という**特殊な解決方式**を要するため、
  パース時点で「wikilink 由来」のマーカーを残す必要がある — 既存の
  `ref:`/`term:` 等のスキーム付き Ref とは解決ロジックが異なる)
- 角括弧のエスケープ(D40 で対応済みの `\[`)との相互作用を確認

### WP-D59-2: 解決(build)

- **解決方式(D59 確定)**: ワークスペース内の**文書タイトル**(フロントマター
  `title:` → 無ければ最初の H1 → 無ければファイル名 stem)との**完全一致**
  (大文字小文字・NFC 正規化は既存の Term ID 正規化(D15)を参考に方針を決め
  報告)で対象 Document ノードを解決する
- 単一ファイル build でも同一文書内の自己参照は成立してよいが、
  他文書へのタイトル一致は **ワークスペース build が必要**(D42/D53 の
  `CrossDocRef`/`DocRefNeedsWorkspace` と同じ思想)
- **同名衝突**: 複数文書が同じタイトルを持つ場合は診断 Error で曖昧性を報告
  (新規診断種別、名前は裁量)。黙って先頭を選ばない
- **未解決**: 一致する文書が無ければ診断 Error(新規診断種別)。CommonMark 側の
  「素通し」とは違い、wikilink は明示的にリンクの意図があるため黙って
  リテラル化しない(D59 の狙いそのもの — 静かに壊れる状態の解消)
- canonical グラフ上は解決後、対象 **Document ノードへの通常の Ref**
  (`rel: refers-to`)になる(D59 記載どおり、ソース記法の情報=wikilink 由来
  であることは保持しない)。`doc:` スキーム(D53)の解決経路を再利用できるか
  確認し、できれば共通化(車輪の再発明をしない)

### WP-D59-3: 波及

- `strata-typst` / `strata-md` / `strata-context`: 通常の Document 向け Ref
  (D53/D44 で実装済み)と同じ描画経路に乗るはずなので、新規実装は最小限のはず
  — 確認して報告
- `strata-search`: wikilink 経由の参照も通常の refers-to エッジとして
  検索・逆引き対象になることを確認

## テスト

- パース: `[[target]]`・`[[target|text]]`・エスケープとの相互作用
- 解決: 単一文書内自己参照・ワークスペース内他文書解決・単一ファイル build
  でのワークスペース外参照エラー・同名衝突エラー・未解決エラー
- 実データ smoke: `kurochanBrainPrivate` から**代表的な数ファイルだけ**を
  スクラッチにコピーして(実 vault は変更しない)、fmt → build が通ることを
  確認。304ファイル全部の一括検証はしない(このハンドオフのスコープ外)

## 完了の定義

- `cargo test --workspace` 全通過、clippy 新規警告ゼロ(strata-html 除く)
- fixture 無変更、既存コマンド非退行
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所(Inline 表現方法・
  診断種別名・NFC正規化方針・doc: との共通化可否)・実データ smoke 結果を
  まとめて終了
