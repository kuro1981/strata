# Milestone 1 実装ハンドオフ(サブエージェント向け作業指示)

本書は SML パーサ(Milestone 1)の実装を、設計セッションとは別のセッション/
サブエージェントに引き継ぐための自己完結な作業指示書。

## 必読ドキュメント(この順で読むこと)

1. `AGENTS.md` — リポジトリの作業ルール(**commit はユーザー指示なしに絶対しない**、
   テスト時は実装を触らない・実装時はテストを触らない)
2. `docs/sml-spec.md` — SML の**正典**。記法・D1〜D6 の設計決定。食い違ったらこれが正
3. `docs/sml-parser-design.md` — 本実装の設計書。アーキテクチャ・データ構造・
   テスト戦略・受け入れ条件はすべてここに従う
4. `docs/sml_example_draft.sml` / `docs/sml_example_formatted.sml` — ゴールデンペア
5. 参考: `crates/tex2math/src/lib.rs` — 手書きパーサ+enum エラーのリポジトリ内前例

## スコープ境界(やらないこと)

- **fmt(M2)・build(M3)は実装しない**。パース(テキスト→スパン付き SML-AST)まで
- **strata-core / tex2math に依存しない**。数式は TeX ソース文字列+スパンのまま保持
- **エイリアス→ULID の解決をしない**。AST は `RefTarget::Ulid | Label` の区別まで
- 既存クレート(strata-core / strata-vault / strata-html / strata-typst / strata-cli /
  tex2math)には**一切手を入れない**
- `docs/` の仕様書も変更しない。仕様の矛盾・曖昧さを見つけたら、勝手に解釈して
  進めず**報告リストに積んで作業を止めるか、最も保守的な解釈を採って報告する**

## 作業パッケージ分割(WP)

依存関係: `WP1 → WP2 → {WP3 ∥ WP4} → WP5`(WP3 と WP4 は並列可)

### WP1: クレート雛形 + 層A(ブロックスキャナ)

- `crates/strata-sml` をワークスペースに追加(edition 2024、依存: serde/serde_json/ulid
  — 他クレートと同バージョン指定に揃える)
- `span.rs`(`Span { start, end }` バイトオフセット)、`scan.rs`(設計書 §3 層A)、
  `error.rs`(`Diag`/`DiagKind` の骨格)
- ブロック種別判定: 見出し / リスト項目 / `::` フェンス / コードフェンス / 属性行 / 段落
- フェンスは閉じまで本体を不透明スパンとして飲む。閉じ忘れ → `UnclosedFence`
- 属性行の束縛規則(直後に空行なしでブロックが続く場合のみ。孤立 → `OrphanAttrLine`)
- **受け入れ**: スパン被覆不変条件テスト(昇順・非重複・隙間は空行のみ・全被覆)が、
  ゴールデンペア2ファイル+意地悪入力(空ファイル/空行のみ/閉じ忘れ)で通る

### WP2: 層B ブロック内パース(`block.rs`)

- 行末 `{#id}` / `{#ULID alias=x}` タグの抽出。`inner_span`(fmt の置換対象)を正確に
- 属性行の `key=value` パース。リスト値 `supports=[a, b]`、引用符付き値 `caption="..."`
- ULID 判定(26字 Crockford Base32)と `RefTarget::Ulid | Label` の振り分け
- `{#}` と `[id=]` の併記検出 → `DuplicateId`
- key/エイリアス字句 `[A-Za-z0-9_-]+` の検証 → `BadKeyCharset`(D5)
- **受け入れ**: IDタグ4形(なし / `{#ULID}` / `{#label}` / `{#ULID alias=x}`)×
  ブロック位置(見出し/リスト項目/フェンス)の組み合わせテスト

### WP3: `::table` 本体(`table.rs`)

- `@rows:` / `@cols:` / `@cells:` セクション、フェンス内属性行、行頭 `#` コメント
- インデント(2スペース)による次元⇄メンバー交互ネスト、フラット糖衣 `- name: [a, b]`、
  member ラベル `- key "表示名"`
- セル行 `path | path : 値`。値の型付きパース6種(sml-spec §6.1 D4 の表の通り。
  数量 `45 ms` の単位トークン規則は設計書 §10 参照)
- 座標の葉パス実在検証は**やらない**(build の仕事。字句検証まで)
- **受け入れ**: ゴールデンペアの表がパースでき、次元木・セル値型が期待値と一致。
  `BadCellCoord` / `InconsistentIndent` の失敗ケーステスト

### WP4: インラインパース(`inline.rs`)

- `**strong**` `*em*` `` `code` `` `$tex$`(スパンのみ)/ 参照5スキーム
  (`ref:` `term:` `table:` `fig:` `math:` `cell:...#path|path`)
- 未対応・不正なインライン構文は**プレーンテキストにフォールバック**
  (ブロックは厳格、インラインは寛容 — 設計書 §3)
- `term:` の target のみ日本語等の用語名を許す(他は ULID/エイリアス字句)
- **受け入れ**: 各スキーム+閉じ忘れ+`UnknownScheme` のテスト。
  vault の旧実装(`strata-vault/src/lib.rs` の `parse_inline_str`)は**参考にしない**
  こと(旧仕様。rel を DependsOn に誤って畳んでいる)

### WP5: 統合・ゴールデンテスト(`tests/golden.rs`)

- ゴールデンペア2ファイルがエラーゼロ(diags 空)でパース
- **draft と formatted の AST が「IDタグ・id属性を無視すれば同型」**の検証
  (同型比較関数もこの WP で書く)
- 非対応 Markdown(blockquote / GFM表 / setext見出し)がエラーでなく
  プレーンテキスト/段落になるフォールバックテスト
- 設計書 §8 の受け入れ条件チェックリストを全消化して結果を報告

## 完了の定義(全WP後)

`docs/sml-parser-design.md` §8 のチェックリスト全項目 + `cargo test --workspace` 全通過
+ `cargo clippy` 警告ゼロ。**コミットはせず**、変更ファイル一覧と受け入れ条件の
消化状況・発見した仕様の曖昧点を報告して終了する。

## 実装中に裁量で決めてよいこと(設計書 §10)

- `Span` の行/列変換の持ち方(キャッシュ or 都度計算)
- 行末 `{#id}` 検出の空白規則(末尾空白は許す方向)
- 内部モジュールの細かい分割・ヘルパの置き場

これ以外の仕様判断(記法の追加・変更に見えるもの)は裁量で決めず報告に回すこと。
