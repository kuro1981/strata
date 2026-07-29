# vault 移行バッチ2 ハンドオフ(未移行分の継続)

`docs/handoffs/vault-integration-handoff.sml`(バッチ1: Clippings 46件・
00_Daily 32件を本番配置済み)の続き。`~/dev/strata-notes/goal.sml` の
ゴール(②意味の記録=グラフ昇格が本線)に従う。

## 対象と件数(2026-07-29 時点で判明)

| ソース | フォルダ | 件数 | 既知の形 |
|---|---|---|---|
| kurochanBrainPrivate | `02_Calendar/00_Daily` 残り | 54 | **既知**(バッチ1の00_Dailyと同一形式、frontmatterなし) |
| kukulog-api AiVault | `X/` | 29 | **既知**(frontmatter: type/author/name/url/date/tags、末尾に❤️🔁📅統計行 — 当初の調査で確認済み) |
| kukulog-api AiVault | ルート直下(雑多note) | 29 | 未調査 |
| kukulog-api AiVault | `Clippings/` | 1 | 未調査(1件のみ) |
| kurochanBrainPrivate | `00_Atlas` | 10 | 未調査 |
| kurochanBrainPrivate | `04_Resources` | 1 | 未調査 |
| kurochanBrainPrivate | `10_Archive` | 45 | 未調査 |
| kurochanBrainPrivate | `20_Meta` | 10 | 未調査 |
| kurochanBrainPrivate | `+`直下(雑多note) | 9 | 未調査 |
| kurochanBrainPrivate | `diary/` | 3 | **既知**(ほぼ空、既に調査済み — 2ファイル0バイト・1ファイル1行) |
| kurochanBrainPrivate | ルート直下 | 1 | 未調査(HOME.md 想定) |

## 方針(段階的慎重さを維持)

**既知パターンは本番直行、未調査は先にパイロット**(今日の運用と同じ規律):

### A. 本番直行(スクラッチ経由の非破壊コピー→変換→build検証→strata-notesへ配置)

1. `02_Calendar/00_Daily` 残り54件 → `strata-notes/daily/`(バッチ1と同じ
   変換規則。バッチ1の32件と合わせて86件フルカバーになる)
2. AiVault `X/` 29件 → `strata-notes/inbox/`(AI Vault 由来=inbox という
   既定の位置づけ。frontmatterの type/author/name/url/date/tags は
   `docs/guides/obsidian-import-mapping.sml` の既存マッピング(class/record
   分離)を新規に**Xポスト用として設計**する必要がある — Clippings用の
   mapping とは frontmatterのキーが異なるため流用不可。tags→class,
   author/name/url/date→record が妥当な出発点、実データで確認して決める)
3. `diary/` 3件 → 内容確認の上、意味があるもの(1行リンクの
   `2026-04-28.md`)だけ `strata-notes/inbox/` へ、空の2件はスキップ記録

### B. 先にパイロット(スクラッチ上で調査→少数変換→報告、strata-notesへの
本番書き込みはしない。私(オーケストレーター)が結果を見てから本番化を判断)

4. 未調査の8フォルダ(AiVaultルート29件・Clippings1件、kurochanBrainPrivate
   の00_Atlas・04_Resources・10_Archive・20_Meta・`+`直下9件・ルート1件、
   計約96件): まず `docs/guides/obsidian-import-skill.md` §1 の調査コマンド
   でfrontmatterの実態を調べる。既存マッピング(Clippings用)で足りるものは
   足りると報告し、新しいマッピングが必要な形が見つかったら**代表数件だけ**
   スクラッチで変換・build検証し、残りは実行せず件数・内容概要・提案
   マッピングを最終報告に書く(本番配置はしない)

## 命名衝突・ファイル名注意

- 00_Atlas・10_Archive等はサブフォルダを持つ可能性がある(maxdepth指定なしで
  再帰調査すること)
- 長いファイル名(Web クリップ由来)は CLI の一時ファイル名バグは既に修正済み
  (`a9614e5`)なので問題にならないはず。念のため確認

## ルール

- git操作禁止。移行元(kukulog-api の AiVault、KnowledgeVault の
  kurochanBrainPrivate)には一切書き込まない(読み取りのみ)
- strata-notes への書き込みはA(本番直行分)のみ。Bは調査・パイロットに留め
  strata-notesへの本番書き込みはしない
- strata リポジトリ本体(~/dev/strata)は変更しない(CLI 実行のみ)
- 曖昧な点は裁量として最終報告に明記

## 検証

- `strata fmt --check` 冪等・`strata build --workspace ~/dev/strata-notes/strata.toml`
  診断ゼロ(A分の配置後)
- `strata search --workspace ... "class:..."` で新規データが検索可能なことを確認

## 完了の定義

A(本番直行3件)完了、B(調査+パイロット)の報告完了。コミットしない。

最終報告: A の配置件数・検証結果 / X用マッピングの設計内容 / B の調査結果
(フォルダごとのfrontmatter実態・既存マッピングで足りるか)/ B のパイロット
結果(実施した場合)/ 次バッチ(本番化)の提案 / 裁量箇所
