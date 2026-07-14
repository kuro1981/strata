# M4x 実装ハンドオフ — ドッグフーディング裁定(D23〜D25)の実装

本書は M4 完了後のドッグフーディング(実履歴書 2 文書・30 プロジェクトの SML 化、
`~/dev/strata-my-resume/sml/`)で顕在化した摩擦点への裁定 D23〜D25
(`sml-spec.md` §1.4、2026-07-14 対話にて確定)を実装に落とし込む自己完結な作業指示。

## 前提(M4 完了時点の状態)

- パイプライン `fmt → build → render → typst compile` は一本通っている(全300テスト green)
- `strata-cli`: `fmt` / `build` / `render` の3サブコマンド
- `strata-typst`: M3 語彙対応済み。ゴールデン `docs/sml_example_formatted.typ`
- ドッグフーディング成果物: `~/dev/strata-my-resume/sml/{resume,work_history}.sml`
  (fmt 済み・build/render 通過済み)。既知の内容: 面接メモは 【補足】 で始まる段落、
  実名は匿名名称直後の 【実際: 〇〇】(インライン)、ネストしたかった箇所は
  「。」連結で平坦化してある

## 必読(この順)

1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリとも)
2. `docs/sml-spec.md` §1.4(D23〜D25)・§4.1(`class` キー追加済み)・§10
3. `crates/strata-sml/src/`(パーサ・fmt)、`crates/strata-build/src/lib.rs`、
   `crates/strata-core/src/lib.rs`、`crates/strata-typst/src/lib.rs`、
   `crates/strata-cli/src/main.rs`
4. 本書の残り全部

## スコープ境界(やらないこと)

- インラインの出し分け記法(保留 §10)・ビュー定義ファイル・テンプレート pull 型
  レンダリング(保留 §10)はやらない
- 「リスト項目=段落1つ」の制約解除(項目内複数ブロック)はやらない(D24 はネストのみ)
- SML への組版ヒント構文の追加はやらない(D25)
- strata-html は凍結のまま触らない
- fixture `docs/sml_example_draft.sml` / `sml_example_formatted.sml` は**改版しない**
  (ゴールデン `.typ` は WP-X3 の組版変更で更新してよい — 意図的更新として報告)

## 設計確定事項の実装展開

### D-X1: `class` 属性の全層対応(D23)

1. **strata-sml**: 属性行キーに `class` を追加。値は key 字句(`[A-Za-z0-9_-]+`)の
   タグ、単一または `[a, b]` リスト。`UnknownAttrKey` Warning の対象から外す。
   fmt は class を保存し既存契約(純挿入・冪等)を壊さない。
   属性行が現状プローズ・リスト以外(見出し・フェンス)に付けられない場合、
   class を書ける場所をどう確保するかは裁量(例: フェンスはフェンス内属性行
   `[class=note]` を許可)— 決定内容を報告
2. **strata-core**: `Node` に `classes: Vec<String>` を追加。serde は
   `#[serde(default, skip_serializing_if = "Vec::is_empty")]` で後方互換
3. **strata-build**: class を検証(字句違反は BuildError)して Node.classes に格納。
   build の成否・グラフ構造は class に**非依存**(全ノード格納)
4. **strata-cli / strata-typst**: `render --hide <class>`(複数指定可、例:
   `--hide note --hide actual-name`)。該当 class を1つでも持つブロックノードを
   **contains サブツリーごと**非描画。非描画ノードへの `Ref` は Warning
   (stderr、exit code に影響なし)を出しつつリンクを剥がしてプレーンテキスト化
   (表示 text があれば text、無ければ短い代替表記)。render の警告返却方法
   (`RenderOutput { text, warnings }` 等)は裁量 — fmt/build の Warning 表示形式
   (「行:列: warning: ...」相当。ソース位置が無い場合の形式は裁量)に揃える

### D-X2: ネストリスト(D24)

1. **strata-sml パーサ**: リスト項目のインデントによる子リスト(2スペース/レベル、
   `-` と `1.` の混在可)をパース。ネスト深さは制限なし(実用上の上限は裁量)。
   リストとして解釈できないインデント行(例: インデント幅が不正)は**診断を出す**
   — 従来の「無警告で `- ` 混じりの別段落に化ける」挙動を根絶する。
   Error か Warning かは全か無かの原則(§8.2)との整合で判断し報告
2. **fmt**: ネスト項目にも行末 `{#ULID}` を注入。純挿入・冪等の契約維持
3. **strata-build**: 子リストを `List` ノードとして親項目(または親 List)に
   contains で接続。ord 順序維持。既存の平坦リストのグラフ表現は変えない
4. **strata-typst**: ネストを Typst のネストリストとして描画。ラベル付与は
   既存の `#block[...] <ULID>` 方式と整合させる

### D-X3: レンダラ組版改善(D25)

対象は strata-typst のみ(SML・グラフのスキーマ変更なし):

1. 表 figure の **breakable 化**(`#show figure: set block(breakable: true)` 等)
   — 表前の空白ページ解消もこれで確認
2. **列幅戦略**: 長文テキスト列が幅ゼロに潰れて1文字縦書きになる問題の解消
   (例: 列幅を `auto` から fr 単位・上限付き auto へ。手段は裁量)
3. 改ページで行内容が次ページと重なる問題の解消(1 と同根の可能性が高い)
4. **合格基準**: `~/dev/strata-my-resume/sml/work_history.sml` の 30 行ネスト表
   (project-index)が「読める・重ならない・空白ページなし」で PDF 化されること

### D-X4: ドッグフーディング再適用(検証を兼ねる)

`~/dev/strata-my-resume/sml/` の 2 文書を D23/D24 対応の形に更新して再生成:

1. 【補足】 段落に `[class=note]` を付与。インラインの 【実際: 〇〇】 は本文から
   抜き、該当プロジェクトの note ブロック側へ移す(例: 【補足】 段落の冒頭に
   「実際の客先: 〇〇。」)— インライン出し分けが無い v0 での既知の妥協として報告
2. 「。」連結で平坦化していた箇所を D24 のネストリストに戻す(元 YAML
   `~/dev/strata-my-resume/work_history.yaml` の構造が原本)
3. 再生成(出力はすべて `sml/` 内、既存の repo ルートの PDF 等は上書き禁止):
   - 確認版: `render`(--hide なし)→ `sml/work_history_check.pdf` / `sml/resume_check.pdf`
   - 提出版: `render --hide note` → `sml/work_history.pdf` / `sml/resume.pdf`
   - 提出版 PDF に 【補足】・実名が残っていないこと、確認版に全部あることを確認
4. `~/dev/strata-my-resume` では git 操作禁止・`sml/` 以外への書き込み禁止

## 作業パッケージ分割

依存: WP-X1(class)∥ WP-X2(ネストリスト)は理論上並列可だが、両方が
sml パーサ・fmt・build・typst を触るため**同一エージェントで順次実行**を推奨。
WP-X3(組版)は typst のみで独立。WP-X4 は全部の後。

- **WP-X1**: D-X1 の全部 + 単体テスト(class パース・fmt 保存・グラフ格納・
  --hide サブツリー・隠し Ref の Warning とテキスト化)
- **WP-X2**: D-X2 の全部 + 単体テスト(2〜3 段ネストのパース/fmt 冪等/グラフ形状/
  描画、不正インデントの診断)+ 旧誤パース入力が診断になる回帰テスト
- **WP-X3**: D-X3 の全部。ゴールデン `.typ` の意図的更新可(差分を報告)
- **WP-X4**: D-X4 の全部。摩擦の再評価(直ったか・新たな摩擦はないか)を報告

## 完了の定義

- 全 WP のテスト込みで `cargo test --workspace` 通過
- `cargo clippy --workspace --all-targets` で新規・変更コードの警告ゼロ
  (凍結中 strata-html の既存警告は対象外)
- fmt の契約(純挿入・冪等・全か無か)の非退行
- `.sml` fixture 無変更。ゴールデン `.typ` の変更は意図的更新のみ
- 提出版/確認版の 4 PDF が `~/dev/strata-my-resume/sml/` に生成され、出し分けが正しい
- **コミットはしない**。変更ファイル一覧・テスト消化状況・裁量で決めた箇所・
  仕様の曖昧点(勝手に解釈せず報告)をまとめて終了

## 既知の注意点

- 旧ゴールデン JSON(strata-build のテスト)に `classes` が無い前提 —
  `skip_serializing_if` で既存ゴールデンを壊さないこと
- ネストリストの診断は「今まで通っていた文書が通らなくなる」変更。既存 fixture・
  ドッグフーディング文書に影響が無いことを確認(平坦化済みなので無いはず)
- `--hide` でルート直下の Section が消える場合、目次的に空になっても正常動作すること
- typst compile は PATH の typst(0.14.x)で確認。フォント警告(variable fonts)は既知・無害
