---
id: 01KYP2T2BFK7MB88F2HYTRQCSD
title: "M8 実装ハンドオフ — 設計文書の自己 SML 化(D47)"
alias: spec-sml-m8-handoff
---

# M8 実装ハンドオフ — 設計文書の自己 SML 化(D47) {#01KYP2T2BFK7MB88F2HYTRQCSE}

[id=01KYP2T2BFK7MB88F2HYTRQCSF]
sml-spec §1.12(2026-07-15 確定)の実行指示。コード変更は無し(または最小)—
**変換・エッジ付与・検証・摩擦収集**の作業。狙いは (1) 意味エッジの実戦データ、
(2) 仕様書ジャンルでの摩擦収集、(3) 設計対話の context 武装、(4) 将来のグラフ UI の
実データ。

## 最重要 {#01KYP2T2BFK7MB88F2HYTRQCSG}

[id=01KYP2T2BFK7MB88F2HYTRQCSH]
- **正典は(旧)`sml-spec.md`(現 docs/spec-sml/decisions.sml + grammar.sml)のまま。md を1バイトも変更しない**。
  SML 版は派生実験(移行判断は摩擦レポート後に別途) {#01KYP2T2BFK7MB88F2HYTRQCSJ}
- **git commit/push はしない** {#01KYP2T2BFK7MB88F2HYTRQCSK}
- [sml-agent-guide](doc:sml-agent-guide)(AI 執筆ガイド)の作法に従うこと — 特に
  ULID 不発明・エッジの確信原則・コンテナ class(D46) {#01KYP2T2BFK7MB88F2HYTRQCSM}

## 必読(この順) {#01KYP2T2BFK7MB88F2HYTRQCSN}

[id=01KYP2T2BFK7MB88F2HYTRQCSP]
1. `AGENTS.md` {#01KYP2T2BFK7MB88F2HYTRQCSQ}
2. [sml-agent-guide](doc:sml-agent-guide) — あなたの執筆作法の正典 {#01KYP2T2BFK7MB88F2HYTRQCSR}
3. [decisions.sml](doc:decisions) 全体(変換元)と[D47](ref:decisions/d47) {#01KYP2T2BFK7MB88F2HYTRQCSS}
4. [view-def-v1](doc:view-def-v1) は不要。CLI は `cargo run -q -p strata-cli --`(cwd: リポジトリルート) {#01KYP2T2BFK7MB88F2HYTRQCST}

## 作業パッケージ {#01KYP2T2BFK7MB88F2HYTRQCSV}

### WP-S1: 変換 {#01KYP2T2BFK7MB88F2HYTRQCSW}

[id=01KYP2T2BFK7MB88F2HYTRQCSX]
- 置き場所: `docs/spec-sml/` を新設 — `decisions.sml`(§1 の裁定群)+
  `strata.toml`(members は当面 decisions.sml のみ。将来 §2 以降や原理編を
  足せる形) {#01KYP2T2BFK7MB88F2HYTRQCSY}
- 構造: {#01KYP2T2BFK7MB88F2HYTRQCSZ}
  - フロントマター: title「SML 設計決定の記録」+ 文書 alias `decisions` {#01KYP2T2BFK7MB88F2HYTRQCT0}
  - H1 = 文書タイトル、H2 = マイルストーン節(§1.1〜§1.12 に対応。
    「M4(render)設計決定(2026-07-14)」等。alias 例: `m4-render`) {#01KYP2T2BFK7MB88F2HYTRQCT1}
  - **H3 = 1裁定**(「D23 出し分けの層分離」等。**alias は `d1`〜`d47`、
    `p1`〜`p4`**)。本文は原文の表セル(論点/裁定)を prose 段落に展開:
    第1段落=論点、以降=裁定本文。太字・箇条書き等の原文の強調構造は保持 {#01KYP2T2BFK7MB88F2HYTRQCT2}
  - 各マイルストーン節の前文(§1.4 の定性評価等)はその節の導入段落として保持 {#01KYP2T2BFK7MB88F2HYTRQCT3}
  - **情報欠落ゼロ**: 原文の全文言を保つ(prose 化に伴う接続詞の追加は可、
    削除・要約は不可)。取り消し線等の md 記法もそのまま(M6 で対応済み) {#01KYP2T2BFK7MB88F2HYTRQCT4}
- ドラフト → `fmt` → 冪等確認、はガイドどおり {#01KYP2T2BFK7MB88F2HYTRQCT5}

### WP-S2: エッジ付与(本ミッションの核心) {#01KYP2T2BFK7MB88F2HYTRQCT6}

[id=01KYP2T2BFK7MB88F2HYTRQCT7]
D37 の確信原則で:

[id=01KYP2T2BFK7MB88F2HYTRQCT8]
1. **inline ref(ナビ)**: 本文が他の裁定を明示的に名指しする箇所
   (「D9 の Term 方式と同型」「D23 の継承」等)は `[D9](ref:d9)` の
   インライン参照にする。全裁定横断で機械的に(Dn/Pn の言及を漏らさない) {#01KYP2T2BFK7MB88F2HYTRQCT9}
2. **depends-on(意味エッジ)**: 本文が**依拠**を明文で語る場合のみ
   (「〜と同型で導出」「〜に従い」「〜の再裁定」「〜を改定」)。
   単なる言及・対比は張らない。1裁定に複数可 {#01KYP2T2BFK7MB88F2HYTRQCTA}
3. **境界事例は張らずに最終報告へ列挙**(「D40 は D39 の実装裁定なので
   depends-on が自然に見えるが、本文の文言は…」のような形で人間の裁定に回す) {#01KYP2T2BFK7MB88F2HYTRQCTB}
4. 改定関係(D11 が M1 実装を改定、D35 が D32 の運用初適用等)を depends-on と
   別 rel にしたくなったら、**新 rel は発明せず**摩擦として報告(rel 語彙は
   strata-spec 側の凍結事項) {#01KYP2T2BFK7MB88F2HYTRQCTC}

### WP-S3: 検証とデモ {#01KYP2T2BFK7MB88F2HYTRQCTD}

[id=01KYP2T2BFK7MB88F2HYTRQCTE]
1. `fmt --check` 冪等・`build --workspace docs/spec-sml/strata.toml` 診断ゼロ {#01KYP2T2BFK7MB88F2HYTRQCTF}
2. `render --format md` / `--format typst`(+typst compile)が通る —
   出力 md が「読める設計決定文書」になっているか自己評価 {#01KYP2T2BFK7MB88F2HYTRQCTG}
3. **デモ2本**(最終報告に出力を引用): {#01KYP2T2BFK7MB88F2HYTRQCTH}
  - `context --node d23 --hops 1` — D23 の近傍(依存してくる裁定・D23 が
     依拠する裁定)が1チャンクで出ること {#01KYP2T2BFK7MB88F2HYTRQCTJ}
  - 「D32 に依存する裁定の一覧」を統合グラフの JSON から抽出
     (エッジの逆引き。方法は裁量 — grep でよい) {#01KYP2T2BFK7MB88F2HYTRQCTK}

### WP-S4: 摩擦レポート(第二ジャンルの収穫) {#01KYP2T2BFK7MB88F2HYTRQCTM}

[id=01KYP2T2BFK7MB88F2HYTRQCTN]
履歴書との違いから来る摩擦を重点的に:

[id=01KYP2T2BFK7MB88F2HYTRQCTP]
- 長い論証 prose・裁定表 → prose 変換の書き味 {#01KYP2T2BFK7MB88F2HYTRQCTQ}
- 「裁定が裁定を改定する」「保留 → 解消」のような**時間発展**の表現
  (現状の rel 語彙で足りたか) {#01KYP2T2BFK7MB88F2HYTRQCTR}
- 見出し=裁定名の粒度は適切だったか(1裁定内の複数論点) {#01KYP2T2BFK7MB88F2HYTRQCTS}
- §10(保留リスト)や設計原理のような「裁定でないブロック」の扱い {#01KYP2T2BFK7MB88F2HYTRQCTT}
- 正典移行の判断材料: SML 版を正典にした場合に困ること・md に残したい理由 {#01KYP2T2BFK7MB88F2HYTRQCTV}

## 完了の定義 {#01KYP2T2BFK7MB88F2HYTRQCTW}

[id=01KYP2T2BFK7MB88F2HYTRQCTX]
- docs/spec-sml/(decisions.sml + strata.toml)が fmt 冪等・build 診断ゼロ {#01KYP2T2BFK7MB88F2HYTRQCTY}
- D1〜D47・P1〜P4 の**全 51 裁定**が alias 付きで存在し、情報欠落ゼロ
  (原文とのカバレッジ確認方法は裁量・報告) {#01KYP2T2BFK7MB88F2HYTRQCTZ}
- inline ref と depends-on エッジの件数・内訳の報告、境界事例の列挙 {#01KYP2T2BFK7MB88F2HYTRQCV0}
- デモ2本の出力、摩擦レポート {#01KYP2T2BFK7MB88F2HYTRQCV1}
- **コミットはしない** {#01KYP2T2BFK7MB88F2HYTRQCV2}
