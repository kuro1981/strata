# Obsidian frontmatter マッピング仕様 v0(D61)

策定: 2026-07-29(設計対話から起草)。`docs/sml-spec.md` §1.19(D60〜D62)の実装。
`split: whitespace` コンビネータ(2.1)は同 §1.20(D63、パイロット移行フィードバック)で追加。

view-def-v1.md(D30〜D35)と同じ思想: **固定コンビネータのみ、スクリプトや正規表現は
持たない**。マッピングは vault ごとに1ファイル書く。「ユーザーの自由度」は
Strata がキーの意味を決め打ちしないことで確保され、この宣言ファイルの中に住む。

## 1. 全体像

```
Obsidian vault(*.md + YAML frontmatter)
    ↓ 人間 or AI が obsidian-import.yaml を書く(D62 はまずスキルとして提供)
    ↓ マッピングに従って各ファイルを SML ドラフトに変換
SML ドラフト → strata fmt → strata build で検証
```

マッピングファイルは実行可能なコードではなく、**変換の意図を記録した仕様書**。
LLM が提案し人間が読んで批准できることが品質基準(M5-C と同じ運用)。

## 2. 文法

```yaml
version: 1

frontmatter:
  <キー名 or "グロブパターン">:
    as: class | record | ignore
    key: <record の場合のみ。SML 側の record キー名(省略時は元のキー名をそのまま使う)>
    type: text | date | ref   # record の場合のみ。省略時は text(型付きパースは D28 の型に委ねる)

  default:
    as: ignore   # 未列挙キーの既定動作。省略時は ignore
```

### 2.1 `as: class` — 分類

値は frontmatter の `class:` キー(D61 で新設)に写す。元の値がリストなら
そのまま複数 class として展開する。

```yaml
tags: { as: class }
```

```yaml
# Obsidian 側
---
tags: [ai, mcp]
---
```
```yaml
# SML 側(生成されるフロントマター)
---
title: ...
class: [ai, mcp]
---
```

D46 の実効 class により、文書直下の全ブロックに自動継承される
(Obsidian の「ノート全体へのタグ付け」の意味論と一致)。

#### `split: whitespace`(D63)

Web クリッパー系プラグインは複数タグを1リスト項目に空白区切りで詰めることが
ある(例: `tags: ["clippings RAG tools"]` — 1要素の中に3語)。これを素朴に
`as: class` へ流すと、値がそのまま1個の class 名として扱われ `class:` の
字句規則(`[A-Za-z0-9_-]+`、空白不可)に反して `BadClass` で build が落ちる。

`split: whitespace` を `as: class` に添えると、**各リスト項目を空白で分割
してから**複数 class として展開する(スカラー値・リスト値どちらにも適用可。
分割後の各トークンが1 class になる):

```yaml
tags: { as: class, split: whitespace }
```

```yaml
# Obsidian 側
---
tags: ["clippings RAG tools"]
---
```
```yaml
# SML 側(生成されるフロントマター)
---
title: ...
class: [clippings, RAG, tools]
---
```

`split` を指定しないキー(既定)は、リストの各要素をそのまま1 class として
扱う(2.1 冒頭の挙動)。`split: whitespace` は空白分割のみを行う固定コンビ
ネータであり、任意の区切り文字やスクリプトは持たない(D32 の運用に整合)。

### 2.2 `as: record` — 記述的事実

本文 H1 の直後に `::record` ブロックとして持ち上げる。`key:` で SML 側の
キー名を指定できる(日本語可、D28)。`type:` で値の型付きパースのヒントを
与えられる(date/ref。省略時は text)。

```yaml
author:    { as: record, key: 著者 }
source:    { as: record, key: 出典, type: ref }
created:   { as: record, key: 作成日, type: date }
published: { as: record, key: 公開日, type: date }
description: { as: record, key: 概要 }
```

複数キーが `as: record` なら、1つの `::record` ブロックにまとめる
(1文書1メタデータブロックが既定。裁量で分けてもよい)。

**非スカラー値(リスト・ネスト)の扱い**: v0 は素朴に区切り文字で連結して
Text 化する(既定 `separator: ", "`。裁量で上書き可)。構造を保ちたい場合は
`as: class`(リストのまま複数 class に展開できる)を使うか、`as: ignore` で
見送る。

### 2.3 `as: ignore` — 実行条件・プラグイン設定・意味を持たせないもの

変換しない。元ファイルには残る(D60 により Warning のみで fmt/build は
通る)が、SML ドラフト生成時にはこのキーを記述的データとして持ち上げない。

```yaml
"excalidraw-*": { as: ignore }   # プレフィックス一致(グロブ)
ref-from: { as: ignore }
default: { as: ignore }
```

### 2.4 ファイル/フォルダ単位の除外

マッピングとは別に、変換対象そのものから除外するファイル(`.excalidraw.md`
など、Strata の文書モデルに意味的に合わないもの)は `strata.toml` の
`members` グロブで最初から拾わない(ワークスペース定義の仕事、D41)。

## 3. 実例(kurochanBrainPrivate の実データ調査より)

```yaml
version: 1

frontmatter:
  tags: { as: class, split: whitespace }

  author:      { as: record, key: 著者 }
  source:      { as: record, key: 出典, type: ref }
  created:     { as: record, key: 作成日, type: date }
  published:   { as: record, key: 公開日, type: date }
  description: { as: record, key: 概要 }

  status: { as: class }   # タスク管理ノート。分類として扱う判断(裁量の余地あり)
  due:    { as: record, key: 期日, type: date }

  "excalidraw-*": { as: ignore }
  ref-from: { as: ignore }
  data: { as: ignore }

  default: { as: ignore }
```

**`split: whitespace` が必要になった実例**(kurochanBrainPrivate の Web
クリップ由来ファイルより、D63)。Web クリッパープラグインが1リスト項目に
複数語を空白区切りで詰めていたケース:

```yaml
# Obsidian 側の frontmatter
---
tags: ["clippings RAG tools"]
---
```

`tags: { as: class }`(`split` なし)のまま変換すると `class: ["clippings RAG tools"]`
となり、空白を含む値が `class:` の字句規則に反して `strata build` が
`BadClass` で落ちる。`tags: { as: class, split: whitespace }` にすることで
正しく3つの class に展開される:

```yaml
# SML 側(生成されるフロントマター)
---
title: ...
class: [clippings, RAG, tools]
---
```

## 4. 品質基準

- 実データの frontmatter キー頻度調査(`grep`/`awk` 等で実施)を先に行い、
  それを根拠にマッピングを書く。当てずっぽうで書かない
- マッピングは vault のオーナーが読んで意味が分かる長さ・粒度に保つ
  (数十キーを超えて肥大化する場合、vault 側の frontmatter 運用自体を
  見直すシグナルとして報告する)
