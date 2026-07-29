---
id: 01KYP2JYCC22N9S6VV89D1684F
title: "Obsidian → SML インポートガイド(AI エージェント向け、D62)"
alias: obsidian-import-skill
---

# Obsidian → SML インポートガイド(AI エージェント向け、D62) {#01KYP2JYCC22N9S6VV89D1684G}

[id=01KYP2JYCC22N9S6VV89D1684H]
対象読者: Obsidian vault を Strata ワークスペースへ取り込む作業を行う
AI エージェント。**このガイドだけを読めば実行できる**ことを品質基準とする。
記法は [sml-agent-guide](doc:sml-agent-guide)、マッピング仕様の文法は
[obsidian-import-mapping](doc:obsidian-import-mapping) を参照。

[id=01KYP2JYCC22N9S6VV89D1684J]
[D62](ref:decisions/d62)の裁定どおり、これは**スキル**であって専用バイナリでは
ない。既存の `strata fmt`/`strata build`/`strata context` と D37 の作法を使う。

## 0. 絶対原則 {#01KYP2JYCC22N9S6VV89D1684K}

[id=01KYP2JYCC22N9S6VV89D1684M]
1. **元の Obsidian vault(`*.md`)には書き込まない**。変換先は別ディレクトリ
   (新しい SML ワークスペース)。非破壊的インポート — 何度でもやり直せる {#01KYP2JYCC22N9S6VV89D1684N}
2. **ULID を書かない・発明しない**(D37 と同じ)。fmt に任せる {#01KYP2JYCC22N9S6VV89D1684P}
3. **確信のない変換は行わず、報告する**(D37 の確信原則の ETL への適用)。
   マッピングに無いキー・構造が壊れたファイル・意味不明な frontmatter は
   スキップしてトリアージ対象として最終報告に列挙する {#01KYP2JYCC22N9S6VV89D1684Q}
4. 実データを見ずにマッピングを書かない(§1 で必ず調査する) {#01KYP2JYCC22N9S6VV89D1684R}
5. **§3 の変換規則と §4 の検証結果が衝突する場合は §4(検証結果)を優先
   する**。例えば「`[[wikilink]]` はそのまま残す」(§3)が原則でも、実際に
   `strata build` がそれを Error にする状況(部分移行時の
   `UnresolvedWikilink`/`AmbiguousWikilink` 等、§3 参照)では、規則どおりに
   書いて Error を放置せず、検証が通る形に直すことを優先する {#01KYP2JYCC22N9S6VV89D1684S}
6. **内容がセンシティブ(性的な description・個人情報等)と判断される
   ファイルは、変換自体を妨げない**が、変換して良いかどうかは自分で判断
   せず**トリアージに回して人間の判断を仰ぐ**。安全側に倒してスキップして
   よい {#01KYP2JYCC22N9S6VV89D1684T}

## 1. frontmatter の実態調査 {#01KYP2JYCC22N9S6VV89D1684V}

[id=01KYP2JYCC22N9S6VV89D1684W]
対象ディレクトリの全 `.md` ファイルから frontmatter のキーを頻度順に洗い出す。
`tr -d '\r' |` を挟んで **CRLF 改行のファイルも取りこぼさない**こと(実データで
CRLF ファイルが19%を占め、これを挟まないと awk の行末判定がずれて調査から
漏れた実績がある):

```bash {#01KYP2JYCC22N9S6VV89D1684X}
find <vault> -name "*.md" -not -path "*/.obsidian/*" -print0 \
  | xargs -0 -I{} sh -c 'tr -d "\r" < "{}" | awk "/^---\$/{c++; if(c==2) exit; next} c==1"' \
  | grep -oE '^[a-zA-Z_][a-zA-Z0-9_-]*:' | sort | uniq -c | sort -rn
```

[id=01KYP2JYCC22N9S6VV89D1684Y]
少数のキーに集約されるはず(Web クリッパー・タスク管理・プラグイン設定など、
出所ごとに一貫したスキーマが多い)。個々のファイルも数件サンプリングして
実際の値の形(スカラーかリストか、日付の書式)を確認する。

## 2. マッピングの提案と批准 {#01KYP2JYCC22N9S6VV89D1684Z}

[id=01KYP2JYCC22N9S6VV89D16850]
[obsidian-import-mapping](doc:obsidian-import-mapping) の文法で `obsidian-import.yaml` 案を書く
(調査結果に基づく — 頻度・値の形から `as: class`/`as: record`/`as: ignore`
を判断する目安):

[id=01KYP2JYCC22N9S6VV89D16851]
- リスト値・カテゴリ的な意味(tags/status 等)→ `as: class` {#01KYP2JYCC22N9S6VV89D16852}
- スカラーの記述的事実(author/source/created 等)→ `as: record` {#01KYP2JYCC22N9S6VV89D16853}
- プラグイン固有・実行条件・低頻度の未知キー → `as: ignore` {#01KYP2JYCC22N9S6VV89D16854}

[id=01KYP2JYCC22N9S6VV89D16855]
**この案を人間に提示し、批准を待つ**(M5-C の運用: 提案は AI、決定は人間)。
批准前に変換作業へ進まない。

## 3. ファイルごとの変換規則 {#01KYP2JYCC22N9S6VV89D16856}

[id=01KYP2JYCC22N9S6VV89D16857]
批准されたマッピングに従い、各 `.md` を SML ドラフトへ変換する。

### フロントマター {#01KYP2JYCC22N9S6VV89D16858}

```yaml {#01KYP2JYCC22N9S6VV89D16859}
# 変換規則
title: <元の title:> または 最初の H1 または ファイル名 stem   # D59 のタイトル解決と同じ優先順位に揃える
class: <as: class に該当するキーの値をまとめて列挙>
```

[id=01KYP2JYCC22N9S6VV89D1685A]
`id:`/`alias:` は書かない(fmt/build に委ねる。ワークスペース内で他文書から
参照させたい場合のみ、人間の指示があれば `alias:` を追加してよい)。

[id=01KYP2JYCC22N9S6VV89D1685B]
**frontmatter の値と `::record` の値はクォート規則が異なる**: frontmatter は
生テキストそのまま(Obsidian の YAML クォートをそのまま踏襲してよい)だが、
`::record` は表セルと同じ型付きパースでダブルクォートを剥がした値を書く
(下記「本文」参照)。**元の YAML のクォート記法をそのまま record 側にコピー
しない**こと。

### 本文 {#01KYP2JYCC22N9S6VV89D1685C}

[id=01KYP2JYCC22N9S6VV89D1685D]
- **`[[wikilink]]` はそのまま残す**(D59 でネイティブ対応済み。変換不要 —
  Strata が文書タイトル一致で自動解決する)。ただし**部分移行**(vault の
  一部だけを対象にする場合)では、対象外の文書への `[[wikilink]]` が
  `UnresolvedWikilink`(参照先が変換対象に無い)や `AmbiguousWikilink`
  (タイトル重複)で `strata build` の Error になることがある。対処は
  (a) その wikilink をデリンクして平文化する、または (b) `--workspace` の
  対象範囲を段階的に広げて参照先も変換対象に含める、のいずれか。どちらを
  選ぶかは移行の進め方(一括か段階的か)次第であり、人間に確認するか
  裁量で選び最終報告に明記する {#01KYP2JYCC22N9S6VV89D1685E}
- `[[target|表示テキスト]]` もそのまま {#01KYP2JYCC22N9S6VV89D1685F}
- **本文冒頭に H1 が無い場合**(Web クリップ等、タイトルが frontmatter の
  `title:` にしか無いファイル)は、`title:` の値から H1 を合成して本文の
  先頭に挿入する。**判定は必ず本文の先頭のみを見る**こと — 本文中盤に
  無関係な見出しが紛れ込んでいることがあり、それを H1 の存在と誤認しない {#01KYP2JYCC22N9S6VV89D1685G}
- `as: record` のキーは、H1 直後に1つの `::record` ブロックとしてまとめる:
  ```markdown
  ::record {#meta}
  著者: <author の値>
  出典: <source の値>
  作成日: <created の値>
  ::
  ```
  値は表セルと同じ型付きパースでダブルクォートを剥がしたプレーンテキストに
  する(frontmatter のクォート記法をそのまま貼らない)。**値が wikilink
  記法を含む場合**(例: `author: ["[[Name]]"]`)は `[[` `]]` を剥がして
  プレーンテキスト化する(`Name` のみを書く) — record 値は wikilink 解決の
  対象外のため、`[[ ]]` を残すとそのままテキストとして表示されてしまう {#01KYP2JYCC22N9S6VV89D1685H}
- **埋め込み `![[target]]` は非対応**(§10 保留)。見つけたら本文はそのまま
  残しつつ、最終報告のトリアージ一覧に加える(黙って落とさない) {#01KYP2JYCC22N9S6VV89D1685J}
- 通常の Markdown(見出し・リスト・強調・外部リンク等)はそのまま
  SML ドラフトとして有効(D39 の上位互換性) {#01KYP2JYCC22N9S6VV89D1685K}

### スコープ外ファイル {#01KYP2JYCC22N9S6VV89D1685M}

[id=01KYP2JYCC22N9S6VV89D1685N]
- `.excalidraw.md`(図形描画プラグインの内部形式)は変換しない。一覧に
  「スコープ外」として記録するのみ {#01KYP2JYCC22N9S6VV89D1685P}
- Dataview インラインフィールド(`key:: value`)・callout(`> [!note]`)を
  多用するファイルは、変換後に情報が欠落する可能性が高いのでトリアージ対象
  として報告(§10 保留、v0 では素通しテキスト化される) {#01KYP2JYCC22N9S6VV89D1685Q}

## 4. 検証 {#01KYP2JYCC22N9S6VV89D1685R}

[id=01KYP2JYCC22N9S6VV89D1685S]
各ファイルを変換したら、必ず実行する(D37 §4 と同じ):

```bash {#01KYP2JYCC22N9S6VV89D1685T}
strata fmt <file.sml>
strata fmt --check <file.sml>   # 冪等確認
strata build --workspace <strata.toml>   # 診断ゼロを確認(Warning は内容を読む)
```

[id=01KYP2JYCC22N9S6VV89D1685V]
Error が出たら自分で直す(黙って人間に投げない)。直せない・原因が不明な
場合はそのファイルをトリアージ一覧に回す。

## 5. 最終報告に含めること {#01KYP2JYCC22N9S6VV89D1685W}

[id=01KYP2JYCC22N9S6VV89D1685X]
- 変換したファイル数・スキップしたファイル数と理由(スコープ外/トリアージ) {#01KYP2JYCC22N9S6VV89D1685Y}
- 提案したマッピングと、批准された最終形の差分(あれば) {#01KYP2JYCC22N9S6VV89D1685Z}
- トリアージ一覧(埋め込み・Dataview・変換不能ファイル) {#01KYP2JYCC22N9S6VV89D16860}
- `strata build` の Warning 一覧(D60 で許容されるが内容の確認は要る) {#01KYP2JYCC22N9S6VV89D16861}
