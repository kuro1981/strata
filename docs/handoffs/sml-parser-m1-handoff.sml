---
id: 01KYP2JYFBH3Y6RCFE09BTFAXA
title: "Milestone 1 実装ハンドオフ(サブエージェント向け作業指示)"
alias: sml-parser-m1-handoff
---

# Milestone 1 実装ハンドオフ(サブエージェント向け作業指示) {#01KYP2JYFBH3Y6RCFE09BTFAXB}

[id=01KYP2JYFBH3Y6RCFE09BTFAXC]
本書は SML パーサ(Milestone 1)の実装を、設計セッションとは別のセッション/
サブエージェントに引き継ぐための自己完結な作業指示書。

## 必読ドキュメント(この順で読むこと) {#01KYP2JYFBH3Y6RCFE09BTFAXD}

[id=01KYP2JYFBH3Y6RCFE09BTFAXE]
1. `AGENTS.md` — リポジトリの作業ルール(**commit はユーザー指示なしに絶対しない**、
   テスト時は実装を触らない・実装時はテストを触らない) {#01KYP2JYFBH3Y6RCFE09BTFAXF}
2. [grammar.sml](doc:grammar)+[decisions.sml](doc:decisions) — SML の**正典**。記法・[D1](ref:decisions/d1)〜[D6](ref:decisions/d6) の設計決定。食い違ったらこれが正 {#01KYP2JYFBH3Y6RCFE09BTFAXG}
3. [sml-parser-design](doc:sml-parser-design) — 本実装の設計書。アーキテクチャ・データ構造・
   テスト戦略・受け入れ条件はすべてここに従う {#01KYP2JYFBH3Y6RCFE09BTFAXH}
4. `docs/sml_example_draft.sml` / `docs/sml_example_formatted.sml` — ゴールデンペア {#01KYP2JYFBH3Y6RCFE09BTFAXJ}
5. 参考: `crates/tex2math/src/lib.rs` — 手書きパーサ+enum エラーのリポジトリ内前例 {#01KYP2JYFBH3Y6RCFE09BTFAXK}

## スコープ境界(やらないこと) {#01KYP2JYFBH3Y6RCFE09BTFAXM}

[id=01KYP2JYFBH3Y6RCFE09BTFAXN]
- **fmt(M2)・build(M3)は実装しない**。パース(テキスト→スパン付き SML-AST)まで {#01KYP2JYFBH3Y6RCFE09BTFAXP}
- **strata-core / tex2math に依存しない**。数式は TeX ソース文字列+スパンのまま保持 {#01KYP2JYFBH3Y6RCFE09BTFAXQ}
- **エイリアス→ULID の解決をしない**。AST は `RefTarget::Ulid | Label` の区別まで {#01KYP2JYFBH3Y6RCFE09BTFAXR}
- 既存クレート(strata-core / strata-vault / strata-html / strata-typst / strata-cli /
  tex2math)には**一切手を入れない** {#01KYP2JYFBH3Y6RCFE09BTFAXS}
- `docs/` の仕様書も変更しない。仕様の矛盾・曖昧さを見つけたら、勝手に解釈して
  進めず**報告リストに積んで作業を止めるか、最も保守的な解釈を採って報告する** {#01KYP2JYFBH3Y6RCFE09BTFAXT}

## 作業パッケージ分割(WP) {#01KYP2JYFBH3Y6RCFE09BTFAXV}

[id=01KYP2JYFBH3Y6RCFE09BTFAXW]
依存関係: `WP1 → WP2 → {WP3 ∥ WP4} → WP5`(WP3 と WP4 は並列可)

### WP1: クレート雛形 + 層A(ブロックスキャナ) {#01KYP2JYFBH3Y6RCFE09BTFAXX}

[id=01KYP2JYFBH3Y6RCFE09BTFAXY]
- `crates/strata-sml` をワークスペースに追加(edition 2024、依存: serde/serde_json/ulid
  — 他クレートと同バージョン指定に揃える) {#01KYP2JYFBH3Y6RCFE09BTFAXZ}
- `span.rs`(`Span { start, end }` バイトオフセット)、`scan.rs`(設計書 §3 層A)、
  `error.rs`(`Diag`/`DiagKind` の骨格) {#01KYP2JYFBH3Y6RCFE09BTFAY0}
- ブロック種別判定: 見出し / リスト項目 / `::` フェンス / コードフェンス / 属性行 / 段落 {#01KYP2JYFBH3Y6RCFE09BTFAY1}
- フェンスは閉じまで本体を不透明スパンとして飲む。閉じ忘れ → `UnclosedFence` {#01KYP2JYFBH3Y6RCFE09BTFAY2}
- 属性行の束縛規則(直後に空行なしでブロックが続く場合のみ。孤立 → `OrphanAttrLine`) {#01KYP2JYFBH3Y6RCFE09BTFAY3}
- **受け入れ**: スパン被覆不変条件テスト(昇順・非重複・隙間は空行のみ・全被覆)が、
  ゴールデンペア2ファイル+意地悪入力(空ファイル/空行のみ/閉じ忘れ)で通る {#01KYP2JYFBH3Y6RCFE09BTFAY4}

### WP2: 層B ブロック内パース(`block.rs`) {#01KYP2JYFBH3Y6RCFE09BTFAY5}

[id=01KYP2JYFBH3Y6RCFE09BTFAY6]
- 行末 `{#id}` / `{#ULID alias=x}` タグの抽出。`inner_span`(fmt の置換対象)を正確に {#01KYP2JYFBH3Y6RCFE09BTFAY7}
- 属性行の `key=value` パース。リスト値 `supports=[a, b]`、引用符付き値 `caption="..."` {#01KYP2JYFBH3Y6RCFE09BTFAY8}
- ULID 判定(26字 Crockford Base32)と `RefTarget::Ulid | Label` の振り分け {#01KYP2JYFBH3Y6RCFE09BTFAY9}
- `{#}` と `[id=]` の併記検出 → `DuplicateId` {#01KYP2JYFBH3Y6RCFE09BTFAYA}
- key/エイリアス字句 `[A-Za-z0-9_-]+` の検証 → `BadKeyCharset`(D5) {#01KYP2JYFBH3Y6RCFE09BTFAYB}
- **受け入れ**: IDタグ4形(なし / `{#ULID}` / `{#label}` / `{#ULID alias=x}`)×
  ブロック位置(見出し/リスト項目/フェンス)の組み合わせテスト {#01KYP2JYFBH3Y6RCFE09BTFAYC}

### WP3: `::table` 本体(`table.rs`) {#01KYP2JYFBH3Y6RCFE09BTFAYD}

[id=01KYP2JYFBH3Y6RCFE09BTFAYE]
- `@rows:` / `@cols:` / `@cells:` セクション、フェンス内属性行、行頭 `#` コメント {#01KYP2JYFBH3Y6RCFE09BTFAYF}
- インデント(2スペース)による次元⇄メンバー交互ネスト、フラット糖衣 `- name: [a, b]`、
  member ラベル `- key "表示名"` {#01KYP2JYFBH3Y6RCFE09BTFAYG}
- セル行 `path | path : 値`。値の型付きパース6種(sml-spec §6.1 D4 の表の通り。
  数量 `45 ms` の単位トークン規則は設計書 §10 参照) {#01KYP2JYFBH3Y6RCFE09BTFAYH}
- 座標の葉パス実在検証は**やらない**(build の仕事。字句検証まで) {#01KYP2JYFBH3Y6RCFE09BTFAYJ}
- **受け入れ**: ゴールデンペアの表がパースでき、次元木・セル値型が期待値と一致。
  `BadCellCoord` / `InconsistentIndent` の失敗ケーステスト {#01KYP2JYFBH3Y6RCFE09BTFAYK}

### WP4: インラインパース(`inline.rs`) {#01KYP2JYFBH3Y6RCFE09BTFAYM}

[id=01KYP2JYFBH3Y6RCFE09BTFAYN]
- `**strong**` `*em*` `` `code` `` `$tex$`(スパンのみ)/ 参照5スキーム
  (`ref:` `term:` `table:` `fig:` `math:` `cell:...#path|path`) {#01KYP2JYFBH3Y6RCFE09BTFAYP}
- 未対応・不正なインライン構文は**プレーンテキストにフォールバック**
  (ブロックは厳格、インラインは寛容 — 設計書 §3) {#01KYP2JYFBH3Y6RCFE09BTFAYQ}
- `term:` の target のみ日本語等の用語名を許す(他は ULID/エイリアス字句) {#01KYP2JYFBH3Y6RCFE09BTFAYR}
- **受け入れ**: 各スキーム+閉じ忘れ+`UnknownScheme` のテスト。
  vault の旧実装(`strata-vault/src/lib.rs` の `parse_inline_str`)は**参考にしない**
  こと(旧仕様。rel を DependsOn に誤って畳んでいる) {#01KYP2JYFBH3Y6RCFE09BTFAYS}

### WP5: 統合・ゴールデンテスト(`tests/golden.rs`) {#01KYP2JYFBH3Y6RCFE09BTFAYT}

[id=01KYP2JYFBH3Y6RCFE09BTFAYV]
- ゴールデンペア2ファイルがエラーゼロ(diags 空)でパース {#01KYP2JYFBH3Y6RCFE09BTFAYW}
- **draft と formatted の AST が「IDタグ・id属性を無視すれば同型」**の検証
  (同型比較関数もこの WP で書く) {#01KYP2JYFBH3Y6RCFE09BTFAYX}
- 非対応 Markdown(blockquote / GFM表 / setext見出し)がエラーでなく
  プレーンテキスト/段落になるフォールバックテスト {#01KYP2JYFBH3Y6RCFE09BTFAYY}
- 設計書 §8 の受け入れ条件チェックリストを全消化して結果を報告 {#01KYP2JYFBH3Y6RCFE09BTFAYZ}

## 完了の定義(全WP後) {#01KYP2JYFBH3Y6RCFE09BTFAZ0}

[id=01KYP2JYFBH3Y6RCFE09BTFAZ1]
[sml-parser-design](doc:sml-parser-design) §8 のチェックリスト全項目 + `cargo test --workspace` 全通過
[id=01KYP2JYFBH3Y6RCFE09BTFAZ2]
+ `cargo clippy` 警告ゼロ。**コミットはせず**、変更ファイル一覧と受け入れ条件の
消化状況・発見した仕様の曖昧点を報告して終了する。 {#01KYP2JYFBH3Y6RCFE09BTFAZ3}

## 実装中に裁量で決めてよいこと(設計書 §10) {#01KYP2JYFBH3Y6RCFE09BTFAZ4}

[id=01KYP2JYFBH3Y6RCFE09BTFAZ5]
- `Span` の行/列変換の持ち方(キャッシュ or 都度計算) {#01KYP2JYFBH3Y6RCFE09BTFAZ6}
- 行末 `{#id}` 検出の空白規則(末尾空白は許す方向) {#01KYP2JYFBH3Y6RCFE09BTFAZ7}
- 内部モジュールの細かい分割・ヘルパの置き場 {#01KYP2JYFBH3Y6RCFE09BTFAZ8}

[id=01KYP2JYFBH3Y6RCFE09BTFAZ9]
これ以外の仕様判断(記法の追加・変更に見えるもの)は裁量で決めず報告に回すこと。
