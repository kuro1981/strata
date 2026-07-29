---
id: 01KYP4QVQGGV3MZBFTYF8SQD84
title: 統合 vault 構築ハンドオフ(ゴール修正後)
alias: vault-integration-handoff
---

# 統合 vault 構築ハンドオフ(ゴール修正後、2026-07-29) {#01KYP4QVQGGV3MZBFTYF8SQD85}

[id=01KYP4QVQGGV3MZBFTYF8SQD86, alias=intro]
`~/dev/strata-notes/goal.sml` に固定したゴールに沿って、strata-notes を**唯一の日常ドライバー vault** として構築する。①(互換性工学)ではなく②(意味の記録=グラフ昇格)を本線とする。

## 前提 {#01KYP4QVQGGV3MZBFTYF8SQD87}

[id=01KYP4QVQGGV3MZBFTYF8SQD88]
- `~/dev/strata-notes` は git 管理済み(初回スナップショット済み、ただし editor 実機テストの残骸 `zettel/saa.sml`・`zettel/test.sml`・`daily/` 配下に類似のものがあれば含む — 要掃除) {#01KYP4QVQGGV3MZBFTYF8SQD89}
- 既にパイロット検証済みのデータ(スクラッチ上、実 vault は無変更): {#01KYP4QVQGGV3MZBFTYF8SQD8A}
  - `pilot-clippings-src/sml-out/`(46件、Clippings、`obsidian-import.yaml` 適用済み・build 検証済み) {#01KYP4QVQGGV3MZBFTYF8SQD8B}
  - `pilot-daily-src/sml-out/`(32件、00_Daily、build 検証済み) {#01KYP4QVQGGV3MZBFTYF8SQD8C}
- 移行元(読み取りのみ、書き込み禁止): {#01KYP4QVQGGV3MZBFTYF8SQD8D}
  - AI Vault: `/home/kuro/dev/kukulog-api/obsidian-vault/AiVault/`(X/, Clippings/、59件、git管理外) {#01KYP4QVQGGV3MZBFTYF8SQD8E}
  - Private vault: `/mnt/c/usr/KnowledgeVault/kurochanBrainPrivate/`(304件、git管理済み) {#01KYP4QVQGGV3MZBFTYF8SQD8F}

## ゴールに沿った設計方針 {#01KYP4QVQGGV3MZBFTYF8SQD8G}

[id=01KYP4QVQGGV3MZBFTYF8SQD8H]
- **strata-notes が唯一の生きた vault**。移行元の Obsidian ファイルは「移行が終わったら編集しなくなる履歴的原本」という位置づけ(git 履歴が安全網、コピーを残す必要はない) {#01KYP4QVQGGV3MZBFTYF8SQD8J}
- 単なる「読めるようにする」変換ではなく、**意味が実際にグラフへ昇格する**ことを検証基準にする: class(分類)・record(記述的事実)・意味エッジが実際に使われているか {#01KYP4QVQGGV3MZBFTYF8SQD8K}

## 作業 {#01KYP4QVQGGV3MZBFTYF8SQD8M}

### WP-V1: strata-notes の構造整備 {#01KYP4QVQGGV3MZBFTYF8SQD8N}

[id=01KYP4QVQGGV3MZBFTYF8SQD8P]
- `zettel/saa.sml`・`zettel/test.sml`(editor 実機テストの残骸、内容がテスト用で無意味なもの)を削除する(内容を確認し、本当に無意味なら削除。何か実質的な内容があれば残す) {#01KYP4QVQGGV3MZBFTYF8SQD8Q}
- ディレクトリ構成を拡張(既存の `zettel/`・`daily/` は維持、新設): `inbox/`(AI Vault 由来の収集物、トリアージ待ち)・`para/projects/`・`para/areas/`・`para/resources/`・`para/archive/` {#01KYP4QVQGGV3MZBFTYF8SQD8R}
- `strata.toml` の members を更新してこれらを含める {#01KYP4QVQGGV3MZBFTYF8SQD8S}
- `index.sml` の運用規約に inbox/para の説明を追記(PARA と `class` の対応: `class=project`/`area`/`resource`/`archive`。トリアージは「inbox → para へ**移動**+class 付与、または確信があれば **リンク**(`ref:`/`supports`)を張って inbox に残す」の両方を許容し、どちらを選ぶかは内容次第(裁量、判断基準を最終報告に書く)) {#01KYP4QVQGGV3MZBFTYF8SQD8T}

### WP-V2: パイロット済みバッチの本番昇格 {#01KYP4QVQGGV3MZBFTYF8SQD8V}

[id=01KYP4QVQGGV3MZBFTYF8SQD8W]
検証済みの2バッチを scratch から strata-notes へ本番配置する:

[id=01KYP4QVQGGV3MZBFTYF8SQD8X]
- Clippings 46件 → `para/resources/`(記述的事実の record を持つ資料は基本 resources が妥当。個別に project/area 相当と判断したものがあれば `class` で調整)。`obsidian-import.yaml` も `docs/`(適切な場所)に永続化する {#01KYP4QVQGGV3MZBFTYF8SQD8Y}
- 00_Daily 32件 → `daily/`(既存の daily ディレクトリへ、ファイル名は日付のまま)。既存の `daily/2026-07-15.sml`(今回のセッション内で作成済み)と衝突しないことを確認 {#01KYP4QVQGGV3MZBFTYF8SQD8Z}
- `strata fmt --check` 冪等・`strata build --workspace strata.toml` 診断ゼロを確認 {#01KYP4QVQGGV3MZBFTYF8SQD90}

### WP-V3: 未移行分の一覧化(実行はしない) {#01KYP4QVQGGV3MZBFTYF8SQD91}

[id=01KYP4QVQGGV3MZBFTYF8SQD92, alias=wp-v3]
AI Vault の X/(59件中 Clippings 除く分)と、kurochanBrainPrivate の `00_Atlas`・`04_Resources`・`10_Archive`・`20_Meta`・独自 `+/Clippings`(47件、パイロットBとは別物)は**このタスクでは移行しない**。次バッチ候補として件数・内容の概要を最終報告に書くのみ。

[id=01KYP4QVQGGV3MZBFTYF8SQD93, alias=wp-v3-correction, class=note]
実行時の訂正: `+/Clippings` はパイロットBと「別物」ではなく**同一プール**であることが実データ調査で判明(47件中46件が変換済み、残り1件は空クリップで意図的スキップ)。ハンドオフの前提記述が誤っていた実例として記録。

## ルール {#01KYP4QVQGGV3MZBFTYF8SQD94}

[id=01KYP4QVQGGV3MZBFTYF8SQD95]
- git 操作禁止(strata-notes は既に init 済みだが commit はしない) {#01KYP4QVQGGV3MZBFTYF8SQD96}
- 移行元(kukulog-api の AiVault、KnowledgeVault の kurochanBrainPrivate)には一切書き込まない(読み取りのみ) {#01KYP4QVQGGV3MZBFTYF8SQD97}
- strata リポジトリ本体(~/dev/strata)は変更しない(CLI 実行のみ) {#01KYP4QVQGGV3MZBFTYF8SQD98}
- 曖昧な点は裁量として最終報告に明記 {#01KYP4QVQGGV3MZBFTYF8SQD99}

## 完了の定義 {#01KYP4QVQGGV3MZBFTYF8SQD9A}

[id=01KYP4QVQGGV3MZBFTYF8SQD9B]
- WP-V1/V2 完了、`strata build --workspace ~/dev/strata-notes/strata.toml` 診断ゼロ {#01KYP4QVQGGV3MZBFTYF8SQD9C}
- `strata search --workspace ~/dev/strata-notes/strata.toml "class:resource"` 等で昇格したデータが検索可能なことを確認 {#01KYP4QVQGGV3MZBFTYF8SQD9D}
- コミットしない {#01KYP4QVQGGV3MZBFTYF8SQD9E}

[id=01KYP4QVQGGV3MZBFTYF8SQD9F, alias=report-req]
最終報告: 構造整備の内容 / 本番配置したファイル数と配置理由 / トリアージ判断基準(移動 vs リンク)/ 未移行分の一覧(次バッチ候補)/ 検証結果 / 裁量箇所
