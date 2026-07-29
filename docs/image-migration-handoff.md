# 画像対応 移行スキル更新 + 実データ再移行ハンドオフ

画像対応 Phase A(strata本体、パーサ・ワークスペース・レンダラ対応)が完了した
ことを前提に、(1) 移行スキルに添付バイナリのコピー手順を追記し、(2)
strata-notes の既存移行データ(17ファイルが `![[file.png]]` を使用)を実際に
救済する。

## 前提(Phase A で確定した仕様)

- `strata.toml` に `assets = ["assets/**/*"]` のような明示的globを追加すると、
  そのglobにマッチする画像ファイル(png/jpg/jpeg/gif/svg/webp)がbasenameで
  解決対象になる。
- `![[file.png]]` はワークスペースbuild時に解決され、未解決は `UnresolvedAsset`、
  同名複数は `AmbiguousAsset`、ワークスペース無しでは `AssetNeedsWorkspace`。
- 詳細は `crates/strata-build/src/convert.rs` の asset 解決ロジックと
  `docs/image-support-handoff.md` を参照。

## 作業1: `docs/guides/obsidian-import-skill.sml` の更新

添付バイナリの非破壊コピー手順を追記する:

- 移行対象の `.md` ファイル内に `![[file.png]]`(認識拡張子: png/jpg/jpeg/gif/
  svg/webp)が見つかったら、元vault内でそのファイルを検索し(Obsidianは
  vault内でファイル名一意という前提が一般的だが、複数箇所に同名ファイルが
  存在する可能性は排除しない)、`strata-notes/assets/` へ**非破壊コピー**する
  (D62の非破壊原則、既存の「ソースvaultには一切書き込まない」を踏襲)。
- ファイル名衝突(コピー先で既に同名ファイルが存在し、内容が異なる)は
  スキップして最終報告に記録する(D37確信原則: 曖昧なものはAIが決め打ちで
  上書きしない)。
- `strata.toml` の `members` と並ぶ形で `assets = ["assets/**/*"]` を追記する
  手順も明記する。
- 既存の `docs/guides/obsidian-import-mapping.sml` に画像embed関連の言及が
  必要か確認し、必要なら追記する。

## 作業2: strata-notes の実データ再移行

対象: vault移行バッチ1〜3で既に `strata-notes/inbox`・`strata-notes/para/**`
へ配置済みの198文書のうち、`![[file.png]]` 形式の画像embed(認識拡張子)を
含む17ファイル(具体的なファイル一覧は `grep -rl '!\[\[' ~/dev/strata-notes/inbox
~/dev/strata-notes/para` 等で再列挙すること — 拡張子で画像/非画像を判別し、
画像のものだけが対象)。

1. `~/dev/strata-notes/strata.toml` に `assets = ["assets/**/*"]` を追記する。
2. 上記17ファイルが参照する画像ファイルを、元vault
   (`/mnt/c/usr/KnowledgeVault/kurochanBrainPrivate`)内から実際に探し、
   `~/dev/strata-notes/assets/` へ非破壊コピーする(読み取りのみ、書き込み
   禁止は厳守)。**元vaultには実際に79件の画像ファイルが存在することを
   確認済み**(今回移行済みの17ファイルが参照する分だけで足りるはずだが、
   見つからないものはスキップして報告)。
3. `strata build --workspace ~/dev/strata-notes/strata.toml` を実行し、
   診断ゼロ(または既存のHtmlNotSupported warning 12件のみ)になることを
   確認する。`UnresolvedAsset`/`AmbiguousAsset` が出た場合は個別に原因調査し、
   解決できないものはスキップして最終報告に列挙する。
4. `strata fmt --check` で冪等性を確認(画像embed構文自体はfmtで書き換わる
   ものではないはずだが念のため)。
5. 可能なら1〜2件について `strata render --format typst` →
   `typst compile --root /` で実際に画像入りPDFが生成できることを確認する
   (Phase Aの検証と同じ手順)。

## ルール

git操作禁止。移行元(kurochanBrainPrivate)には一切書き込まない(読み取りの
み)。strataリポジトリ本体は `docs/guides/obsidian-import-skill.sml` の更新
以外変更しない。曖昧な点は裁量として最終報告に明記。

## 完了の定義

作業1(スキル更新)・作業2(実データ再移行、可能な範囲まで)完了。
strata-notesのbuild診断が新規エラーゼロ。両リポジトリともコミットしない。

最終報告: スキル更新内容 / 再移行できたファイル数・救済できた画像数 /
スキップしたファイルとその理由 / build検証結果 / typst compile検証結果
(実施できた場合)/ 裁量箇所
