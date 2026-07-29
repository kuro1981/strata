---
id: 01KYP2T29EYM4DFQB4GV22J691
title: "M5-B 実装ハンドオフ — AI 執筆ガイド(D37)の起草とドッグフーディング"
alias: agent-guide-m5b-handoff
---

# M5-B 実装ハンドオフ — AI 執筆ガイド(D37)の起草とドッグフーディング {#01KYP2T29EYM4DFQB4GV22J692}

[id=01KYP2T29EYM4DFQB4GV22J693]
sml-spec §1.7 D37(2026-07-15 確定)の実行指示。新機能の実装ではなく、
**作法の文書化(正典ガイド)と、そのガイドだけに従った執筆ドッグフーディング**。

## 必読(この順) {#01KYP2T29EYM4DFQB4GV22J694}

[id=01KYP2T29EYM4DFQB4GV22J695]
1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリ) {#01KYP2T29EYM4DFQB4GV22J696}
2. [D36](ref:decisions/d36)/[D37](ref:decisions/d37)と[decisions.sml](doc:decisions)+[grammar.sml](doc:grammar)全体(ガイドは spec の要約を含むため通読) {#01KYP2T29EYM4DFQB4GV22J697}
3. [view-def-v1](doc:view-def-v1)(view --check への言及用) {#01KYP2T29EYM4DFQB4GV22J698}
4. `~/dev/strata-my-resume/sml/work_history.sml`(ドッグフーディング対象の実例) {#01KYP2T29EYM4DFQB4GV22J699}

## WP-B1: [sml-agent-guide](doc:sml-agent-guide) の起草 {#01KYP2T29EYM4DFQB4GV22J69A}

[id=01KYP2T29EYM4DFQB4GV22J69B]
対象読者: SML 文書を書く・編集する **AI エージェント**(Claude に限らない)。
自己完結(このファイルだけ読めば書ける)だが、詳細は[decisions.sml](doc:decisions)/[grammar.sml](doc:grammar)への参照で逃がす。

[id=01KYP2T29EYM4DFQB4GV22J69C]
含めるもの:

[id=01KYP2T29EYM4DFQB4GV22J69D]
1. **SML 記法の実用要約**(再定義ではなく要約+実例): 見出し/段落/リスト
   (ネスト可)/属性行(`[id=, alias=, class=, supports=, depends-on=, cites=]`)/
   `::table`(次元・member label・セル値型)/`::record`/`::math`/`::figure`/
   フロントマター/`date-format=`/Date・Period の書式 {#01KYP2T29EYM4DFQB4GV22J69E}
2. **D37 の作法**(正典はこの4つ): {#01KYP2T29EYM4DFQB4GV22J69F}
  - ULID を書かない・発明しない。ID は `strata fmt` が注入する。
     draft では `{#ラベル}` か無記名でよい {#01KYP2T29EYM4DFQB4GV22J69G}
  - alias を積極的に付ける(意味のある名前。ビューと人の両方から引ける) {#01KYP2T29EYM4DFQB4GV22J69H}
  - 既存ノードへの参照・エッジは `strata context` 出力のアドレスタグ
     (`{#ULID alias=x}` — SML と同一記法)からコピーする {#01KYP2T29EYM4DFQB4GV22J69J}
  - エッジは**確信のあるものだけ**張る。推測で張らない(誤エッジは無エッジより害)。
     迷ったら張らず、人間への提案として報告に書く {#01KYP2T29EYM4DFQB4GV22J69K}
  - AI 下書きの専用 class は付けない(class は意味分類専用)。
     レビューは git diff で行われる {#01KYP2T29EYM4DFQB4GV22J69M}
3. **執筆前の読み方**: `strata context <file>` で全体像、`--node <alias> --hops 1`
   で編集対象周辺、`--class <tag>` で横断確認 {#01KYP2T29EYM4DFQB4GV22J69N}
4. **書き込み後の必須検証シーケンス**: `strata fmt <file>` → `strata build <file>`
   →(ビュー定義があるプロジェクトなら `strata view --check`)。
   診断が出たら自分で解消してから人間レビューに出す。exit code の意味も記載 {#01KYP2T29EYM4DFQB4GV22J69P}
5. **チェックリスト**(最後に箇条書きで: ULID 発明してないか/alias 付けたか/
   エッジは確信分だけか/検証シーケンス通したか/推測エッジ候補を報告に書いたか) {#01KYP2T29EYM4DFQB4GV22J69Q}

[id=01KYP2T29EYM4DFQB4GV22J69R]
品質基準: **このガイドだけを読んだ別のエージェントが正しく書けること**。
[view-def-v1](doc:view-def-v1) と同様、後日ユーザーが批准する前提の起草。

## WP-B2: ドッグフーディング — ガイドだけに従った追記 {#01KYP2T29EYM4DFQB4GV22J69S}

[id=01KYP2T29EYM4DFQB4GV22J69T]
実文書を汚さないため、**コピーで実施**する:

[id=01KYP2T29EYM4DFQB4GV22J69V]
1. `~/dev/strata-my-resume/sml/work_history.sml` をあなたのスクラッチディレクトリに
   コピー {#01KYP2T29EYM4DFQB4GV22J69W}
2. 以下の架空ブリーフ(デモ用・事実ではない)から、新しいプロジェクト経歴を
   **WP-B1 で書いたガイドだけに従って**追記する: {#01KYP2T29EYM4DFQB4GV22J69X}
[id=01KYP2T29EYM4DFQB4GV22J69Y]
> [id=01KYP2T29EYM4DFQB4GV22J69Z]
   > 2026年2月〜現在、リーディングエッジテクノロジーセンターにて社内文書基盤の
   > PoC。LLM エージェントが仕様書・議事録を意味グラフとして管理する仕組みの
   > 技術検証。役割はテックリード。技術は Rust / Typst / Claude API。
   > 成果: 文書からの意味単位のコンテキスト抽出で照会応答の精度向上を確認。
   > 面接メモ(非公開): この PoC は自作 OSS 文書フォーマットの実地検証を兼ねた。
[id=01KYP2T29EYM4DFQB4GV22J6A0]
3. 期待される作業内容(ガイドが正しければ自然にこうなるはず — 誘導ではなく検算用):
   project-index 表への行追加(Period・tech 列)、詳細節(H4+alias)の追加、
   面接メモの `[class=note]` 段落、概略表の整合、ULID は書かずに fmt へ {#01KYP2T29EYM4DFQB4GV22J6A1}
4. 検証シーケンスを実行(fmt → build → `strata context --node <新alias>` で
   自己確認。view --check はコピー先では view 定義のパス関係が崩れるため任意 —
   やったかどうかを報告) {#01KYP2T29EYM4DFQB4GV22J6A2}
5. **実リポジトリの work_history.sml には一切書き込まない**。成果は
   (a) 追記後ファイルの diff(unified 形式)を最終報告に全文引用、
   (b) ガイドに従って書いた際の**ガイドの不足・曖昧箇所**の列挙(ガイド改訂の材料) {#01KYP2T29EYM4DFQB4GV22J6A3}

## 完了の定義 {#01KYP2T29EYM4DFQB4GV22J6A4}

[id=01KYP2T29EYM4DFQB4GV22J6A5]
- [sml-agent-guide](doc:sml-agent-guide) 起草完了(strata リポジトリ内、未コミットのまま) {#01KYP2T29EYM4DFQB4GV22J6A6}
- WP-B2 の diff が検証シーケンス通過済みであること(fmt 冪等・build 診断ゼロ) {#01KYP2T29EYM4DFQB4GV22J6A7}
- 両リポジトリで git 操作なし、~/dev/strata-my-resume への書き込みなし
  (読み取りとスクラッチへのコピーのみ) {#01KYP2T29EYM4DFQB4GV22J6A8}
- 最終報告: ガイドの構成概要 / WP-B2 の diff 全文 / ガイドの不足・曖昧箇所 /
  裁量箇所 / 推測エッジとして「張らずに報告に回した」ものがあればその列挙
  (作法(2)が実際に機能した証拠になる) {#01KYP2T29EYM4DFQB4GV22J6A9}
