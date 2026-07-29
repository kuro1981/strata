---
id: 01KYP2JYDWNRZKJ1X7AMC6FTF9
title: "G2 実装ハンドオフ — strata-editor(D54/D55)"
alias: editor-g2-handoff
---

# G2 実装ハンドオフ — strata-editor(D54/D55) {#01KYP2JYDX2Z4YCCKCH1MNKSYN}

[id=01KYP2JYDX2Z4YCCKCH1MNKSYP]
sml-spec §1.15(2026-07-16 確定)の実装指示。**新リポジトリ `~/dev/strata-editor`**
に Tauri 製エディタ(Obsidian 代替の骨格)を立てる。

## 最重要(D54: 疎結合境界) {#01KYP2JYDX2Z4YCCKCH1MNKSYQ}

[id=01KYP2JYDX2Z4YCCKCH1MNKSYR]
- エディタは strata の**公開境界のみ**に依存する: Rust クレート(path 依存
  `../strata/crates/*`、将来 git 依存化)、graph JSON スキーマ、ui/ ビューアの
  外部参照。**strata リポジトリ本体のコード・仕様は原則変更しない**
  (唯一の例外: ビューア再利用のために ui/ 内のコンポーネントを importable に
  する最小限の再構成(re-export の追加等)は可 — 変更内容を必ず報告) {#01KYP2JYDX2Z4YCCKCH1MNKSYS}
- **git 操作はしない**(新リポジトリの git init も含む — ユーザーが行う)。
  ファイル作成のみ {#01KYP2JYDX2Z4YCCKCH1MNKSYT}
- ~/dev/strata-notes は読み取り+CLI/アプリからの検証利用のみ(v0 のテスト vault) {#01KYP2JYDX2Z4YCCKCH1MNKSYV}

## 必読 {#01KYP2JYDX2Z4YCCKCH1MNKSYW}

[id=01KYP2JYDX2Z4YCCKCH1MNKSYX]
1. `~/dev/strata/AGENTS.md`・[D54](ref:decisions/d54)/[D55](ref:decisions/d55)・[D49](ref:decisions/d49)/[D50](ref:decisions/d50) {#01KYP2JYDX2Z4YCCKCH1MNKSYY}
2. `~/dev/strata/ui/src/`(再利用するビューア: GraphPane/DocumentPane/
   GraphContext/graphIndex 等) {#01KYP2JYDX2Z4YCCKCH1MNKSYZ}
3. `~/dev/strata/crates/strata-build/src/{lib,workspace}.rs`・
   `strata-sml/src/lib.rs`(fmt の公開 API)— Tauri コマンドから呼ぶ面 {#01KYP2JYDX2Z4YCCKCH1MNKSZ0}

## 構成(D54/D55) {#01KYP2JYDX2Z4YCCKCH1MNKSZ1}

``` {#01KYP2JYDX2Z4YCCKCH1MNKSZ2}
~/dev/strata-editor/
  flake.nix          # devshell: rust + node/pnpm + tauri 依存(webkitgtk 等)
  src-tauri/         # Rust: tauri v2。strata crates を path 依存で
  src/               # フロント: Vite + React + shadcn(ui/ ビューアを再利用)
  README.md          # 起動手順(nix develop → pnpm tauri dev)
```

[id=01KYP2JYDX2Z4YCCKCH1MNKSZ3]
- ビューア再利用の v0 方式: pnpm `file:../strata/ui` 依存+Vite alias で
  ソース参照(Tailwind 設定の統合に注意)。無理筋なら strata/ui 側に
  「viewer を re-export する index」を足す最小再構成まで可(報告)。
  それでも駄目ならコピー+出所コメント(最終手段。バックログに
  「ui のパッケージ化」を報告として残す) {#01KYP2JYDX2Z4YCCKCH1MNKSZ4}

## v0 スコープ(D55) {#01KYP2JYDX2Z4YCCKCH1MNKSZ5}

### WP-E1: シェルと読み {#01KYP2JYDX2Z4YCCKCH1MNKSZ6}

[id=01KYP2JYDX2Z4YCCKCH1MNKSZ7]
- vault を開く(起動引数 or ファイルダイアログで strata.toml 選択)。
  最近使った vault(ローカル設定に保存) {#01KYP2JYDX2Z4YCCKCH1MNKSZ8}
- Tauri コマンド `build_workspace(toml_path) -> graph JSON 文字列`
  (strata-build をプロセス内呼び出し)。フロントは G1 ビューアで描画
  (2ペイン・LOD・class トグル — 既存機能そのまま) {#01KYP2JYDX2Z4YCCKCH1MNKSZ9}

### WP-E2: 編集ループ(D55 の心臓) {#01KYP2JYDX2Z4YCCKCH1MNKSZA}

[id=01KYP2JYDX2Z4YCCKCH1MNKSZB]
- 文書ペインに「編集」トグル: 選択中の文書の生 SML を **CodeMirror 6** で表示
  (Markdown ハイライト+SML 属性行の軽い装飾。CodeMirror はビルド時依存で
  自己完結ルールと両立) {#01KYP2JYDX2Z4YCCKCH1MNKSZC}
- 保存(Cmd/Ctrl+S): {#01KYP2JYDX2Z4YCCKCH1MNKSZD}
  1. Tauri コマンド `fmt_text(text) -> Result<FmtOutcome>` — **fmt をインメモリ
     実行**(strata-sml の公開 API。ファイルは書かない) {#01KYP2JYDX2Z4YCCKCH1MNKSZE}
  2. 成功: 返ってきた整形済みテキストを**エディタバッファに適用**(ID がその場で
     生える)→ ディスクへ原子的書き込み → デバウンス(例 300ms)で
     `build_workspace` 再実行 → グラフ更新 {#01KYP2JYDX2Z4YCCKCH1MNKSZF}
  3. fmt/build 失敗: バッファは保持、**グラフは last-good のまま**、診断パネルに
     「行:列: 種別: メッセージ」を列挙し、クリックでエディタの該当行へジャンプ {#01KYP2JYDX2Z4YCCKCH1MNKSZG}
- 外部エディタでの変更: ファイルウォッチ(notify クレート)で検知 → 再 build →
  グラフ更新(開いているバッファと衝突したら「外部変更あり」の非破壊的な通知。
  マージはしない — v0 は読み直しボタン) {#01KYP2JYDX2Z4YCCKCH1MNKSZH}

### WP-E3: カード作成 {#01KYP2JYDX2Z4YCCKCH1MNKSZJ}

[id=01KYP2JYDX2Z4YCCKCH1MNKSZK]
- 「新規カード」: ファイル名(スラッグ)入力 → `zettel/<slug>.sml` を
  フロントマター(title/alias=スラッグ)付きで生成 → strata.toml の members に
  合致することを確認(グロブで拾えるか)→ エディタで開く {#01KYP2JYDX2Z4YCCKCH1MNKSZM}
- 「今日のノート」ボタン: `daily/YYYY-MM-DD.sml` を開く(無ければ雛形生成)。
  雛形の形は ~/dev/strata-notes の既存 daily に合わせる {#01KYP2JYDX2Z4YCCKCH1MNKSZN}

### WP-E4: 検証 {#01KYP2JYDX2Z4YCCKCH1MNKSZP}

[id=01KYP2JYDX2Z4YCCKCH1MNKSZQ]
- `cargo build`(src-tauri)と `pnpm build` が通ること。GUI 実行はユーザーの
  目視ゲート(WSLg 前提。起動手順を README と最終報告に明記) {#01KYP2JYDX2Z4YCCKCH1MNKSZR}
- テスト vault = ~/dev/strata-notes で: 開く → 読む → カード編集・保存で ID が
  生える → グラフに反映、の一連をコードレベルで自己点検(headless で可能な範囲) {#01KYP2JYDX2Z4YCCKCH1MNKSZS}
- strata 側 `cargo test --workspace` の非退行(ui/ を触った場合は pnpm build も) {#01KYP2JYDX2Z4YCCKCH1MNKSZT}

## v1 に持ち越し(やらない) {#01KYP2JYDX2Z4YCCKCH1MNKSZV}

[id=01KYP2JYDX2Z4YCCKCH1MNKSZW]
検索・バックリンクパネル・コマンドパレット・インクリメンタル build(§10)・
Tauri への UI 資産埋め込み配布・マージ UI

## 最終報告 {#01KYP2JYDX2Z4YCCKCH1MNKSZX}

[id=01KYP2JYDX2Z4YCCKCH1MNKSZY]
構成と依存 / ビューア再利用の実現方式(file 依存が通ったか)/ 編集ループの
実装詳細(fmt パッチ適用・デバウンス・last-good)/ 裁量箇所 / 起動手順 /
既知の制限と v1 持ち越し
