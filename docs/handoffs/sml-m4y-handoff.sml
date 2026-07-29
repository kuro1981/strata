---
id: 01KYP2JYF6T0FTERAY58NTT6N8
title: "M4y 実装ハンドオフ — 構造化データ語彙(D26〜D29)"
alias: sml-m4y-handoff
---

# M4y 実装ハンドオフ — 構造化データ語彙(D26〜D29) {#01KYP2JYF6T0FTERAY58NTT6N9}

[id=01KYP2JYF6T0FTERAY58NTT6NA]
本書は sml-spec §1.5(2026-07-15 確定)の実装指示。ビュー v0 で定量化された
SML の語彙欠落(年月正規表現 26 件・key-value 分解 13 件・DRY 違反 30 件)を解消し、
その効果を **convert_sml.py の正規表現削減量**で実証するところまでが M4y。

## 前提 {#01KYP2JYF6T0FTERAY58NTT6NB}

[id=01KYP2JYF6T0FTERAY58NTT6NC]
- M4x 完了(全335テスト green、7329b6d)。ビュー v0 完了:
  `~/dev/strata-my-resume/sml/convert_sml.py` が graph JSON → content/*.yaml →
  JIS テンプレートで PDF 3種を再現済み {#01KYP2JYF6T0FTERAY58NTT6ND}
- v0 の棚卸し詳細は [view-v0-handoff](doc:view-v0-handoff) と本書末尾の「WP-Y4」を参照 {#01KYP2JYF6T0FTERAY58NTT6NE}

## 必読(この順) {#01KYP2JYF6T0FTERAY58NTT6NF}

[id=01KYP2JYF6T0FTERAY58NTT6NG]
1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリ) {#01KYP2JYF6T0FTERAY58NTT6NH}
2. [D26](ref:decisions/d26)〜[D29](ref:decisions/d29)・[フェンスブロック](ref:grammar/fence-blocks)・[セル値の型付きパース](ref:grammar/table-fence-cell-parsing)・
   [保留一覧](ref:grammar/frozen-vs-pending)(ネスト record・値トランスクルージョン・エンティティはやらない) {#01KYP2JYF6T0FTERAY58NTT6NJ}
3. `crates/strata-sml`・`strata-core`・`strata-build`・`strata-typst`・`strata-cli` の現状 {#01KYP2JYF6T0FTERAY58NTT6NK}
4. `~/dev/strata-my-resume/sml/convert_sml.py` と同 `resume.sml` / `work_history.sml` {#01KYP2JYF6T0FTERAY58NTT6NM}

## スコープ境界(やらないこと) {#01KYP2JYF6T0FTERAY58NTT6NN}

[id=01KYP2JYF6T0FTERAY58NTT6NP]
- ネスト record(キーの階層化)、値のトランスクルージョン、エンティティ(§10 保留) {#01KYP2JYF6T0FTERAY58NTT6NQ}
- 宣言的ビュー定義(v1)・テンプレート・マニフェスト(次フェーズ) {#01KYP2JYF6T0FTERAY58NTT6NR}
- strata-html(凍結) {#01KYP2JYF6T0FTERAY58NTT6NS}
- fixture `docs/sml_example_draft.sml` / `sml_example_formatted.sml` の改版
  (record/Date を使っていないので影響しないはず。ゴールデン `.typ` も同様) {#01KYP2JYF6T0FTERAY58NTT6NT}
- `~/dev/strata-my-resume` は `sml/` 配下のみ書き込み可・git 操作禁止 {#01KYP2JYF6T0FTERAY58NTT6NV}

## 作業パッケージ {#01KYP2JYF6T0FTERAY58NTT6NW}

[id=01KYP2JYF6T0FTERAY58NTT6NX]
依存: WP-Y1 ∥ WP-Y2 ∥ WP-Y3 は独立だが全部が build/typst を触るため**順次実行**。
WP-Y4 は全部の後。

### WP-Y1: alias のグラフ出力(D26) {#01KYP2JYF6T0FTERAY58NTT6NY}

[id=01KYP2JYF6T0FTERAY58NTT6NZ]
- build が解決済みエイリアスを graph JSON に出力する。表現形式は裁量
  (例: `Node.alias: Option<String>` を serde default + skip_serializing_if で
  後方互換に)。既存ゴールデン JSON テストへの影響は「意図的更新」として報告 {#01KYP2JYF6T0FTERAY58NTT6P0}
- テスト: alias 付き/無しノードの JSON 形、往復 {#01KYP2JYF6T0FTERAY58NTT6P1}

### WP-Y2: 子 List ノードの決定的 ID(D27) {#01KYP2JYF6T0FTERAY58NTT6P2}

[id=01KYP2JYF6T0FTERAY58NTT6P3]
- 現状 build 毎に自動生成している子 List ノードの ID を、
  **親リスト項目の ULID + 位置から決定的に導出**(D9 の Term ID 実装を参考に同型で) {#01KYP2JYF6T0FTERAY58NTT6P4}
- テスト: 同一入力の2回 build で全ノード ID が一致すること {#01KYP2JYF6T0FTERAY58NTT6P5}

### WP-Y3: `::record` フェンス(D28)+ 日付・期間セル値型(D29) {#01KYP2JYF6T0FTERAY58NTT6P6}

[id=01KYP2JYF6T0FTERAY58NTT6P7]
1. **strata-sml**: フェンス種別に `record` を追加(`::record {#id alias=...}`)。
   本体は「キー: 値」行の列(キー=行頭から最初の `:` まで、自由テキスト・日本語可、
   前後空白トリム。空キー・`:` 無し行・重複キーは診断 — Error/Warning の別は
   §8.2 と D17 に整合させ裁量・報告)。フェンス内 `#` コメント・空行は既存踏襲。
   fmt はフェンスマーカー行への ID 注入のみ(本体不変、純挿入・冪等の契約維持) {#01KYP2JYF6T0FTERAY58NTT6P8}
2. **strata-core**: `NodePayload::Record`(キーと値の順序保存列)。
   `CellValue::Date { y, m, d: Option }` / `CellValue::Period { from, to: Option }`
   (to 無し=現在)。serde 後方互換に注意 {#01KYP2JYF6T0FTERAY58NTT6P9}
3. **型付きパース(D29)**: 表セルと record 値で共通。既定は ISO
   (`YYYY-MM-DD` / `YYYY-MM`)のみ Date 化、期間は「A 〜 B」「A 〜 現在」
   (`〜`/`~` 両可、前後空白許容)。**書式スニッフィングはしない**。
   フェンス属性 `date-format=` が宣言された場合のみ追加書式を受理
   (最低限 `"YYYY年M月"` と `"YYYY年M月D日"` パターンをサポート。書式言語の
   語彙は最小で裁量・報告)。ISO に見えて不正な値(13月等)は診断 {#01KYP2JYF6T0FTERAY58NTT6PA}
4. **strata-build**: Record ノードのグラフ格納(contains 位置は他フェンスと同様)。
   Date/Period のセル値検証 {#01KYP2JYF6T0FTERAY58NTT6PB}
5. **strata-typst**: Record の標準描画(2列表相当、ラベル `<ULID>` 付与は他ブロックと
   整合)。Date/Period セルの表示(素直に `1997-03` / `2020-10 〜 現在` 形式で可 —
   日本語化はビューの仕事なのでやらない) {#01KYP2JYF6T0FTERAY58NTT6PC}
6. テスト: パース・fmt 冪等・グラフ形・描画・診断(record 3種以上、date/period の
   正常系/異常系/date-format 宣言) {#01KYP2JYF6T0FTERAY58NTT6PD}

### WP-Y4: ドッグフーディング反映と効果実証 {#01KYP2JYF6T0FTERAY58NTT6PE}

[id=01KYP2JYF6T0FTERAY58NTT6PF]
`~/dev/strata-my-resume/sml/` の2文書を新語彙で書き直し、convert_sml.py を追随させる:

[id=01KYP2JYF6T0FTERAY58NTT6PG]
1. **resume.sml**: 「基本情報」「その他」の散文リストを `::record` 化。
   フィールド粒度は**テンプレートのスロットから逆算**(§1.5 の設計原理):
   姓/名/姓読み/名読み を分離、郵便番号/住所/住所読み を分離、生年月日は Date 型。
   **満年齢は削除**(導出値 — convert_sml.py が生年月日と作成日から計算する側に移す)。
   「学歴」「職歴」「免許・資格」は年月を Date 列に持つ `::table` 化
   (職歴のサブ項目(配属等)の表への畳み方は裁量・報告) {#01KYP2JYF6T0FTERAY58NTT6PH}
2. **work_history.sml**: project-index 表に `tech`(主要技術)列を追加し、
   詳細節の「期間|役割|主要技術」メタ行 30 件を削除(表を正とする)。
   期間セルは Period 型に。概略表の行ラベルに埋め込まれた在籍期間も period 列へ分離 {#01KYP2JYF6T0FTERAY58NTT6PJ}
3. **convert_sml.py**: 新グラフ構造(alias・Record・Date/Period・tech 列)に追随。
   v0 の正規表現 F1〜F7(view-v0-handoff の報告参照)が**何件消えたか**を定量報告。
   セレクタも alias 化できる箇所(S1 見出し一致 8 件・S2 caption 一致 2 件)は
   alias 参照に置換(必要なら SML 側の section に alias を付与してよい) {#01KYP2JYF6T0FTERAY58NTT6PK}
4. 再生成と検証: fmt 冪等・build green・JIS PDF 3種再生成、content YAML の
   diff が意図どおりか(年齢が正しく導出されるか含む)、提出版に note/実名が
   無いことの再確認 {#01KYP2JYF6T0FTERAY58NTT6PM}

## 完了の定義 {#01KYP2JYF6T0FTERAY58NTT6PN}

[id=01KYP2JYF6T0FTERAY58NTT6PP]
- `cargo test --workspace` 全通過(新テスト込み)、clippy 新規警告ゼロ
  (strata-html の既存警告は対象外) {#01KYP2JYF6T0FTERAY58NTT6PQ}
- fixture 無変更。ゴールデン JSON/typ の変更は意図的更新のみ(差分報告) {#01KYP2JYF6T0FTERAY58NTT6PR}
- WP-Y4: JIS PDF 3種が新 SML から再現され、正規表現・見出し一致セレクタの削減量が
  定量報告されること {#01KYP2JYF6T0FTERAY58NTT6PS}
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所・削減量・残摩擦を
  まとめて終了 {#01KYP2JYF6T0FTERAY58NTT6PT}

## 既知の注意点 {#01KYP2JYF6T0FTERAY58NTT6PV}

[id=01KYP2JYF6T0FTERAY58NTT6PW]
- `氏名: 080-...` のような非日付値は従来どおり Text フォールバック(診断不要)。
  Date 化は ISO 形か date-format 宣言に合致した時のみ {#01KYP2JYF6T0FTERAY58NTT6PX}
- record のキーに `:` を含めたい場合の扱いは v0 では未対応でよい(最初の `:` で分割) {#01KYP2JYF6T0FTERAY58NTT6PY}
- 概略表(career-overview)の期間分離は履歴書側テンプレートの年/月スロットと
  CV 側の from/to/span スロット両方から逆算すること {#01KYP2JYF6T0FTERAY58NTT6PZ}
- convert_sml.py の実行は `nix develop ~/dev/strata-my-resume -c python3 ...` {#01KYP2JYF6T0FTERAY58NTT6Q0}
