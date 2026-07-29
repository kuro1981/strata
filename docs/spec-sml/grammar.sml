---
id: 01KYP3EMGW72H78J431YMDVZMN
title: SML (Strata Markup Language) 仕様 v0.1
alias: grammar
---

# SML (Strata Markup Language) 仕様 v0.1 {#01KYP3EMGW72H78J431YMDVZMP}

[id=01KYP3EMGW72H78J431YMDVZMQ, alias=grammar-canonical-notice]
> [id=01KYP3EMGW72H78J431YMDVZMR]
> 本書(`docs/spec-sml/grammar.sml`)は SML の文法・処理パイプライン・凍結/保留事項の正典である。設計決定の記録(D1〜D63・P1〜P4)は `docs/spec-sml/decisions.sml` を参照。`strata-spec.md` §10、`strata_markup_design_notes.md`、`sml_example_*.sml` と食い違う場合は本書(decisions.sml と併せて)を正とする。それらの旧記述は本書確定後に順次追従させる。

[id=01KYP3EMGW72H78J431YMDVZMS, alias=grammar-intro]
SML は Strata 3層モデルの **層1(オーサリング表面)** を担うテキストフォーマット。人間と AI が書きやすく、`strata fmt` / `strata build` を通じて層2(canonical グラフ)へロスレスに落ちることを目的とする。基本は Markdown (CommonMark) 互換で、Strata 固有の構造(ID・意味エッジ・多次元表・数式・図)をアノテーションとして追加する。

## 文書モデルとブロック分類 {#01KYP3EMGW72H78J431YMDVZMT alias=doc-model}

[id=01KYP3EMGW72H78J431YMDVZMV, alias=doc-model-intro]
SML ファイル1つ = ドキュメント1つ。ファイル内はブロックの列であり、各ブロックが canonical の Node に対応する。見出しのレベルが `contains` のネストを作る(`##` セクションは直前の `#` セクションの子)。

[id=01KYP3EMGW72H78J431YMDVZMW, alias=doc-model-block-classes]
ブロックは ID 記法の観点から2クラスに分かれる([D2](ref:decisions/d2)):

[id=01KYP3EMGW72H78J431YMDVZMX]
| クラス | ブロック | ID の書き方 |
|---|---|---|
| **行型** | 見出し、リスト項目、フェンスマーカー(`::table` 等)、コードフェンス開始行(D10) | 行内の `{#...}` タグ |
| **プローズ** | 段落、**リスト全体**(D11) | 直前の属性行 `[id=...]` |

[id=01KYP3EMGW72H78J431YMDVZMY, alias=doc-model-list-structure]
リストは二層構造になる: リスト**全体**の ID は前置属性行(プローズ扱い)、各**項目**の ID は行内 `{#...}`(行型)。canonical では List ノードが項目 Para を contains する。

### フロントマター(D12) {#01KYP3EMGW72H78J431YMDVZMZ alias=frontmatter}

[id=01KYP3EMGW72H78J431YMDVZN0, alias=frontmatter-intro]
ファイル先頭(1バイト目)が `---` 単独行で始まる場合、次の `---` 単独行までをフロントマターとして解釈する。中身は YAML 風の `key: value` 行(自前の最小実装。ネスト・リスト・引用符エスケープは持たない):

```markdown {#01KYP3EMGW72H78J431YMDVZN1}
---
id: 01J2T8Z0000000000000000000
title: 機械学習モデルの評価レポート
---
```

[id=01KYP3EMGW72H78J431YMDVZN2, alias=frontmatter-keys]
- キーは v0 では `id`(ULID、任意)・`title`(自由文字列、任意)・`alias`(文書エイリアス、key 字句、任意。[D41](ref:decisions/d41) — ワークスペースの横断参照 `ref:<文書alias>/<...>` の左辺になる)・`class`([D46](ref:decisions/d46) 実効 class の値構文、複数可。[D61](ref:decisions/d61) — 文書全体に効く分類)の4つ。未知キーは診断(「出たら足す」方針。`UnknownFrontmatterKey`、[D60](ref:decisions/d60) により**`Warning`**。[D17](ref:decisions/d17) の `UnknownAttrKey` と同型で全か無かの対象外) {#01KYP3EMGW72H78J431YMDVZN3}
- 同一キー(`id` / `title` / `alias` / `class`)が複数行で宣言された場合、挙動は**後勝ち**(最後の宣言が採用される)のまま変えず、診断 `DuplicateFrontmatterKey`(`Warning`、[D17](ref:decisions/d17))を出す {#01KYP3EMGW72H78J431YMDVZN4}
- **`id` の値は ULID のみ**。人間ラベルは書けない(診断)。フロントマターは通常 fmt が生成するものであり、ラベル → ULID+alias の置換系をここに持ち込まない {#01KYP3EMGW72H78J431YMDVZN5}
- fmt: フロントマターが**無ければ** `id` 入りで先頭に生成(挿入のみ)。**あって `id` が無ければ** `id: <ULID>` 行を `---` の直後に挿入 {#01KYP3EMGW72H78J431YMDVZN6}
- build: フロントマターがあれば **Document ノード**(canonical)を作り、全トップレベルブロック(H1 含む)を文書順の `contains` で繋ぐ。無ければ Document ノードは作らない(フォレスト)。ファイル → 文書の対応管理は将来の vault 層の仕事 {#01KYP3EMGW72H78J431YMDVZN7}
- 閉じ `---` が無い場合は診断(全か無かにより fmt/build は動かない) {#01KYP3EMGW72H78J431YMDVZN8}

## ID とエイリアス(D3) {#01KYP3EMGW72H78J431YMDVZN9 alias=ids-aliases}

### ID タグ {#01KYP3EMGW72H78J431YMDVZNA alias=id-tags}

``` {#01KYP3EMGW72H78J431YMDVZNB}
{#<ULID>}
{#<ULID> alias=<エイリアス>}
```

[id=01KYP3EMGW72H78J431YMDVZNC, alias=id-tags-notes]
- ドラフト段階では人間ラベルを直接書いてよい: `{#eval-table}` {#01KYP3EMGW72H78J431YMDVZND}
- `strata fmt` は非 ULID のラベルを検出すると、ULID を発行して `{#01J2T8V... alias=eval-table}` に置換する(**エイリアスは消えない**) {#01KYP3EMGW72H78J431YMDVZNE}
- エイリアスの字句は key と同じ `[A-Za-z0-9_-]+` {#01KYP3EMGW72H78J431YMDVZNF}
- **alias を併記できるのは ULID の id だけ**。`{#label alias=x}`(非 ULID + alias)は診断 `AliasWithoutUlid`([P3](ref:decisions/p3))。ドラフトでは `{#label}` とだけ書き、fmt がラベルを alias へ昇格させる {#01KYP3EMGW72H78J431YMDVZNG}
- エイリアスのスコープは**ファイル内**。ファイル横断の参照は ULID で行う(グローバルエイリアス表は保留 §10) {#01KYP3EMGW72H78J431YMDVZNH}

### 属性行での ID(プローズブロック用) {#01KYP3EMGW72H78J431YMDVZNJ alias=attr-line-id}

```markdown {#01KYP3EMGW72H78J431YMDVZNK}
[id=01J2T8Z6..., supports=eval-table]
予測精度はモデルの実用性を担保するために最も重要な指標であり、…
```

[id=01KYP3EMGW72H78J431YMDVZNM, alias=attr-line-id-notes]
- ドラフトでは `[id=my-label]` と書いてよい。fmt が `[id=01J2..., alias=my-label]` に置換 {#01KYP3EMGW72H78J431YMDVZNN}
- ID の無い段落には、fmt が直前に `[id=<新規ULID>]` 行を**挿入**する {#01KYP3EMGW72H78J431YMDVZNP}
- `id` の値は**裸トークン(ULID またはラベル)のみ**([P2](ref:decisions/p2))。引用符付き(`[id="..."]`)・リスト(`[id=[a, b]]`)は診断 `BadIdValue`。ラベルの字句は key と同じ `[A-Za-z0-9_-]+` で、違反は `BadKeyCharset` {#01KYP3EMGW72H78J431YMDVZNQ}

### リスト項目(行型) {#01KYP3EMGW72H78J431YMDVZNR alias=list-item-id}

```markdown {#01KYP3EMGW72H78J431YMDVZNS}
- 評価は2軸で行う {#01J2T8X0...}
- 再現性は別レポートで扱う {#01J2T8X1...}
```

[id=01KYP3EMGW72H78J431YMDVZNT, alias=list-item-id-p1]
fmt は ID の無い項目の行末に ` {#ULID}` を追記する。項目にも安定 ID を与えるのは不変条件1(ID 安定)のため。§2.4 の需要駆動昇格は「anchor ノード化」の機構であり、「ID を持つか」とは独立の話として切り分ける。

[id=01KYP3EMGW72H78J431YMDVZNV, alias=list-item-id-continuation]
**継続行(lazy continuation、[D52](ref:decisions/d52))**: マーカー行の直後、空行・新規マーカー行・他ブロック開始行を挟まずに続く非マーカー行は、CommonMark 準拠で項目本文へ併合する(インデント継続・lazy 継続の両方)。「項目=段落1つ」の制約は維持——その段落が複数ソース行にまたがれるようになるだけ:

```markdown {#01KYP3EMGW72H78J431YMDVZNW}
- 評価は2軸で行う。長い説明が
  折り返して次の行に続くこともある {#01J2T8X0...}
- 再現性は別レポートで扱う {#01J2T8X1...}
```

[id=01KYP3EMGW72H78J431YMDVZNX, alias=list-item-id-p2]
fmt の ` {#ULID}` タグは項目の**最終行**(継続行があればその末尾)に付く(Setext 見出し・[D40](ref:decisions/d40) と同型)。空行は従来どおりブロック境界であり、継続行にはならない。

[id=01KYP3EMGW72H78J431YMDVZNY, alias=list-item-id-p3]
リスト**全体**の ID は前置属性行 `[id=...]` で与える([D11](ref:decisions/d11)、2026-07-14 改定)。canonical の List ノード(ordered 情報を持ち項目を contains する)の ID がこれに対応する。fmt は ID の無いリストの直前に `[id=<新規ULID>]` 行を挿入する。

### canonical との関係 {#01KYP3EMGW72H78J431YMDVZNZ alias=canonical-relation}

[id=01KYP3EMGW72H78J431YMDVZP0, alias=canonical-relation-p]
エイリアスは**層1の道具**である。`strata build` はエイリアス→ULID の解決表を構築して参照を解決するが、canonical グラフ(Node/Edge)には ULID しか入らない。

## 属性行と意味エッジ(D1) {#01KYP3EMGW72H78J431YMDVZP1 alias=attrs-edges}

[id=01KYP3EMGW72H78J431YMDVZP2, alias=attrs-edges-intro]
属性行 `[key=value, ...]` は**直後のブロック**に束縛される。どのブロッククラスにも前置できるが、`id` を書けるのはプローズブロックの属性行だけ(行型は `{#}` を使う。重複はエラー)。

[id=01KYP3EMGW72H78J431YMDVZP3, alias=attrs-edges-id-rules]
`id` の置き場所の規則を精密化する(v0.1 パーサ実装で確定):

[id=01KYP3EMGW72H78J431YMDVZP4]
- 行型ブロック(見出し・フェンス・コードフェンス)の前置属性行に `id=` を書くとエラー。自身の `{#...}` タグとの併記なら「重複」、単独でも「置き場所違反」として診断される {#01KYP3EMGW72H78J431YMDVZP5}
- **リスト**は前置属性行の `id=` を**許す**([D11](ref:decisions/d11)、2026-07-14 改定。M1 実装の「常に置き場所違反」を廃止): リスト全体を指す単一行が存在しないため、プローズ同様に前置属性行で ID を与える。項目の ID は従来どおり行内 `{#id}` {#01KYP3EMGW72H78J431YMDVZP6}
- **フェンス内属性行**(`::table` 直後の `[caption=...]` 等の位置)にも `id` は書けない。フェンスの ID はマーカー行の `{#...}` のみ {#01KYP3EMGW72H78J431YMDVZP7}

### 意味エッジの宣言 {#01KYP3EMGW72H78J431YMDVZP8 alias=semantic-edge-decl}

[id=01KYP3EMGW72H78J431YMDVZP9, alias=semantic-edge-decl-intro]
エッジ関係カタログ(strata-spec §4)の rel 名をそのままキーに使う:

```markdown {#01KYP3EMGW72H78J431YMDVZPA}
[id=01J2..., supports=eval-table]
特に、Dataset-A における Opt-v2 のレイテンシは 12 ms であり、…

[supports=[claim-1, claim-2], cites=izenman-2008]
複数の主張を同時に支持する段落。
```

[id=01KYP3EMGW72H78J431YMDVZPB, alias=semantic-edge-decl-notes]
- 使えるキー: `supports` / `depends-on` / `cites`(+将来追加される rel)。加えて `id` / `alias`(§3.2、fmt が注入)と `class`([D23](ref:decisions/d23)。エッジではなく意味分類。値は key 字句のタグ、単一または `[a, b]` リスト)も同じ属性行に共存できる {#01KYP3EMGW72H78J431YMDVZPC}
- 値: 単一ターゲット、または `[a, b]` のリスト {#01KYP3EMGW72H78J431YMDVZPD}
- ターゲット: ULID / エイリアス / `term:<用語名>` {#01KYP3EMGW72H78J431YMDVZPE}
- build 時に `Edge(このブロック → ターゲット, rel)` が materialise される {#01KYP3EMGW72H78J431YMDVZPF}
- 上記6キー(`supports` / `depends-on` / `cites` / `id` / `alias` / `class`)以外のキーは、エッジが張られないタイポの可能性として診断 `UnknownAttrKey`(`Warning`、[D17](ref:decisions/d17))を出す。挙動そのものは従来どおり無視のまま(build はエッジを張らない) {#01KYP3EMGW72H78J431YMDVZPG}

[id=01KYP3EMGW72H78J431YMDVZPH, alias=semantic-edge-decl-defines]
`defines` は属性キーではなく、用語定義ブロックの側から張る(§5.2 の `term:` 参照と組で設計。詳細は実装時に確定 → §10 保留)。

## インライン記法 {#01KYP3EMGW72H78J431YMDVZPJ alias=inline-notation}

### 基本(Markdown 互換) {#01KYP3EMGW72H78J431YMDVZPK alias=inline-basic}

[id=01KYP3EMGW72H78J431YMDVZPM, alias=inline-basic-p]
`**強調**`、`*イタリック*`、`` `コード` ``、`$TeX$`(インライン数式 → tex2math で MathNode 木にパース)。

### ナビゲーション参照(D1) {#01KYP3EMGW72H78J431YMDVZPN alias=nav-refs}

[id=01KYP3EMGW72H78J431YMDVZPP, alias=nav-refs-intro]
**インライン参照はすべてナビゲーション(弱参照)であり、意味エッジを張らない。** 文中の流れを止めずに書けることを優先する。

``` {#01KYP3EMGW72H78J431YMDVZPQ}
[表示テキスト](<scheme>:<target>)
```

[id=01KYP3EMGW72H78J431YMDVZPR]
| scheme | 意味 | materialise されるもの |
|---|---|---|
| `ref:` | 任意ブロックへの汎用参照 | `Edge(rel: refers-to)` |
| `term:` | 用語の使用。target は用語名またはID | `Inline::Term` + `Edge(rel: term-ref)` |
| `table:` / `fig:` / `math:` | `ref:` の種別付き糖衣。build が対象ノード型を検証 | `Edge(rel: refers-to)` |
| `cell:` | 表の特定セルへの参照(§5.3) | `Edge(rel: refers-to)`(対象は table ノード) |
| `doc:` | 文書そのものへの参照(Document ノード直指し、D53、§1.14) | `Edge(rel: refers-to)`(対象は Document ノード) |

[id=01KYP3EMGW72H78J431YMDVZPS, alias=nav-refs-notes]
- target は ULID でもエイリアスでもよい。`#` を target の前に付けない(旧例の `(table:#eval-table)` は誤り。`#` は cell 座標の区切りに予約) {#01KYP3EMGW72H78J431YMDVZPT}
- `doc:` の target は ULID か**文書 alias**のみ([D41](ref:decisions/d41) のフロントマター `alias:`)。他のスキームと違い `<文書alias>/<ブロックalias>` のスラッシュ修飾は取らない(文書そのものを指すので「文書内のブロック」という第2階層が無い)。単一ファイル build では自文書 alias のみ解決可、他文書は `--workspace` が必要(§1.10 [D41](ref:decisions/d41)〜[D43](ref:decisions/d43) と同型) {#01KYP3EMGW72H78J431YMDVZPV}
- 論証関係(supports 等)を張りたければ、ブロック属性行で宣言する(§4) {#01KYP3EMGW72H78J431YMDVZPW}
- **外部リンク**(`[表示](https://...)` 等の URL)は v0 に存在しない。`://` を含む dest はスキーム未定義として**診断なしでプレーンテキスト扱い**になる(著者のリンクが静かに不活性化するので注意。`url:` スキームの導入は保留 §10) {#01KYP3EMGW72H78J431YMDVZPX}

### セル参照 {#01KYP3EMGW72H78J431YMDVZPY alias=cell-refs}

``` {#01KYP3EMGW72H78J431YMDVZPZ}
[12 ms](cell:eval-table#Opt-v2|Dataset-A.Latency)
```

[id=01KYP3EMGW72H78J431YMDVZQ0, alias=cell-refs-p]
文法: `cell:<表のtarget>#<行path>|<列path>`。path の文法は §7。

## フェンスブロック {#01KYP3EMGW72H78J431YMDVZQ1 alias=fence-blocks}

[id=01KYP3EMGW72H78J431YMDVZQ2, alias=fence-blocks-intro]
`::<kind> {#id}` で開き、単独行の `::` で閉じる。マーカー直後に `[key=value]` 形式の**フェンス内属性行**を置ける。フェンス内では行頭 `#` をコメントとして扱う(フェンス外の文書レベルコメントは保留 §10)。

### `::table` — 多次元表(D4) {#01KYP3EMGW72H78J431YMDVZQ3 alias=table-fence}

```markdown {#01KYP3EMGW72H78J431YMDVZQ4}
::table {#01J2T8V... alias=eval-table}
[caption="モデル別・データセット別の性能比較"]

# 行軸: 実験対象のモデル(フラット次元は [...] 糖衣で書ける)
@rows:
  - model: [Baseline-v1, Opt-v2]

# 列軸: データセット × メトリクス(ネスト次元)
@cols:
  - dataset:
    - Dataset-A:
      - metric: [F1-Score, Latency]
    - Dataset-B:
      - metric: [F1-Score, Latency]

@cells:
  Baseline-v1 | Dataset-A.F1-Score : 0.82
  Baseline-v1 | Dataset-A.Latency  : 45 ms
  Opt-v2      | Dataset-A.F1-Score : 0.91
  Opt-v2      | Dataset-A.Latency  : 12 ms
::
```

[id=01KYP3EMGW72H78J431YMDVZQ5, alias=table-fence-structure]
構造の対応(strata-core §5):

[id=01KYP3EMGW72H78J431YMDVZQ6]
- `- <次元名>:` が `Dim`。直下の項目が `Member` の列 {#01KYP3EMGW72H78J431YMDVZQ7}
- member 行 `- <key>:` の下に次元があれば入れ子(`Member.children`) {#01KYP3EMGW72H78J431YMDVZQ8}
- `- <次元名>: [k1, k2, ...]` はフラット次元の糖衣 {#01KYP3EMGW72H78J431YMDVZQ9}
- member に表示名を付ける場合: `- q1 "第1四半期"`(key の後に引用符で label。初版案) {#01KYP3EMGW72H78J431YMDVZQA}

[id=01KYP3EMGW72H78J431YMDVZQB, alias=table-fence-cell-parsing]
**セル値の型付きパース規則([D4](ref:decisions/d4)):**

[id=01KYP3EMGW72H78J431YMDVZQC]
| 書き方 | パース結果 |
|---|---|
| `0.82`, `-3`, `1e5` | Number |
| `45 ms`(数値 + 空白 + 単位トークン) | 数量: 値 45 / 単位 "ms" として構造化(canonical 表現は §9-3)。単位トークンの字句は `[A-Za-zµ%°]` で始まり `[A-Za-z0-9/^·%°-]*` が続く1トークン(v0.1 で確定) |
| `"任意の テキスト"` | Text |
| 裸の非数値テキスト | Text(寛容にフォールバック) |
| `~` または空 | Empty |
| `ref:<target>` | CellValue::Ref(value ノード等との共有) |

### `::math` — ブロック数式 {#01KYP3EMGW72H78J431YMDVZQD alias=math-fence}

```markdown {#01KYP3EMGW72H78J431YMDVZQE}
::math {#01J2T8ZE... alias=loss-formula}
L = \frac{1}{N} \sum_{i=1}^{N} (y_i - \hat{y}_i)^2
::
```

[id=01KYP3EMGW72H78J431YMDVZQF, alias=math-fence-p]
本文は TeX。`strata build` が tex2math で MathNode 木(MathML サブセット)へパースする。パース不能な綴りは `UnknownCommand` エラー(出たら足す方針、strata-spec §6)。

### `::figure` — 図 {#01KYP3EMGW72H78J431YMDVZQG alias=figure-fence}

[id=01KYP3EMGW72H78J431YMDVZQH, alias=figure-fence-chart-intro]
記号図(chart。データを焼かない):

```markdown {#01KYP3EMGW72H78J431YMDVZQJ}
::figure {#01J2T8ZG... alias=perf-chart}
[kind=chart, data-ref=eval-table, mark=bar]
[encode-x="model", encode-y="Dataset-A.F1-Score"]
[depicts="Baseline-v1 と Opt-v2 の Dataset-A における F1 スコア比較の棒グラフ。"]
[caption="モデルごとの予測精度(F1-Score)比較"]
::
```

[id=01KYP3EMGW72H78J431YMDVZQK, alias=figure-fence-photo-intro]
写真(画素に意味がある):

```markdown {#01KYP3EMGW72H78J431YMDVZQM}
::figure {#01J2...}
[kind=image, src="asset://photos/2026-ski-hakuba.jpg"]
[alt="雪山でスキーをする人物"]
[depicts.subject="...", depicts.setting="..."]
[caption="..."]
::
```

[id=01KYP3EMGW72H78J431YMDVZQN, alias=figure-fence-data-ref]
`data-ref` は ULID / エイリアスのどちらでもよい。

## 語彙的制約(D5) {#01KYP3EMGW72H78J431YMDVZQP alias=lexical-constraints}

[id=01KYP3EMGW72H78J431YMDVZQQ, alias=lexical-constraints-list]
- **key / エイリアス**: `[A-Za-z0-9_-]+`。日本語などの表示名は `label`(§6.1)やリンクの表示テキストに書く。エスケープ文法は持たない — 制限のほうが安い {#01KYP3EMGW72H78J431YMDVZQR}
- **座標**: `<行path>|<列path>`。path は `key ("." key)*`。`|` `.` `:` の前後の空白は無視(`@cells` の桁揃えのため) {#01KYP3EMGW72H78J431YMDVZQS}

## 処理パイプライン {#01KYP3EMGW72H78J431YMDVZQT alias=pipeline}

``` {#01KYP3EMGW72H78J431YMDVZQV}
[ドラフト SML] (ID未付与・ラベル参照)
      │  strata fmt … ID発行・逆注入(挿入のみ)。エイリアス・参照は温存
      ▼
[管理用 SML] (ULID+alias 併記済み)          ← git 管理の単位
      │  strata build … パース・エイリアス解決・Node/Edge構築・不変条件検証
      ▼
[canonical グラフ] (層2。ULID のみ)
```

### fmt の実装方式と契約(D6) {#01KYP3EMGW72H78J431YMDVZQW alias=fmt-contract}

[id=01KYP3EMGW72H78J431YMDVZQX, alias=fmt-contract-p1]
**スパンパッチ方式**: パーサが全ブロックのバイトオフセットを記録し、fmt は `(offset, テキスト)` のパッチ列を生成して元バイト列に後ろから適用する。元テキストの再シリアライズは行わない — 触っていないバイトは構造的に無傷。

[id=01KYP3EMGW72H78J431YMDVZQY, alias=fmt-contract-p2]
fmt が行う変更は4種類だけ([D10](ref:decisions/d10)〜[D12](ref:decisions/d12) で対象を拡張。2026-07-14 改定):

[id=01KYP3EMGW72H78J431YMDVZQZ, alias=fmt-contract-list]
1. 行型ブロック(見出し・リスト項目・フェンスマーカー・コードフェンス開始行)の行末に ` {#ULID}` を**追記** {#01KYP3EMGW72H78J431YMDVZR0}
2. ID の無いプローズブロック(段落・リスト全体)の直前に `[id=ULID]` 行を**挿入** {#01KYP3EMGW72H78J431YMDVZR1}
3. 非 ULID ラベルの `{#label}` / `[id=label]` を `{#ULID alias=label}` 形式に**置換**(`{...}` / `[...]` の内側のみ) {#01KYP3EMGW72H78J431YMDVZR2}
4. フロントマターが無ければ先頭に `id` 入りで**生成**、あって `id` が無ければ `id: <ULID>` 行を**挿入**(§2.1。どちらも純粋な挿入) {#01KYP3EMGW72H78J431YMDVZR3}

[id=01KYP3EMGW72H78J431YMDVZR4, alias=fmt-contract-p3]
契約(テストで固定する受け入れ条件):

[id=01KYP3EMGW72H78J431YMDVZR5, alias=fmt-contract-conditions]
- **冪等性**: `fmt(fmt(x)) == fmt(x)` {#01KYP3EMGW72H78J431YMDVZR6}
- **挿入のみ**: fmt 前後の diff は上記3種の追記・挿入・囲み内置換のみ {#01KYP3EMGW72H78J431YMDVZR7}
- **意味保存**: `build(fmt(draft))` ≅ `build(formatted)`(同型のグラフ)。build 未実装の間は「パース結果の**ID無視同型**」で代用する。**ID情報**の定義([P1](ref:decisions/p1))= 行末 `{#...}` タグ全体(alias 含む)+ 属性行の `id`・`alias` エントリ + 全スパン値。これらを消した正規化構造が一致すること {#01KYP3EMGW72H78J431YMDVZR8}
- **原子性**: 一時ファイルに書いて rename。途中状態をディスクに残さない {#01KYP3EMGW72H78J431YMDVZR9}

### エラー方針 {#01KYP3EMGW72H78J431YMDVZRA alias=error-policy}

[id=01KYP3EMGW72H78J431YMDVZRB, alias=error-policy-p1]
**全か無か**: 1箇所でもパースエラーがあれば、fmt はファイルに一切触れずエラー位置(スパン)を報告して終了する。「半分だけ処理された状態」を作らない。build も同じ方針([D13](ref:decisions/d13))。

[id=01KYP3EMGW72H78J431YMDVZRC, alias=error-policy-p2]
**Error のみが対象([D17](ref:decisions/d17)、2026-07-14)**: 診断には severity(`Error`/`Warning`)があり(§1.2b)、「全か無か」は **`Error` にのみ適用**する。`Warning` 診断(`DuplicateFrontmatterKey`・`UnknownAttrKey`)だけの入力は fmt/build とも**成功**し、その `Warning` を処理結果と併せて呼び出し側に返す。CLI(`fmt`/`build` 両方)は `Warning` を「`行:列: warning: 種別: メッセージ`」形式で stderr に出しつつ exit 0 で終了する(`Error` が1件でもあれば従来どおりファイルには一切触れず exit 2、全診断を報告)。

### rowan / CST への乗り換えトリガ {#01KYP3EMGW72H78J431YMDVZRD alias=rowan-trigger}

[id=01KYP3EMGW72H78J431YMDVZRE, alias=rowan-trigger-p]
次のいずれかが要件になった時点で再評価する。それまではパッチ方式を維持:

[id=01KYP3EMGW72H78J431YMDVZRF, alias=rowan-trigger-list]
1. fmt に**整形機能**(表の桁揃え・インデント正規化)を持たせたくなった {#01KYP3EMGW72H78J431YMDVZRG}
2. **エディタ統合(LSP)** — 打鍵中の壊れた文書への増分再パースが必要になった {#01KYP3EMGW72H78J431YMDVZRH}
3. オフセット管理のバグが繰り返し出て負債化した {#01KYP3EMGW72H78J431YMDVZRJ}

## strata-core への波及(必要な拡張) {#01KYP3EMGW72H78J431YMDVZRK alias=core-extensions}

[id=01KYP3EMGW72H78J431YMDVZRM, alias=core-extensions-intro]
本仕様の実装に必要な strata-core 拡張。**方針は [D8](ref:decisions/d8)/[D12](ref:decisions/d12) で確定済み(2026-07-14)、実装は M3**:

[id=01KYP3EMGW72H78J431YMDVZRN, alias=core-extensions-list]
1. **`Rel::RefersTo` の追加** — インラインのナビゲーション参照(§5.2)が materialise する弱参照。現行カタログには存在しない(現実装は `DependsOn` に誤って畳んでいる) {#01KYP3EMGW72H78J431YMDVZRP}
2. **セル参照の座標保持** — `Inline::Ref` に `coord: Option<CellCoord>` を追加(serde では省略可能に)。`CellCoord` は `{row_path, col_path}`(strata-core 側に定義。strata-sml とは依存しないため型は別) {#01KYP3EMGW72H78J431YMDVZRQ}
3. **数量(数値+単位)の canonical 表現** — `CellValue::Quantity { v, unit }` を追加。SML の型付きパース結果([D4](ref:decisions/d4))と 1:1 対応。prose と共有したい値だけを明示的に `Value` ノード + `CellValue::Ref` にする道は従来どおり残す {#01KYP3EMGW72H78J431YMDVZRR}
4. **エイリアス解決表** — build が保持する層1の道具。canonical グラフには入れない {#01KYP3EMGW72H78J431YMDVZRS}
5. **`Document` ノードの追加**([D12](ref:decisions/d12)) — フロントマターに対応する文書ルート。`title: Option<String>` を持ち、トップレベルブロックを contains する {#01KYP3EMGW72H78J431YMDVZRT}
6. **Term ノードの安定 ID 導出**([D9](ref:decisions/d9)、[D15](ref:decisions/d15)) — build が用語名から決定的に導出した128bit を ID として Term ノードを自動生成する。導出式(v0 凍結、[D15](ref:decisions/d15)、2026-07-14): 名前を **NFC 正規化**してから `Sha256("strata:term:v0:{name}")` の先頭16バイトを読む {#01KYP3EMGW72H78J431YMDVZRV}
7. **参照の表示テキスト保持** — `Inline::Ref` / `Inline::Term` は現在 `to` しか持たず、`[レイテンシ](cell:...)` の表示テキストが落ちる。ロスレス原則の帰結として `text: String` を追加する(2026-07-14、[D8](ref:decisions/d8) の一部として確定) {#01KYP3EMGW72H78J431YMDVZRW}
8. **`Table.caption` / `Chart.depicts`**([D16](ref:decisions/d16)、2026-07-14) — `Table.caption: Option<Vec<Inline>>`(§6.1 の `[caption=...]`)、`Chart.depicts: BTreeMap<String,String>`(§6.3 の `[depicts=...]`/`[depicts.<key>=...]`。`ImageFigure.depicts` と同形・同じキー畳み規則: 裸の `depicts` は `"description"` キー、`depicts.<key>` はその `<key>` をキーにする)を追加する。両方とも後方互換フィールド(`default` + `skip_serializing_if`) {#01KYP3EMGW72H78J431YMDVZRX}

## 凍結 vs 保留 {#01KYP3EMGW72H78J431YMDVZRY alias=frozen-vs-pending}

[id=01KYP3EMGW72H78J431YMDVZRZ, alias=frozen-p1]
**凍結(本書の契約):** D1〜D63・P1〜P4 の全決定、§2.1・§3〜§8 の記法と fmt/build 契約。

[id=01KYP3EMGW72H78J431YMDVZS0, alias=frozen-p2]
**解消済み(2026-07-14):** コードフェンスの ID 記法(→ [D10](ref:decisions/d10))、文書ルート/フロントマター(→ [D12](ref:decisions/d12))、リスト全体の ID(→ [D11](ref:decisions/d11))、ネストリスト(→ [D24](ref:decisions/d24)。項目内複数ブロックは保留継続)、出し分けのブロック単位(→ [D23](ref:decisions/d23))。

[id=01KYP3EMGW72H78J431YMDVZS1, alias=pending-intro]
**保留(後で決める):**

[id=01KYP3EMGW72H78J431YMDVZS2, alias=pending-list]
- 文書レベルのコメント構文(フェンス内 `#` のみ確定。`<!-- -->` が有力候補) {#01KYP3EMGW72H78J431YMDVZS3}
- 外部リンク用スキーム(`url:` 等)の導入と、無診断フォールバックの是非(§5.2) {#01KYP3EMGW72H78J431YMDVZS4}
- member label 構文の最終形(`- key "label"` は初版案) {#01KYP3EMGW72H78J431YMDVZS5}
- `defines` エッジの SML 表現(用語定義ブロックの記法)— 当面は [D9](ref:decisions/d9) の Term 自動生成で参照側だけ成立させる。定義ブロック記法が入った時点で `defines` エッジを接続 {#01KYP3EMGW72H78J431YMDVZS6}
- ~~ファイル横断のエイリアス~~(→ [D41](ref:decisions/d41)〜[D43](ref:decisions/d43) で v0 解消。ワークスペース層 M7) {#01KYP3EMGW72H78J431YMDVZS7}
- ワークスペース v0.5: **context / render の横断**(MD ページ間リンク、横断近傍抽出)と、cross-doc 参照を含む文書の単一ファイル render の扱い([D43](ref:decisions/d43)) {#01KYP3EMGW72H78J431YMDVZS8}
- **index の永続化・インクリメンタル更新**(redb 等。[D42](ref:decisions/d42) — 大規模化時に別裁定。永続 index も常に再構築可能な派生物とし、正本性を持たせない) {#01KYP3EMGW72H78J431YMDVZS9}
- リスト項目の中の複数ブロック(項目=段落1つ、を当面の制約とする。ネストは [D24](ref:decisions/d24) で解禁) {#01KYP3EMGW72H78J431YMDVZSA}
- ~~インラインテキスト中のリテラル `[` `$` 等のエスケープ~~(→ [D40](ref:decisions/d40) で解消) {#01KYP3EMGW72H78J431YMDVZSB}
- GFM 脚注(`[^1]`。[D40](ref:decisions/d40) で保留継続 — 定義行の可視化事故だけは [D40](ref:decisions/d40) の参照スタイルリンク処理と併せて要注意) {#01KYP3EMGW72H78J431YMDVZSC}
- Obsidian 埋め込み `![[target]]`([D59](ref:decisions/d59) の wikilink は対応するが埋め込みは対象外。画像/ノート埋め込みの区別・トランスクルージョン(§10 既出項目)との関係を要検討) {#01KYP3EMGW72H78J431YMDVZSD}
- Dataview インラインフィールド(`key:: value`)・callout(`> [!note]`)— [D59](ref:decisions/d59) の実データ調査で低頻度(各1〜2件/304ファイル)を確認、実害が増えたら再検討 {#01KYP3EMGW72H78J431YMDVZSE}
- ~~リスト項目の継続行(lazy continuation)~~(→ [D52](ref:decisions/d52) で解消): 項目本文を折り返した継続行(`- 長い文…\n  続き`)が無診断で別段落ブロックに分断される(M6 監査の取りこぼし、2026-07-15 の notes ドッグフーディングで発見。②「静かに壊れる」級 — 診断を出すか継続行を項目に併合するか要裁定) {#01KYP3EMGW72H78J431YMDVZSF}
- ~~文書そのものを指す参照記法~~(→ [D53](ref:decisions/d53) で doc: スキーム採用。旧状: 「H1 に alias=top」規約で代用 — notes vault の運用規約に記載。`ref:<文書alias>` 単独で Document ノードを指せる方が自然か要裁定) {#01KYP3EMGW72H78J431YMDVZSG}
- フロントマターのキー追加(authors・date 等。v0 は id / title のみ) {#01KYP3EMGW72H78J431YMDVZSH}
- インラインの出し分け(見出し・段落内の一部スパンだけを class 分類する記法。[D23](ref:decisions/d23) のブロック単位では実名の 【実際: 〇〇】 のようなインライン情報を扱えない) {#01KYP3EMGW72H78J431YMDVZSJ}
- **値のトランスクルージョン**(`cell:` 等の参照先の**値**を本文・セルに埋め込み表示する記法。現状の参照はリンクのみで、同じ事実の二重記述を防げない — v0 でメタ行 30 件の DRY 違反として定量化。当面は「表を正として重複側を削除」で回避) {#01KYP3EMGW72H78J431YMDVZSK}
- **エンティティ**(会社・人物など同一実体の表記ゆれを束ねるノード。`term:` は用語専用。v0 では正規化+前方一致のあいまい一致で代用した) {#01KYP3EMGW72H78J431YMDVZSM}
- ~~ビュー定義の `template`/`concat` コンビネータ~~(→ [D45](ref:decisions/d45) で concat 採用。例: `"{details}({level})"`。[D35](ref:decisions/d35) で見送り — 実需が出た時点で [D32](ref:decisions/d32) の運用に従い再裁定) {#01KYP3EMGW72H78J431YMDVZSN}
- テンプレート・マニフェストの拡張(M5-C 盲検デモ 2026-07-15 の知見): per-field の**型ヒント**(`--check` は存在検査のみで `as: int` の欠落を検出できない)と per-field の**自由記述ノート**(「志望動機はデータに無ければ健康状態・趣味等から構成する」のような業務意図の伝達)。デモの結論: 「LLM 提案 → `--check` → 人間批准」は**二段ゲート**(機械検査+人間の diff レビュー)として成立(盲検で1反復収束・7ファイル中5バイト一致・値117/118一致)。`--check` 通過のみでの自動採用は不可 {#01KYP3EMGW72H78J431YMDVZSP}
- **ビュー/テンプレート層**(ドッグフーディング定性評価より): JIS 履歴書のような既存テンプレートへの pull 型流し込み(テンプレートが先にあり、グラフから埋める)、リスト/拡張表現をビュー側で表に組み替える変換、ビュー定義(体裁カスタム・class フィルタのプロファイル)を文書外ファイルとして持つ仕組み。M5(AI ビュー)と地続き。2026-07-15 の壁打ちでの骨格合意: {#01KYP3EMGW72H78J431YMDVZSQ}
  - **バインディング層**の新設 — グラフの意味構造とテンプレートのスロットスキーマの対応表(ER 図的な2スキーマ間マッピング)。CSS(流れの装飾)では足りず、XSLT / headless CMS(クエリ+コンポーネント)の系譜。ミスマッチはアドレス・粒度・多重度の3種 {#01KYP3EMGW72H78J431YMDVZSR}
  - バインディングは**データを持たない純粋な対応表**。テンプレートファースト=バインディングを逆向きに実行して SML 雛形(記入フォーム)を生成、ドキュメントファースト=既存両スキーマの対応を設計。両フローは同じ成果物を逆方向から作る。データは常に文書に着地(第二の真実の源を作らない) {#01KYP3EMGW72H78J431YMDVZSS}
  - 段階案: v0=バインディングをコードで(プロジェクトローカルなスクリプト、graph JSON → テンプレート入力データ、テンプレート無改変。セレクタと整形の語彙の棚卸しが目的)→ v1=宣言的ビュー定義に蒸留(`strata view` 的コマンド)→ v2=バインディング=スキーマとして逆向き利用(記入検証・雛形生成) {#01KYP3EMGW72H78J431YMDVZST}
  - 設計を軽くする3点: テンプレート・マニフェスト(必要スロットの宣言をデータで持つ)/ グラフ側のアドレス規約(alias・class・セル座標で引きやすい書き方)/ バインディングの検証ループ(dry-run で未充足スロット・未使用情報を診断) {#01KYP3EMGW72H78J431YMDVZSV}
  - LLM の使い所: バインディング定義(案)の提案。宣言的定義(v1)が前提 — 提案(LLM)と実行(決定的な実行器)の分離で監査可能にする。M5 の入口 {#01KYP3EMGW72H78J431YMDVZSW}

## 付録A. 文法スケッチ(EBNF 風・実装時に厳密化) {#01KYP3EMGW72H78J431YMDVZSX alias=appendix-ebnf}

```ebnf {#01KYP3EMGW72H78J431YMDVZSY}
document    = frontmatter? block* ;
frontmatter = "---" NL ( fm-key ":" SP fm-value NL )* "---" NL ;   (* ファイル先頭のみ。D12 *)
fm-key      = "id" | "title" ;                       (* 未知キーは診断。id の値は ULID のみ *)
block       = attr-line? ( heading | fence | code-fence | list-item+ | paragraph ) ;
code-fence  = "```" lang? ( SP id-tag )? NL ... "```" NL ;          (* 開始行末尾に id 可。D10 *)

attr-line   = "[" attr ( "," attr )* "]" NL ;
attr        = key "=" attr-value ;              (* id はプローズ用属性行のみ *)

heading     = "#"+ SP inline ( SP id-tag )? NL ;
list-item   = marker-line continuation-line* ;      (* id-tag は列挙する行群の**最終行**
                                                        末尾にのみ付く。D52 *)
marker-line = ( "-" | DIGITS "." ) SP inline ( SP id-tag )? NL ;
continuation-line = inline ( SP id-tag )? NL ;      (* 空行・新規マーカー行・他ブロック
                                                        開始行が来たら継続行ではない *)
id-tag      = "{#" ( ULID | label ) ( SP "alias=" label )? "}" ;

fence       = "::" kind ( SP id-tag )? NL fence-body "::" NL ;
kind        = "table" | "math" | "figure" ;

(* ::table 本体 *)
table-body  = ( fence-attr | comment | rows | cols | cells | BLANK )* ;
rows        = "@rows:" NL dim+ ;   cols = "@cols:" NL dim+ ;
dim         = INDENT "-" dim-name ":" ( SP "[" member-list "]" NL | NL member+ ) ;
member      = INDENT "-" key ( SP STRING )? ( ":" NL dim+ )? NL ;  (* STRING = label *)
cells       = "@cells:" NL cell-line* ;
cell-line   = path SP? "|" SP? path SP? ":" SP? cell-value NL ;
path        = key ( "." key )* ;
key         = /[A-Za-z0-9_-]+/ ;

(* インライン参照 *)
inline-ref  = "[" text "](" scheme ":" target ( "#" path "|" path )? ")" ;
scheme      = "ref" | "term" | "table" | "fig" | "math" | "cell" | "doc" ;
target      = ULID | alias | term-name ;        (* term-name は term: のみ。doc: の
                                                    alias は文書 alias(D53) *)
```

## 付録B. 総合例 {#01KYP3EMGW72H78J431YMDVZSZ alias=appendix-example}

[id=01KYP3EMGW72H78J431YMDVZT0, alias=appendix-example-draft-intro]
ドラフト(人間/AI が書く):

```markdown {#01KYP3EMGW72H78J431YMDVZT1}
# 評価レポート

## 分析 {#analysis}

[id=key-finding, supports=eval-table]
特に、[レイテンシ](cell:eval-table#Opt-v2|Dataset-A.Latency) は大幅に改善した。

::table {#eval-table}
@rows:
  - model: [Baseline-v1, Opt-v2]
@cols:
  - metric: [F1-Score, Latency]
@cells:
  Opt-v2 | Latency : 12 ms
::
```

[id=01KYP3EMGW72H78J431YMDVZT2, alias=appendix-example-fmt-intro]
fmt 後(挿入と `{...}`/`[...]` 内置換のみ。参照は温存):

```markdown {#01KYP3EMGW72H78J431YMDVZT3}
# 評価レポート {#01J2AAAA...}

## 分析 {#01J2AAAB... alias=analysis}

[id=01J2AAAC..., alias=key-finding, supports=eval-table]
特に、[レイテンシ](cell:eval-table#Opt-v2|Dataset-A.Latency) は大幅に改善した。

::table {#01J2AAAD... alias=eval-table}
@rows:
  - model: [Baseline-v1, Opt-v2]
@cols:
  - metric: [F1-Score, Latency]
@cells:
  Opt-v2 | Latency : 12 ms
::
```

[id=01KYP3EMGW72H78J431YMDVZT4, alias=appendix-example-build-intro]
build 後(canonical。要点のみ):

[id=01KYP3EMGW72H78J431YMDVZT5, alias=appendix-example-build-list]
- Node: Section×2, Para×1, Table×1(セル値 `12 ms` は数量として構造化) {#01KYP3EMGW72H78J431YMDVZT6}
- Edge: `contains`(見出しネストから)、`supports`(key-finding → eval-table、属性行から)、`refers-to`(key-finding → eval-table、インライン cell 参照から) {#01KYP3EMGW72H78J431YMDVZT7}
- グラフ内に alias は存在しない(ULID のみ) {#01KYP3EMGW72H78J431YMDVZT8}
