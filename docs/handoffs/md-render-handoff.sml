---
id: 01KYP2JYEBP1XGKVF9PXW8DCYF
title: "MD レンダラ実装ハンドオフ — render --format md(D38/D39)"
alias: md-render-handoff
---

# MD レンダラ実装ハンドオフ — `render --format md`(D38/D39) {#01KYP2JYEBP1XGKVF9PXW8DCYG}

[id=01KYP2JYEBP1XGKVF9PXW8DCYH]
sml-spec §1.8(2026-07-15 確定)の実装指示。人間向けの最小依存ビューとして
素の Markdown(GFM)出力を追加する。

[id=01KYP2JYEBP1XGKVF9PXW8DCYJ]
**前提(D39)**: SML は素の Markdown の上位互換であることが原則(「MD でできる
ことが全部できないと片手落ち」)。そのため **WP-M0(互換性監査)を先行**させ、
結果表のユーザー裁定を経てから WP-M1〜M3 に着手する。

## WP-M0: CommonMark/GFM 互換性監査(先行・読み取り専用) {#01KYP2JYEBP1XGKVF9PXW8DCYK}

[id=01KYP2JYEBP1XGKVF9PXW8DCYM]
CommonMark コア+GFM 拡張の全機能を列挙し、それぞれを現行の `strata fmt` /
`strata build` に食わせて挙動を分類した**互換性マトリクス**を作る:

[id=01KYP2JYEBP1XGKVF9PXW8DCYN]
- 対象(最低限): 見出し(ATX/Setext)、段落、強調/強い強調/コード/打ち消し、
  外部リンク・参照スタイルリンク・autolink、インライン画像、エスケープ(`\*` 等)、
  blockquote、水平線(`---`。フロントマターとの衝突も確認)、GFM パイプ表、
  タスクリスト、脚注、HTML ブロック/インライン HTML、ハードブレイク(行末2スペース
  /`\`)、リンク内の書式、リスト(番号付き開始値、ゆるい/かたいリスト) {#01KYP2JYEBP1XGKVF9PXW8DCYP}
- 分類: ①対応済み(グラフに正しく落ちる)/②**静かに壊れる**(無診断で情報が
  落ちる・化ける — 最危険)/③診断が出る/④未対応(素通しでテキスト扱い) {#01KYP2JYEBP1XGKVF9PXW8DCYQ}
- 各項目に最小の再現入力と実挙動を添える。②は特に詳細に {#01KYP2JYEBP1XGKVF9PXW8DCYR}
- 成果物はレポートのみ(コード変更なし)。裁定はユーザーが行う {#01KYP2JYEBP1XGKVF9PXW8DCYS}

## WP-M1〜M3 は WP-M0 の裁定後に着手(以下は当初案。裁定で修正されうる) {#01KYP2JYEBP1XGKVF9PXW8DCYT}

## 必読(この順) {#01KYP2JYEBP1XGKVF9PXW8DCYV}

[id=01KYP2JYEBP1XGKVF9PXW8DCYW]
1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリ) {#01KYP2JYEBP1XGKVF9PXW8DCYX}
2. [D38](ref:decisions/d38)・[D19](ref:decisions/d19)/[D22](ref:decisions/d22)(との関係)・[D23](ref:decisions/d23)(--hide) {#01KYP2JYEBP1XGKVF9PXW8DCYY}
3. `crates/strata-typst/src/lib.rs`(render の既存流儀・--hide の実装)、
   `crates/strata-context/src/`(インライン変換・ラベルの既存実装 — 共有候補)、
   `crates/tex2math/src/`(MathNode 定義。逆直列化の対象) {#01KYP2JYEBP1XGKVF9PXW8DCYZ}
4. `crates/strata-cli/src/main.rs`(render サブコマンド) {#01KYP2JYEBP1XGKVF9PXW8DCZ0}

## スコープ境界(やらないこと) {#01KYP2JYEBP1XGKVF9PXW8DCZ1}

[id=01KYP2JYEBP1XGKVF9PXW8DCZ2]
- ファイル横断リンク(§10 ワークスペース層 — 次の設計対話) {#01KYP2JYEBP1XGKVF9PXW8DCZ3}
- HTML 埋め込みによる rowspan/colspan 再現(素の GFM に徹する) {#01KYP2JYEBP1XGKVF9PXW8DCZ4}
- strata-html(凍結)・fixture 改版・既存コマンド非退行 {#01KYP2JYEBP1XGKVF9PXW8DCZ5}
- `~/dev/strata-my-resume` は `sml/` 配下のみ書き込み可・git 操作禁止 {#01KYP2JYEBP1XGKVF9PXW8DCZ6}

## 作業パッケージ {#01KYP2JYEBP1XGKVF9PXW8DCZ7}

### WP-M1: MathNode → TeX 逆直列化 {#01KYP2JYEBP1XGKVF9PXW8DCZ8}

[id=01KYP2JYEBP1XGKVF9PXW8DCZ9]
- `tex2math` に `to_tex(&MathNode) -> String` を追加(パーサの逆) {#01KYP2JYEBP1XGKVF9PXW8DCZA}
- **round-trip テスト**: `parse(to_tex(parse(s))) == parse(s)` を既存対応コマンド
  全種(\frac、Sub/Sup、\sqrt[n]、大型演算子、\text、\left\right、ギリシャ文字、
  \hat 等)で。文字列の完全復元は要求しない(構造同値でよい) {#01KYP2JYEBP1XGKVF9PXW8DCZB}
- 括弧の要不要・空白の入れ方は裁量(TeX として正しく再パースできること) {#01KYP2JYEBP1XGKVF9PXW8DCZC}

### WP-M2: MD レンダラ(D38) {#01KYP2JYEBP1XGKVF9PXW8DCZD}

[id=01KYP2JYEBP1XGKVF9PXW8DCZE]
- 置き場所は裁量(新クレート strata-md 推奨。strata-context の inline 変換を
  共有できるなら公開して再利用 — 重複実装するなら理由を報告) {#01KYP2JYEBP1XGKVF9PXW8DCZF}
- ブロック変換: {#01KYP2JYEBP1XGKVF9PXW8DCZG}
  - 見出し=`#`〜、段落=素のテキスト、リスト=GFM リスト(ネスト対応)、
    コード=フェンス {#01KYP2JYEBP1XGKVF9PXW8DCZH}
  - **`{#ULID}` タグ・alias・エッジは一切出さない**(context との役割分担) {#01KYP2JYEBP1XGKVF9PXW8DCZJ}
  - record = 2列 GFM 表(キー | 値) {#01KYP2JYEBP1XGKVF9PXW8DCZK}
  - **多次元表 = GFM 表へ平坦化**: ネスト行次元は親ラベルを各行に繰り返し
    (MultiIndex 方式)、ネスト列次元はパス連結ヘッダ(例 `Dataset-A / F1`)。
    caption は表の直前に `**表: {caption}**` 等(体裁裁量) {#01KYP2JYEBP1XGKVF9PXW8DCZM}
  - 数式 = `$...$`(インライン)/ `$$...$$`(ブロック、WP-M1 の to_tex) {#01KYP2JYEBP1XGKVF9PXW8DCZN}
  - figure: chart = depicts テキストのプレースホルダ引用、image = `![alt](src)` {#01KYP2JYEBP1XGKVF9PXW8DCZP}
  - Date/Period セル = `1997-03` / `2020-10 〜 現在`(typst と同じ素直表示) {#01KYP2JYEBP1XGKVF9PXW8DCZQ}
- 参照(D38): 見出しへの Ref = GFM アンカーリンク(`[text](#見出しアンカー)`、
  GitHub のアンカー生成規則に合わせる。日本語見出しのアンカー化規則は裁量・報告)。
  表・数式・段落など = `text(表: キャプション)` 形式のテキスト退化(体裁裁量、
  黙って落とさない) {#01KYP2JYEBP1XGKVF9PXW8DCZR}
- `--hide <class>`(D23): typst 側と同じサブツリー非描画+隠し Ref は
  Warning+テキスト化。warnings の返し方も typst 側 `RenderOutput` と揃える {#01KYP2JYEBP1XGKVF9PXW8DCZS}

### WP-M3: CLI 統合とドッグフーディング {#01KYP2JYEBP1XGKVF9PXW8DCZT}

[id=01KYP2JYEBP1XGKVF9PXW8DCZV]
- `render --format <typst|md>`(既定 typst、D19 改定)。`-o` の拡張子は
  ユーザー指定に従う(自動推測しない) {#01KYP2JYEBP1XGKVF9PXW8DCZW}
- テスト: fixture ゴールデン(`docs/sml_example_formatted.md` 新規)+
  要素別単体(表平坦化・record・数式・Ref 退化・--hide) {#01KYP2JYEBP1XGKVF9PXW8DCZX}
- ドッグフーディング: `~/dev/strata-my-resume/sml/work_history.sml` を
  提出版(--hide note)/確認版で MD 出力(`sml/work_history.md` /
  `work_history_check.md`)。**30行×2階層の project-index 表が GFM 表として
  読めるか**を自己評価(平坦化の見栄えが D38 の残論点)。resume.sml も同様 {#01KYP2JYEBP1XGKVF9PXW8DCZY}

## 完了の定義 {#01KYP2JYEBP1XGKVF9PXW8DCZZ}

[id=01KYP2JYEBP1XGKVF9PXW8DD00]
- `cargo test --workspace` 全通過(round-trip・ゴールデン込み)、clippy 新規警告
  ゼロ(strata-html 既存分対象外)、fixture 無変更、既存コマンド非退行 {#01KYP2JYEBP1XGKVF9PXW8DD01}
- MD 出力2文書が生成され、提出版に note/実名が無いこと {#01KYP2JYEBP1XGKVF9PXW8DD02}
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所(アンカー規則・
  平坦化の細部・to_tex の括弧方針)・平坦化見栄えの自己評価・残摩擦をまとめて終了 {#01KYP2JYEBP1XGKVF9PXW8DD03}
