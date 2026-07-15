# ビュー v1 実装ハンドオフ — 宣言的ビュー定義(D30〜D34)

sml-spec §1.6(2026-07-15 確定)の実装指示。v0 のコード製バインディング
(`~/dev/strata-my-resume/sml/convert_sml.py`)を、宣言的ビュー定義ファイル+
決定的実行器 `strata view` に置き換え、JIS 履歴書 PDF の再現で検証する。

## 前提

- M4y 完了(0a8385e、全386テスト green)。graph JSON には alias が出る(D26)、
  `::record` / Date / Period あり(D28/D29)、履歴書2文書は新語彙で記述済み
- v0 の棚卸し(セレクタ頑健度・変換の全種)は `docs/view-v0-handoff.md` と
  convert_sml.py 自体が資料

## 必読(この順)

1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリ)
2. `docs/sml-spec.md` §1.6(D30〜D34)・§1.5(設計原理)・§10
3. `~/dev/strata-my-resume/sml/convert_sml.py` 全体 — **置換対象**。ここにある
   セレクタと変換の全てが、ビュー定義で表現すべき要件の完全な一覧
4. `crates/strata-cli/src/main.rs`(サブコマンドの流儀)、`crates/strata-build`(公開 API)
5. 本書の残り

## スコープ境界(やらないこと)

- LLM によるバインディング提案(v2/M5)・SML 雛形生成(v2)
- SML 記法の変更(強いて必要なら報告のみ。例外: WP-W3 の alias 付与は SML
  **文書**の編集であり可)
- strata-html(凍結。コンパイル維持の最小保守のみ可)
- 正規表現セレクタ・汎用式・スクリプト埋め込み(D31/D32 で禁止)
- fixture(docs/sml_example_*.sml)改版
- `~/dev/strata-my-resume` は `sml/` 配下のみ書き込み可・git 操作禁止

## 作業パッケージ

### WP-W1: strata-view クレート

- ビュー定義(YAML)のパースと、graph(`strata_build::BuildOutput`)への適用
- **セレクタ(D31)**: alias / class / セル座標(row|col パス)/ 型+contains パス
  を一級。見出しテキスト一致は Warning 付きで許可。文法の具体形は裁量 —
  ただし convert_sml.py の全セレクタが書けること
- **コンビネータ(D32)**: rename/pick(record→dict)、rows(表→行 dict 配列、
  列→フィールド対応)、join(子ノード列→区切り文字連結)、date(Date/Period →
  書式文字列。"YYYY" "M" "YYYY年M月" "YY/M" 等、convert_sml.py が現に出力する
  書式を賄う)、age(Date, as-of=セレクタ)、literal(固定値)、class フィルタ。
  **このセット以外は実装しない** — 書けないものが出たら実装せず報告(拡張裁定候補)
- **プロファイル(D34)**: 定義内に profile 宣言、出力の include/exclude を
  class 条件で分岐。1 定義から profile 別の複数出力
- 各スロットは出力先ファイル(-o 起点の相対パス)を宣言できる
  (テンプレートが複数 YAML を読む現実に合わせる)
- 決定的であること: 同一入力・同一定義 → バイト同一出力
- YAML 依存クレートの選定は裁量(serde_yaml は非保守なので代替可・報告)

### WP-W2: CLI `view` サブコマンドとマニフェスト検証(D30/D33)

- `strata view <file.sml> --view <def.yaml> [-o outdir] [--profile <name>] [--check]`
- 内部で build → strata-view 適用。exit code は他コマンドと同じ 0/1/2、
  Warning は stderr + exit 0
- **`--check`(dry-run)**: テンプレート・マニフェスト(ビュー定義から参照する
  YAML。スロット名と必須フィールドの宣言)に対する**未充足スロット**と、
  グラフ側の**未使用ノード**(どのセレクタにも選ばれなかった内容ノード。
  粗い定義で可・裁量)を fmt/build と同じ「行:列(または -:-): 種別: メッセージ」
  流儀で報告
- テスト: セレクタ/コンビネータ単体、profile 分岐、--check の診断、決定性

### WP-W3: 適用と置換検証

1. JIS テンプレート2種(履歴書・CV)の**マニフェストを手書きで起こす**
   (`~/dev/strata-my-resume/sml/` 配下。テンプレ .typ が実際に読むスロットと
   フィールドの宣言。display_timeline() のようなテンプレ内ハードコードも
   マニフェストに注記)
2. **ビュー定義を作成**(例: `sml/views/resume-jis.view.yaml` /
   `cv-jis.view.yaml`)。convert_sml.py の全出力(content/*.yaml、
   submit/check 両 profile)を宣言で再現
3. **M4y 持ち越し(V1-7)**: work_history.sml のプロジェクト詳細節(H4)に
   `proj-*` alias を付与し、project-index の行 key と一致させる。
   ビュー定義は文書順対応ではなく alias で結ぶ(位置依存の根絶)。fmt 冪等維持
4. convert_sml.py を**廃止または PDF ビルドだけの薄いシェルに縮退**(裁量・報告)。
   `strata view` → typst compile で JIS PDF 3種を再生成
5. 合格基準: content YAML が convert_sml.py 出力と一致(意図的差分は判定付き
   報告)、PDF 再現、提出版に note/実名なし、`--check` が現状ゼロ診断
   (わざとスロットを外した negative テストも1つ)

### WP-W4: ビュー定義文法の起草

- 実装した文法を `docs/view-def-v1.md` として起草(次の対話で批准予定)。
  文法・セレクタ・コンビネータ・profile・マニフェストの全てを実例付きで。
  **「読んで分かる」ことが v1 の品質基準**(LLM 提案・人間承認のレビュー対象)

## 完了の定義

- `cargo test --workspace` 全通過(新テスト込み)、clippy 新規警告ゼロ
  (strata-html 既存分は対象外)
- 既存コマンド(fmt/build/render)非退行、fixture 無変更
- WP-W3 の合格基準達成、WP-W4 の文書起草
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所・
  「コンビネータで書けなかったもの」(拡張裁定候補)・ビュー定義の可読性
  自己評価をまとめて終了

## 既知の注意点

- 年齢計算の as-of は resume の record(作成日)から引く(ハードコードしない)
- 出力 YAML は Typst の `yaml()` が読む — 既存 content YAML と同じ構造・
  クォート習慣に合わせ、diff 可能性を保つ
- ビュー定義・マニフェストの置き場所は `sml/` 配下(リポジトリ制約)
- convert_sml.py の挙動のうち「バインディングがデータを持たない」原則に反して
  残っていたもの(environments のフォールバック値・skills の案内文言)は
  literal コンビネータで**ビュー定義側に明示**する(スクリプトの闇からデータの
  宣言へ — これが v1 の思想的な意味)
