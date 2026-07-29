---
id: 01KYP2JYFTGW2WW1EMMWES8CVZ
title: "ビュー/テンプレート層 v0 実装ハンドオフ — バインディング=コード(pull 型の実証)"
alias: view-v0-handoff
---

# ビュー/テンプレート層 v0 実装ハンドオフ — バインディング=コード(pull 型の実証) {#01KYP2JYFTGW2WW1EMMWES8CW0}

[id=01KYP2JYFTGW2WW1EMMWES8CW1]
本書は sml-spec §10「ビュー/テンプレート層」の段階案 v0 の作業指示。
**目的は JIS 履歴書テンプレートが SML ソースから埋まることの実証**と、それ以上に
**「実際に必要だったグラフセレクタと整形処理の棚卸し」**(v1 宣言的ビュー定義の設計材料)。

## アーキテクチャ(2026-07-15 壁打ち合意) {#01KYP2JYFTGW2WW1EMMWES8CW2}

``` {#01KYP2JYFTGW2WW1EMMWES8CW3}
sml/*.sml → strata build(graph JSON)→ 抽出器スクリプト(=バインディング v0)
          → content/*.yaml → 既存 Typst テンプレート(無改変のコピー)→ JIS 履歴書 PDF
```

[id=01KYP2JYFTGW2WW1EMMWES8CW4]
- バインディングはデータを持たない純粋な対応表。データは常に文書(SML)側 {#01KYP2JYFTGW2WW1EMMWES8CW5}
- テンプレートと strata 本体(クレート)は一切変更しない {#01KYP2JYFTGW2WW1EMMWES8CW6}

## 対象リポジトリと制約 {#01KYP2JYFTGW2WW1EMMWES8CW7}

[id=01KYP2JYFTGW2WW1EMMWES8CW8]
- 作業先: `~/dev/strata-my-resume` — **書き込みは `sml/` ディレクトリ配下のみ**。
  リポジトリルートの既存ファイル(*.yaml, main.typ, template/, build_resume/,
  build_cv/, *.pdf 等)は読取専用。テンプレートは `sml/` 配下へ**コピーして**使う {#01KYP2JYFTGW2WW1EMMWES8CW9}
- `~/dev/strata` — 変更禁止(strata-cli は `cargo run -q -p strata-cli --` で使うだけ) {#01KYP2JYFTGW2WW1EMMWES8CWA}
- **git 操作(add/commit/push)は両リポジトリとも絶対にしない** {#01KYP2JYFTGW2WW1EMMWES8CWB}

## 必読(この順) {#01KYP2JYFTGW2WW1EMMWES8CWC}

[id=01KYP2JYFTGW2WW1EMMWES8CWD]
1. `~/dev/strata-my-resume/scripts/convert.py` — 現行の YAML→content 抽出器。
   **スロットスキーマの正典**(何をどの形で出せばテンプレートが動くか) {#01KYP2JYFTGW2WW1EMMWES8CWE}
2. `~/dev/strata-my-resume/main.typ` と `template/*.typ`(履歴書)、
   `cv/main.typ`(無ければ `build_cv/main.typ` と `main_check.typ`)と
   `cv/template/*.typ`(職務経歴書)— スロットの消費側 {#01KYP2JYFTGW2WW1EMMWES8CWF}
3. 既存の `build_resume/content/*.yaml` / `build_cv/content/*.yaml` —
   convert.py の出力実例(**v0 の合格基準の比較対象**) {#01KYP2JYFTGW2WW1EMMWES8CWG}
4. `~/dev/strata-my-resume/sml/resume.sml` / `work_history.sml` — 変換元(fmt 済み) {#01KYP2JYFTGW2WW1EMMWES8CWH}
5. [D23](ref:decisions/d23)〜[D25](ref:decisions/d25)・[ビュー/テンプレート層(保留)](ref:grammar/frozen-vs-pending) {#01KYP2JYFTGW2WW1EMMWES8CWJ}

## 作業内容 {#01KYP2JYFTGW2WW1EMMWES8CWK}

### WP-V1: 抽出器スクリプト {#01KYP2JYFTGW2WW1EMMWES8CWM}

[id=01KYP2JYFTGW2WW1EMMWES8CWN]
- `sml/convert_sml.py`(言語は Python 推奨。PyYAML が無ければ
  `nix develop ~/dev/strata-my-resume -c python3 ...` で実行するか、依存無しの
  素朴な YAML 出力を自前実装 — 裁量、報告) {#01KYP2JYFTGW2WW1EMMWES8CWP}
- 入力: `strata build sml/resume.sml -o ...json` / `sml/work_history.sml -o ...json`
  の canonical graph JSON(**SML を直接パースしない**。グラフ経由が v0 の要点) {#01KYP2JYFTGW2WW1EMMWES8CWQ}
- 出力: `sml/build_resume/content/*.yaml` と `sml/build_cv/content/*.yaml`
  (convert.py の出力と同スキーマ) {#01KYP2JYFTGW2WW1EMMWES8CWR}
- 提出版/確認版(companies.yaml vs companies_check.yaml 等)は **`Node.classes`
  (note)を抽出器が解釈**して出し分ける(render --hide は使わない — テンプレート
  ビューではバインディングがビュー) {#01KYP2JYFTGW2WW1EMMWES8CWS}

### WP-V2: テンプレート適用とビルド {#01KYP2JYFTGW2WW1EMMWES8CWT}

[id=01KYP2JYFTGW2WW1EMMWES8CWV]
- `sml/build_resume/` に `main.typ` + `template/` をコピー、
  `sml/build_cv/` に CV 側の main/main_check + template をコピー(無改変) {#01KYP2JYFTGW2WW1EMMWES8CWW}
- `typst compile` で PDF 生成(出力も `sml/` 配下、例: `sml/resume_jis.pdf`,
  `sml/work_history_jis.pdf`, `sml/work_history_jis_check.pdf`) {#01KYP2JYFTGW2WW1EMMWES8CWX}
- フォント(HackGen 等)が現環境に無い場合は
  `nix develop ~/dev/strata-my-resume -c typst compile ...` を試す。
  それでも不可なら compile はスキップし .typ 生成までで報告 {#01KYP2JYFTGW2WW1EMMWES8CWY}

### WP-V3: 検証 {#01KYP2JYFTGW2WW1EMMWES8CWZ}

[id=01KYP2JYFTGW2WW1EMMWES8CX0]
- **合格基準**: `sml/build_*/content/*.yaml` が既存 `build_*/content/*.yaml` と
  意味的に一致(diff を取り、差分は1件ずつ「SML 化時の意図的変更(匿名化・
  ネスト構造化等)による」か「抽出器のバグ」かを判定して報告) {#01KYP2JYFTGW2WW1EMMWES8CX1}
- 生成 PDF が既存 `resume.pdf` / `work_history*.pdf` と同等の内容であること {#01KYP2JYFTGW2WW1EMMWES8CX2}

## 最終報告に含めること(v1 設計の材料 — 最重要) {#01KYP2JYFTGW2WW1EMMWES8CX3}

[id=01KYP2JYFTGW2WW1EMMWES8CX4]
1. **セレクタ棚卸し**: グラフから情報を引くのに使った手段の全列挙
   (見出しテキスト一致 / alias / class / ノード型 / 表セル座標 / リスト位置 /
   正規表現パース 等)と、それぞれの件数・壊れやすさの所感 {#01KYP2JYFTGW2WW1EMMWES8CX5}
2. **整形処理の棚卸し**: 文字列パースが必要だった箇所の全列挙
   (年月分割・姓名分割・「キー: 値」分解・期間パース等)と、SML 側を
   どう構造化すれば不要になるかの提案 {#01KYP2JYFTGW2WW1EMMWES8CX6}
3. **未充足スロット/未使用情報**: テンプレートが要求するのにグラフから
   引けなかったもの、グラフにあるのにどのスロットにも入らなかったもの {#01KYP2JYFTGW2WW1EMMWES8CX7}
4. content YAML の diff 判定結果、PDF 生成の成否 {#01KYP2JYFTGW2WW1EMMWES8CX8}
5. 作成ファイル一覧、裁量で決めた箇所 {#01KYP2JYFTGW2WW1EMMWES8CX9}

## 完了の定義 {#01KYP2JYFTGW2WW1EMMWES8CXA}

[id=01KYP2JYFTGW2WW1EMMWES8CXB]
- JIS 履歴書 PDF(+CV 2種)が SML ソースだけから再現される(フォント問題で
  compile 不可の場合は content YAML の一致まで) {#01KYP2JYFTGW2WW1EMMWES8CXC}
- 既存ファイル無変更(`git status` で `sml/` 以外に変化が無いこと) {#01KYP2JYFTGW2WW1EMMWES8CXD}
- コミットしない。上記報告をまとめて終了 {#01KYP2JYFTGW2WW1EMMWES8CXE}
