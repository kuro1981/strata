# SML 執筆ガイド(AI エージェント向け)

対象読者: SML(Strata Markup Language)文書を**書く・編集する AI エージェント**
(Claude に限らない)。このガイド**だけ**を読めば SML を正しく書けることを
品質基準とする(sml-spec.md §1.7 D37)。記法の正典は `docs/sml-spec.md`、ビュー
定義の正典は `docs/view-def-v1.md`。細部で迷ったら本書ではなくそちらを見ること
——本書は実用要約であり、食い違えば sml-spec.md が正しい。

## 0. 全体の流れ

```
[ドラフト SML]  ← あなたが書く/編集する。ID未確定・ラベル参照でよい
      │  strata fmt <file>    … ID を発行して逆注入する(挿入・置換のみ)
      ▼
[管理用 SML]    ← git 管理の単位。ULID+alias 併記済み
      │  strata build <file>  … パース・参照解決・不変条件検証
      ▼
[canonical グラフ]
```

あなたの仕事は「ドラフト SML を編集すること」。ID の発行と最終的な整合性検証は
CLI(`strata fmt` / `strata build`)がやる。**書いたら必ず両方を実行する**
(§4)。

---

## 1. SML 記法 実用要約

### 1.1 ブロックの種類と ID の書き方

SML ファイル1つ = ドキュメント1つ。ブロックの列であり、見出しレベルが
`contains` のネストを作る(`##` は直前の `#` の子)。ID の書き方は2クラス:

| クラス | ブロック | ID の書き方 |
|---|---|---|
| **行型** | 見出し、リスト項目、フェンスマーカー(`::table` 等)、コードフェンス開始行 | 行末に `{#...}` |
| **プローズ** | 段落、**リスト全体** | 直前の独立行 `[id=..., ...]`(属性行) |

リストは二層: リスト**全体**の ID は前置属性行、各**項目**の ID は行内 `{#...}`。

ドラフトでは ID を省略してよい(無ければ `strata fmt` が新規発行する)。人間可読な
仮ラベルを付けたい場合は `{#my-label}`(行型)または `[id=my-label]`(プローズ)。
**ULID を自分で書かない・発明しない**(詳細は §2)。

### 1.2 見出し・段落

```markdown
# 職務経歴書

## 職務要約

通常の段落。Markdown インライン記法(`**強調**`、`` `コード` ``、`$TeX$`)が使える。
```

### 1.3 リスト(ネスト可)

```markdown
- トップ項目 {#01KXxxx1}
  - ネストした子項目(2スペースインデント) {#01KXxxx2}
  - もう1つの子項目 {#01KXxxx3}
- 次のトップ項目 {#01KXxxx4}
```

- 項目=段落1つの制約は維持(1項目に複数ブロックは書けない)。子リストのネストは可
- 長い項目は折り返してよい(D52、CommonMark 準拠の継続行): マーカー行の直後に
  空行を挟まず続く行は、同じ項目の本文として併合される(新規マーカー行が来た
  時点でそこは子項目/次の項目になる)。`{#id}` タグは**最終行**の末尾に付く:

```markdown
- 長い説明が折り返して
  次の行に続く1項目 {#01KXxxx5}
```

- リスト全体に ID/意味エッジを付けたいときだけ前置属性行を書く:

```markdown
[id=my-list, supports=some-target]
- 項目1 {#...}
- 項目2 {#...}
```

### 1.4 属性行 `[key=value, ...]`

直後のブロックに束縛される独立行。使えるキー:

| キー | 意味 |
|---|---|
| `id` | プローズブロックの ID(§1.1)。**プローズブロックの属性行専用**。行型ブロックの前置属性行に `id=` を書くとエラー(`{#...}` を使うこと) |
| `alias` | エイリアス(fmt が注入。手で書くのは主にドラフトの仮ラベルを直接 alias にしたい時ではなく、通常は `id=` にラベルを書けば fmt が `alias=` に昇格させてくれる) |
| `class` | 意味分類タグ(自由な key 字句、複数可 `class=[a, b]`)。「誰に見せるか」ではなく「これは何であるか」を書く場所(例: `class=note` = 非公開の補足) |
| `supports` / `depends-on` / `cites` | 意味エッジ(§2 参照) |

上記以外のキーは診断 `UnknownAttrKey`(Warning、ファイルは書き換わるがタイポの
可能性として警告される)。

```markdown
[id=key-finding, supports=eval-table, class=note]
このブロックが eval-table を支持することを宣言しつつ note 分類も付ける段落。
```

**D46(2026-07-15 裁定、sml-spec.md §1.11)**: 実効 class は「自身+祖先
(contains 上流)の和集合」——コンテナ(見出し・リスト・引用)に付けた class は
配下のブロック全部へ自動的に継承される。**複数ブロックにまたがる note は
コンテナに class を1回書けば十分**。1段落ごとに `[class=note]` を繰り返さない:

```markdown
[class=note]
##### 【補足】未来の自分へのメモ：ソリューション立ち上げの経緯 {#01K...}

- 初期（ダイキン時代）：もともとは… {#01K...}
- 転機（キカガクとの協業）：… {#01K...}
- 市場ニーズへの適応：… {#01K...}
```

上記1ブロックで、見出し配下のリスト項目・段落は render --hide note /
context --class note の両方で「見出しの class を継承した note」として扱われる
(見出し自身にも `[class=note]` の直後に `{#...}` の ULID が付く。属性行は
`id=` を書けないので `class` だけの行にする)。1ブロックで完結する単発の note
(例: 「実際の客先は◯◯」の1文だけの補足)はこの限りでなく、従来どおり
そのブロック単体に `class=note` を付ければよい——連打の対象はあくまで
「本来1つの塊であるはずの内容が、段落ごとに `[class=note]` を繰り返す形で
分断されている」場合。

### 1.5 フロントマター(任意)

```markdown
---
id: 01J2T8Z0000000000000000000
title: 職務経歴書
---
```

- キーは `id`(ULID)・`title`(自由文字列)・`alias`(文書エイリアス、
  ワークスペースの横断参照 `ref:<文書alias>/<...>` の左辺。D41)の3つ。
  他のキーは診断(未知キー扱い)
- 通常は `strata fmt` が既存ファイルに無ければ自動生成する。あなたが手で
  `id:` を書く必要はない(書くなら ULID のみ。ラベルは書けない)

### 1.6 `::table` — 多次元表

```markdown
::table {#eval-table}
[caption="モデル別・データセット別の性能比較"]

@rows:
  - model: [Baseline-v1, Opt-v2]

@cols:
  - dataset:
    - Dataset-A:
      - metric: [F1-Score, Latency]
    - Dataset-B:
      - metric: [F1-Score, Latency]

@cells:
  Baseline-v1 | Dataset-A.F1-Score : 0.82
  Opt-v2      | Dataset-A.Latency  : 12 ms
::
```

- `- <次元名>:` の直下が member。`- <次元名>: [k1, k2]` はフラット次元の糖衣
- member をネストすると多層の行/列次元になる(会社×プロジェクトのような二階層)
- member に表示名を付けたい時: `- key "表示ラベル"`(例: `- languages "プログラミング言語"`)
- セル座標は `行path | 列path : 値`。path は `key("."key)*`(ネスト次元はドット連結、
  例 `dentsu.isid_azure_ml_prod`)。`|`/`.`/`:` の前後の空白は無視されるので
  桁揃えしてよい
- セル値の型付きパース: 裸の数値 → Number、`<数値> <単位>`(例 `12 ms`) →
  Quantity(構造化)、`"..."` またはそれ以外の裸テキスト → Text、`~`/空 → Empty、
  `ref:<target>` → Ref
- 日付・期間セルは §1.9 参照
- `[caption=...]`、フィギュアの `[depicts=...]` はフェンス直後の**フェンス内属性行**
  に書く(この位置に `id=` は書けない。ID はマーカー行の `{#...}` のみ)

### 1.7 `::record` — キー・値ブロック

```markdown
::record {#cv-basic-info}
氏名: 黒田 裕伸
作成日: 2026-06-23
::
```

- 本体は「キー: 値」の行の列。**キーは自由テキスト(日本語可)** — 表の座標キー
  (ASCII 限定)とは別物
- 値の型付きパースは表セルと同じ(Number/Quantity/Date/Period/Text/Ref)
- 同一キーの重複宣言は診断 `DuplicateRecordKey`(Warning。値は失われず全件残る)

### 1.8 `::math` — ブロック数式

```markdown
::math {#loss-formula}
L = \frac{1}{N} \sum_{i=1}^{N} (y_i - \hat{y}_i)^2
::
```

本文は TeX。未対応の綴りは build 時にエラーになる(「出たら足す」方針、正典は
strata-spec.md §6)。

### 1.9 `::figure` — 図

記号図(チャート、データを焼かない):

```markdown
::figure {#perf-chart}
[kind=chart, data-ref=eval-table, mark=bar]
[encode-x="model", encode-y="Dataset-A.F1-Score"]
[depicts="Baseline-v1 と Opt-v2 の F1 スコア比較の棒グラフ。"]
[caption="モデルごとの予測精度(F1-Score)比較"]
::
```

写真(画素に意味がある):

```markdown
::figure {#photo1}
[kind=image, src="asset://photos/example.jpg"]
[alt="代替テキスト"]
[depicts.subject="...", depicts.setting="..."]
[caption="..."]
::
```

### 1.10 インライン参照(ナビゲーション、意味エッジは張らない)

```
[表示テキスト](<scheme>:<target>)
```

| scheme | 意味 |
|---|---|
| `ref:` | 任意ブロックへの汎用参照 |
| `term:` | 用語の使用(例: `[アジャイル](term:アジャイル)`)。同名の用語は build が自動で同一 Term ノードに解決する |
| `table:` / `fig:` / `math:` | 種別付き参照(build が対象ノード型を検証。型が違えば `RefTypeMismatch` エラー) |
| `cell:` | 表の特定セルへの参照: `[12 ms](cell:eval-table#Opt-v2|Dataset-A.Latency)` |
| `doc:` | 文書そのものへの参照(Document ノード直指し、D53): `[このカード](doc:typed-links)`。target は ULID か**文書 alias**(フロントマターの `alias:`)のみ——`<doc>/<alias>` のスラッシュ修飾は取らない。単一ファイルでは自文書 alias のみ解決可、他文書は `--workspace` が必要 |

- target は ULID でもエイリアスでもよい。target の前に `#` を付けない(`#` は
  `cell:` の座標区切り専用)
- 論証関係(supports 等)を張りたい場合はインライン参照ではなく §1.4 の属性行を使う
- 外部リンクは通常の Markdown 記法で書ける(M6/D40): `[text](https://…)`・
  `mailto:`・autolink `<https://…>`・画像 `![alt](url)`。エスケープ(`\*` 等)・
  取消線 `~~x~~`・GFM パイプ表・タスクリスト `- [ ]` も対応済み。
  `![alt](ref:...)`(内部参照の画像化)は未対応で明示エラーになる
- 別ファイルのブロックへの参照はワークスペース(D41〜D42)で書ける:
  `[text](ref:<文書alias>/<ブロックalias>)`(文書 alias はフロントマターで宣言)。
  これを含む文書の build/render は `--workspace <strata.toml>` が必要

### 1.11 Date・Period のセル/record 値

既定の受理書式は **ISO のみ**(`YYYY-MM-DD` または `YYYY-MM`)。それ以外の書式
(例: `2026年7月`)を書きたい場合は、そのフェンスの属性行に `date-format=` で
明示宣言する:

```markdown
::table {#career-overview}
[date-format="YYYY年M月"]
@cells:
  dentsu | period : 2020年1月 ~ 現在
::
```

期間は `A ~ B` または `A ~ 現在`(`〜`/`~` どちらも可、`現在` = to 無し)。
宣言と食い違う/レンジ外(13月等)の値は `BadDateValue` でエラーではなく Text
フォールバック(build は止まらない)。

---

## 2. D37 の作法(正典。ここが本書の核)

sml-spec.md §1.7 D37 で確定した、AI が SML を書く際の作法。以下5点は**必ず守る**:

1. **ULID を書かない・発明しない**。ID の発行は `strata fmt` の仕事。ドラフトでは
   `{#ラベル}`(行型)か無記名でよい。それらしい ULID 風の文字列を捏造しない
2. **alias を積極的に付ける**。意味のある名前(例: `proj-new-thing`)を選ぶ。
   ビューからも人からも引ける「アドレス」になる(view-def-v1.md のセレクタは
   alias を最も頑健な参照手段として使う)
3. **既存ノードへの参照・エッジは `strata context` 出力のアドレスタグから
   コピーする**。手で ULID や alias を書き写す/推測しない(§3 参照)。タグの
   記法は SML と同一(`{#ULID}` / `{#ULID alias=x}`)なのでそのまま貼り付けられる
4. **エッジは確信のあるものだけ張る。推測で張らない**。`supports` /
   `depends-on` / `cites` は、その関係が本文から明確に読み取れる時だけ書く。
   誤エッジは無エッジより害が大きい(嘘の論証関係を主張することになる)。
   迷ったら**張らずに**、最終報告で人間への提案として書く(「〜という関係が
   ありそうだが確信が持てないため未実装」)
5. **AI 下書きの専用 class は付けない**(例: `class=ai-draft` のようなタグを
   作らない)。`class` は意味分類専用の語彙であり、執筆プロセスの都合で汚染
   しない。AI が書いたかどうかのレビューは `git diff` で行う(人間のコミット
   指示が承認ゲート)

### 2.1 既存文書へ追記する時の追加作法(2026-07-15 批准追記)

6. **挿入位置は既存の並び順規約に従う**(時系列・グループ順など、context で
   観察して読み取る)。規約が読み取れない・確信が持てない場合は**末尾に追記**し、
   その判断を最終報告に書く
7. **集計・要約ブロックは変更しない**。概要表(career-overview 等)・職務要約の
   ような「他ブロックの内容を集計・要約したノード」は、詳細を追記しても
   自分では書き換えず、**更新候補として最終報告で提案**する(作法4のエッジと
   同じ確信原則の適用 — 要約の意図は著者にしか分からない)
8. **alias の命名は既存パターンを踏襲する**。`strata context` で同種ブロックの
   alias を観察し、同じ接頭辞・語彙・区切り文字で命名する(文書ごとの規約が
   一般規約に優先する)

---

## 3. 書く前に: グラフを読む

編集対象のファイルについて、書く前に `strata context` でグラフ全体・周辺を
把握する。出力は ULID アドレス可能な Markdown で、既存ノードへの参照はここに
出てくる `{#ULID}` / `{#ULID alias=x}` タグを**そのままコピー**すれば SML に
書ける(§2 の作法3)。

```bash
# 1. 文書全体を俯瞰する
strata context <file.sml>

# 2. 編集対象の周辺だけを見る(意味エッジを1ホップ辿った近傍付き)
strata context <file.sml> --node <alias-or-ULID> --hops 1

# 3. 特定の意味分類を横断して見る(例: 既存の note 一覧)
strata context <file.sml> --class note
```

- `--node` は複数指定可。既定 `--hops` は1
- 出力をファイルに保存したい場合は `-o out.md`
- 存在しない alias/ULID を指定すると exit 2 で明確なエラーになる
- 出力末尾の「エッジ」節に `supports` / `depends-on` / `cites` / `refers-to` /
  `term-ref` の一覧が出る(`contains` は見出し構造そのものに現れるので出ない)。
  新しいエッジを張る前に、対象との重複が無いかここを確認する

`--node` で対象サブツリーを見た後、**既存のセクション(会社・プロジェクト等)と
同じ形に揃えて**新しいブロックを書くのが最も安全(既存パターンの模倣)。

### 3.1 grep の代わりに `strata search` を使う

編集対象や参照先を**文字列で**探したい時(「この用語を使っている箇所は他にあるか」
「`eval-` で始まる alias にはどんなものがあるか」等)は、生ファイルへの `grep` では
なく `strata search` を使う(D56、sml-spec.md §1.16)。grep と違い、構造述語・
実効 class(D46: 自身+祖先の和集合)・人間可読ラベル・マッチ箇所のスニペットが
最初から付いて返ってくる:

```bash
# 素のテキスト(空白区切りで AND、CJK は部分文字列一致)
strata search "アジャイル 開発" <file.sml>

# 構造述語: class:<タグ> / term:<用語> / alias:<接頭辞>(混在可)
strata search "class:note" <file.sml>
strata search "term:アジャイル" <file.sml>
strata search "alias:eval-" <file.sml>

# ワークスペース全体を横断
strata search "class:note" --workspace <strata.toml>

# エディタ・他ツール向け構造化出力
strata search "アジャイル" <file.sml> --json
```

ヒットはブロック(ノード)単位。各ヒットに ID・alias・人間可読ラベル・スニペット
(マッチ箇所を `[[...]]` で囲む)・所属文書が付く。既存ノードへの参照を書く時は
`strata context` のアドレスタグと同じ流儀(alias があれば alias、無ければ ULID)を
そのまま使えばよい。`--limit N` で件数を絞れる(既定20)。

---

## 4. 書いた後: 必須検証シーケンス

SML を編集したら、**必ず**以下を順番に実行する。診断が出たら自分で解消して
から人間レビューに出す(黙って警告付きのまま渡さない。ただし Warning は
「全か無か」の対象外なので fmt/build 自体は成功する — それでも内容を読んで
判断すること)。

```bash
# 1. フォーマット(ID発行・逆注入。挿入と {...}/[...] 内置換のみ)
strata fmt <file.sml>

# 2. ビルド(パース・参照解決・不変条件検証。診断ゼロを確認)
strata build <file.sml>

# 3. (このプロジェクトにビュー定義があるなら)
strata view <file.sml> --view <def.yaml> --check
```

### exit code の意味

| コマンド | 0 | 1 | 2 |
|---|---|---|---|
| `fmt` | 成功(書き込み済み。変更不要でも0) | `--check` 指定時: 変更が必要(ファイル未変更) | パースエラー(ファイル未変更、全診断を報告) |
| `build` | 成功(グラフJSON出力。Warning があってもここ) | (通常は使わない: IO失敗時のみ) | ビルドエラー(全か無か。全診断を報告、出力なし) |
| `view --check` | クリーン(未充足スロット・未使用ノードなし) | 診断あり(Warning/MissingSlot/UnusedNode) | マニフェスト自体のエラー |

- **Warning**(`DuplicateFrontmatterKey` / `UnknownAttrKey` / `DuplicateRecordKey`)
  は「全か無か」の対象外 — fmt/build は成功し、stderr に警告として出るだけ
  (exit 0)。とはいえタイポのサインであることが多いので中身を確認すること
- **Error**(上記以外の診断すべて)は1件でもあると fmt/build がファイルに
  一切触れず exit 2 で全件報告する。位置(行:列)とメッセージが出るので該当
  箇所を直して再実行する
- `strata fmt` は**冪等**: 2回連続で実行して2回目に差分が出ないことを確認する
  と安全(`fmt` → `fmt --check` で exit 0 になるはず)。差分が出続ける場合は
  ドラフトの記法自体を見直す
- 文書を**コピーして**検証する場合(実文書を汚さない編集リハーサル等)、
  ビュー定義・マニフェストは相対パスで参照し合うため、`views/`・`manifests/`
  ディレクトリごとコピーすれば `view --check` まで実行できる

### 診断メッセージの実例

診断は `行:列: 種別: メッセージ` 形式で stderr に出る(位置を持たないものは `-:-`):

```
42:3: UnresolvedAlias: 参照先 'proj-new-thin' が見つかりません。
17:1: MissingId: このブロックには ID が付与されていません。`strata fmt` を先に実行してください。
88:5: warning: UnknownAttrKey: 属性キー 'clas' は既知のキー(supports/depends-on/cites/id/alias/class)ではありません(タイポの可能性。エッジは張られません)
```

1行目のような未解決参照は alias のタイポが典型原因(context 出力からコピーし
直す)。3行目のような Warning は exit 0 だが、たいていタイポのサインである。

---

## 5. チェックリスト(書き終えたら)

- [ ] ULID を自分で書いていないか(捏造していないか)
- [ ] 新規ブロックに意味のある alias を付けたか
- [ ] 既存ノードへの参照・エッジは `strata context` 出力のタグからコピーしたか(手打ちで推測していないか)
- [ ] 張ったエッジ(`supports`/`depends-on`/`cites`)はすべて確信があるか(迷ったものは張らずに報告へ)
- [ ] AI 下書き専用の class を付けていないか
- [ ] `strata fmt <file>` を実行し、再実行しても差分が出ない(冪等)ことを確認したか
- [ ] `strata build <file>` の診断がゼロ(Error 無し)であることを確認したか。Warning が出た場合は内容を確認したか
- [ ] (ビュー定義があるプロジェクトなら)`strata view --check` を実行したか
- [ ] 推測で張らずに見送ったエッジ候補があれば、最終報告に人間への提案として書いたか
