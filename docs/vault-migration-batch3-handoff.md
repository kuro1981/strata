# vault 移行バッチ3 ハンドオフ(調査済み8フォルダの本番化)

バッチ2(コミット `de540dc`)で調査・パイロット済みの8フォルダ(計106件)を
本番配置する。今回は「既知パターンは本番直行」の範囲を広げ、以下の**明示的
除外**以外は本番書き込みしてよい。

## 除外(このバッチでは絶対に変換・配置しない)

1. **セキュリティ**: `無題のファイル.md`(AiVault root、API キーらしき文字列)、
   `LightRAGづくり覚えがき.md`(00_Atlas、秘密鍵ファイル `lightragtest_key.pem`
   への埋め込み参照)— この2件は内容を要約・引用もせず完全にスキップし、
   最終報告に「スキップ(セキュリティ、人間確認待ち)」とだけ書く
2. **Obsidian デモ/プラグイン内部ファイル**: `10_Archive/Demo Project/`
   配下5件、`20_Meta/excalibrain.md`、`20_Meta/templates/` 配下6件、
   AiVault root の `ようこそ.md`・`ObsidianTags.md`
3. `無題のファイル 1.md`(AiVault root、Nix設定の無関係な貼り付け、ノイズ)

## 対象と配置方針

| フォルダ | 件数(除外後目安) | 配置先 | 方針 |
|---|---|---|---|
| AiVault root | 約25件 | `inbox/`(既存の X 用マッピング`docs/obsidian-import-x.yaml`流用可能な5件はそのまま、`type:article`新パターン7件はマッピング拡張、frontmatterなしリンク集3件・その他) | バッチ2パイロットで確認済みのパターンを本番適用 |
| AiVault Clippings/ | 1件 | `para/resources/`(既存Clippingsマッピング、record空でも可) | |
| 00_Atlas | 9件(secret-key1件除く) | `para/resources/`(基本)。ただし `HOME.md` はフォルダ参照wikilink問題(下記)を先に解決してから | |
| 04_Resources | 1件 | `para/resources/` | 00_Atlasと同型 |
| 10_Archive | 40件(Demo Project除く) | `para/archive/` | frontmatterなし本文のみ、実在の取引先名を含むものはそのまま(既に許可された実データ) |
| 20_Meta | 3件(プラグイン内部除く) | `para/resources/` または `para/areas/`(内容次第、裁量) | |
| `+`直下 | 8件(Untitled.md除く) | 内容次第で `para/resources/` または `para/areas/`。**不動産所有物件一覧・確定申告ティップスの2件は個人financial情報 — 変換してよいが、最終報告で明示的に「financial情報を含むファイルを移行した」と報告すること**(ユーザーは全体実施を承認済み) | |

## HOME.md のフォルダ参照 wikilink 問題(設計判断)

`00_Atlas/HOME.md` は `[[+/]]` のような**フォルダそのものを指す wikilink**を
使う MOC(Map of Content)。Strata の wikilink 解決はタイトル一致(D59)なので
フォルダパスは解決しようがない。**方針: デリンクして平文化する**(D59の
「ワークスペース外参照はデリンクして通す」既存パターンと同じ扱い。フォルダ
索引ドキュメントを新設するような構造変更はこのバッチのスコープ外)。
可視情報(リンクテキスト)は保持し、リンクの意味だけ落とす。

## date キー対応(00_Atlas/04_Resources の一部)

バッチ2パイロットで確認済みの、時刻付き `date` キー(既存Clippingsマッピングの
`created`/`published` とは別に、時刻部分を含む `date` フィールド)を
型付きパースできることを踏まえ、既存 `docs/obsidian-import-mapping.md` に
基づく `docs/obsidian-import.yaml`(kurochanBrainPrivate用)に `date` キーの
扱いを追記してから適用する(裁量: 既存マッピングファイルへの追記か、
別マッピングにするかは判断し報告)。

## 検証

- `strata fmt --check` 冪等(全新規ファイル)
- `strata build --workspace ~/dev/strata-notes/strata.toml` 診断ゼロ
- `strata search --workspace ... "class:..."` 等で新規データが検索可能

## ルール

- git操作禁止。移行元(kukulog-api の AiVault、KnowledgeVault の
  kurochanBrainPrivate)には一切書き込まない(読み取りのみ)
- strataリポジトリ本体(~/dev/strata)は変更しない(CLI実行のみ)
- 曖昧な点は裁量として最終報告に明記。特にファイル配置先(resources/areas/
  archive の判断)に迷ったら裁量で決めて報告する(過度に立ち止まらない)

## 完了の定義

除外リスト以外の全件配置完了。build診断ゼロ。コミットしない。

最終報告: 配置件数と内訳 / 除外したファイルとその理由(セキュリティ2件は
名前のみ記載) / HOME.mdデリンクの実施結果 / dateキー対応マッピングの
最終形 / financial情報ファイルの移行報告 / 検証結果 / 裁量箇所
