# SML (Strata Markup Language) 仕様 v0.1

> **本書が SML の正典 (normative) である。** `strata-spec.md` §10、`strata_markup_design_notes.md`、
> `sml_example_*.sml` と食い違う場合は本書を正とする。それらの旧記述は本書確定後に順次追従させる。

SML は Strata 3層モデルの **層1(オーサリング表面)** を担うテキストフォーマット。
人間と AI が書きやすく、`strata fmt` / `strata build` を通じて層2(canonical グラフ)へ
ロスレスに落ちることを目的とする。基本は Markdown (CommonMark) 互換で、Strata 固有の
構造(ID・意味エッジ・多次元表・数式・図)をアノテーションとして追加する。

---

## 1. 設計決定の記録(2026-07-13 対話にて確定)

本仕様の骨格は以下の6決定に基づく。変更する場合はここに追記して履歴を残すこと。

| # | 論点 | 決定 |
|---|---|---|
| D1 | 参照の意味をどの層で書くか | **ナビはインライン、意味はブロック属性**。インライン参照はナビゲーション(弱参照)のみ。`supports` 等の意味エッジはブロック属性行でのみ宣言する |
| D2 | ID 記法 | **行型ブロック**(見出し・リスト項目・フェンスマーカー)は行内 `{#id}`、**複数行プローズ**(段落)は直前の属性行 `[id=...]`。1ブロックに両方書いたらエラー |
| D3 | エイリアスと ULID | **併存**。エイリアスは一級市民。fmt はエイリアスを消さず ULID を追記解決する。参照はエイリアス・ULID のどちらでも書ける。canonical グラフには ULID のみが入る |
| D4 | 表ブロック構文 | ネスト次元形(§6.1)を正とする。セル値は型付きパース: 裸の数値→Number、`<数値> <単位>`→数量として構造化、その他→Text |
| D5 | member key の字句 | key は `[A-Za-z0-9_-]+` に制限。表示名は `label` に逃がす。エスケープ文法は持たない |
| D6 | fmt の実装方式 | **スパンパッチ方式**(CST/rowan は当面使わない)。fmt は挿入と `{...}` 内置換のみを行い、冪等・全か無か(§8) |

---

## 2. 文書モデルとブロック分類

SML ファイル1つ = ドキュメント1つ。ファイル内はブロックの列であり、各ブロックが
canonical の Node に対応する。見出しのレベルが `contains` のネストを作る
(`##` セクションは直前の `#` セクションの子)。

ブロックは ID 記法の観点から2クラスに分かれる(D2):

| クラス | ブロック | ID の書き方 |
|---|---|---|
| **行型** | 見出し、リスト項目、フェンスマーカー(`::table` 等) | 行内の `{#...}` タグ |
| **プローズ** | 段落 | 直前の属性行 `[id=...]` |

---

## 3. ID とエイリアス(D3)

### 3.1 ID タグ

```
{#<ULID>}
{#<ULID> alias=<エイリアス>}
```

- ドラフト段階では人間ラベルを直接書いてよい: `{#eval-table}`
- `strata fmt` は非 ULID のラベルを検出すると、ULID を発行して
  `{#01J2T8V... alias=eval-table}` に置換する(**エイリアスは消えない**)
- エイリアスの字句は key と同じ `[A-Za-z0-9_-]+`
- エイリアスのスコープは**ファイル内**。ファイル横断の参照は ULID で行う
  (グローバルエイリアス表は保留 §10)

### 3.2 属性行での ID(プローズブロック用)

```markdown
[id=01J2T8Z6..., supports=eval-table]
予測精度はモデルの実用性を担保するために最も重要な指標であり、…
```

- ドラフトでは `[id=my-label]` と書いてよい。fmt が `[id=01J2..., alias=my-label]` に置換
- ID の無い段落には、fmt が直前に `[id=<新規ULID>]` 行を**挿入**する

### 3.3 リスト項目(行型)

```markdown
- 評価は2軸で行う {#01J2T8X0...}
- 再現性は別レポートで扱う {#01J2T8X1...}
```

fmt は ID の無い項目の行末に ` {#ULID}` を追記する。項目にも安定 ID を与えるのは
不変条件1(ID 安定)のため。§2.4 の需要駆動昇格は「anchor ノード化」の機構であり、
「ID を持つか」とは独立の話として切り分ける。

### 3.4 canonical との関係

エイリアスは**層1の道具**である。`strata build` はエイリアス→ULID の解決表を構築して
参照を解決するが、canonical グラフ(Node/Edge)には ULID しか入らない。

---

## 4. 属性行と意味エッジ(D1)

属性行 `[key=value, ...]` は**直後のブロック**に束縛される。どのブロッククラスにも
前置できるが、`id` を書けるのはプローズブロックの属性行だけ(行型は `{#}` を使う。重複はエラー)。

### 4.1 意味エッジの宣言

エッジ関係カタログ(strata-spec §4)の rel 名をそのままキーに使う:

```markdown
[id=01J2..., supports=eval-table]
特に、Dataset-A における Opt-v2 のレイテンシは 12 ms であり、…

[supports=[claim-1, claim-2], cites=izenman-2008]
複数の主張を同時に支持する段落。
```

- 使えるキー: `supports` / `depends-on` / `cites`(+将来追加される rel)
- 値: 単一ターゲット、または `[a, b]` のリスト
- ターゲット: ULID / エイリアス / `term:<用語名>`
- build 時に `Edge(このブロック → ターゲット, rel)` が materialise される

`defines` は属性キーではなく、用語定義ブロックの側から張る(§5.2 の `term:` 参照と
組で設計。詳細は実装時に確定 → §10 保留)。

---

## 5. インライン記法

### 5.1 基本(Markdown 互換)

`**強調**`、`*イタリック*`、`` `コード` ``、`$TeX$`(インライン数式 → tex2math で
MathNode 木にパース)。

### 5.2 ナビゲーション参照(D1)

**インライン参照はすべてナビゲーション(弱参照)であり、意味エッジを張らない。**
文中の流れを止めずに書けることを優先する。

```
[表示テキスト](<scheme>:<target>)
```

| scheme | 意味 | materialise されるもの |
|---|---|---|
| `ref:` | 任意ブロックへの汎用参照 | `Edge(rel: refers-to)` |
| `term:` | 用語の使用。target は用語名またはID | `Inline::Term` + `Edge(rel: term-ref)` |
| `table:` / `fig:` / `math:` | `ref:` の種別付き糖衣。build が対象ノード型を検証 | `Edge(rel: refers-to)` |
| `cell:` | 表の特定セルへの参照(§5.3) | `Edge(rel: refers-to)`(対象は table ノード) |

- target は ULID でもエイリアスでもよい。`#` を target の前に付けない
  (旧例の `(table:#eval-table)` は誤り。`#` は cell 座標の区切りに予約)
- 論証関係(supports 等)を張りたければ、ブロック属性行で宣言する(§4)

### 5.3 セル参照

```
[12 ms](cell:eval-table#Opt-v2|Dataset-A.Latency)
```

文法: `cell:<表のtarget>#<行path>|<列path>`。path の文法は §7。

---

## 6. フェンスブロック

`::<kind> {#id}` で開き、単独行の `::` で閉じる。マーカー直後に `[key=value]` 形式の
**フェンス内属性行**を置ける。フェンス内では行頭 `#` をコメントとして扱う
(フェンス外の文書レベルコメントは保留 §10)。

### 6.1 `::table` — 多次元表(D4)

```markdown
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

構造の対応(strata-core §5):

- `- <次元名>:` が `Dim`。直下の項目が `Member` の列
- member 行 `- <key>:` の下に次元があれば入れ子(`Member.children`)
- `- <次元名>: [k1, k2, ...]` はフラット次元の糖衣
- member に表示名を付ける場合: `- q1 "第1四半期"`(key の後に引用符で label。初版案)

**セル値の型付きパース規則(D4):**

| 書き方 | パース結果 |
|---|---|
| `0.82`, `-3`, `1e5` | Number |
| `45 ms`(数値 + 空白 + 単位トークン) | 数量: 値 45 / 単位 "ms" として構造化(canonical 表現は §9-3) |
| `"任意の テキスト"` | Text |
| 裸の非数値テキスト | Text(寛容にフォールバック) |
| `~` または空 | Empty |
| `ref:<target>` | CellValue::Ref(value ノード等との共有) |

### 6.2 `::math` — ブロック数式

```markdown
::math {#01J2T8ZE... alias=loss-formula}
L = \frac{1}{N} \sum_{i=1}^{N} (y_i - \hat{y}_i)^2
::
```

本文は TeX。`strata build` が tex2math で MathNode 木(MathML サブセット)へパースする。
パース不能な綴りは `UnknownCommand` エラー(出たら足す方針、strata-spec §6)。

### 6.3 `::figure` — 図

記号図(chart。データを焼かない):

```markdown
::figure {#01J2T8ZG... alias=perf-chart}
[kind=chart, data-ref=eval-table, mark=bar]
[encode-x="model", encode-y="Dataset-A.F1-Score"]
[depicts="Baseline-v1 と Opt-v2 の Dataset-A における F1 スコア比較の棒グラフ。"]
[caption="モデルごとの予測精度(F1-Score)比較"]
::
```

写真(画素に意味がある):

```markdown
::figure {#01J2...}
[kind=image, src="asset://photos/2026-ski-hakuba.jpg"]
[alt="雪山でスキーをする人物"]
[depicts.subject="...", depicts.setting="..."]
[caption="..."]
::
```

`data-ref` は ULID / エイリアスのどちらでもよい。

---

## 7. 語彙的制約(D5)

- **key / エイリアス**: `[A-Za-z0-9_-]+`。日本語などの表示名は `label`(§6.1)や
  リンクの表示テキストに書く。エスケープ文法は持たない — 制限のほうが安い
- **座標**: `<行path>|<列path>`。path は `key ("." key)*`。`|` `.` `:` の前後の空白は無視
  (`@cells` の桁揃えのため)

---

## 8. 処理パイプライン

```
[ドラフト SML] (ID未付与・ラベル参照)
      │  strata fmt … ID発行・逆注入(挿入のみ)。エイリアス・参照は温存
      ▼
[管理用 SML] (ULID+alias 併記済み)          ← git 管理の単位
      │  strata build … パース・エイリアス解決・Node/Edge構築・不変条件検証
      ▼
[canonical グラフ] (層2。ULID のみ)
```

### 8.1 fmt の実装方式と契約(D6)

**スパンパッチ方式**: パーサが全ブロックのバイトオフセットを記録し、fmt は
`(offset, テキスト)` のパッチ列を生成して元バイト列に後ろから適用する。
元テキストの再シリアライズは行わない — 触っていないバイトは構造的に無傷。

fmt が行う変更は3種類だけ:

1. 行型ブロックの行末に ` {#ULID}` を**追記**
2. ID の無い段落の直前に `[id=ULID]` 行を**挿入**
3. 非 ULID ラベルの `{#label}` / `[id=label]` を `{#ULID alias=label}` 形式に**置換**
   (`{...}` / `[...]` の内側のみ)

契約(テストで固定する受け入れ条件):

- **冪等性**: `fmt(fmt(x)) == fmt(x)`
- **挿入のみ**: fmt 前後の diff は上記3種の追記・挿入・囲み内置換のみ
- **意味保存**: `build(fmt(draft))` ≅ `build(formatted)`(同型のグラフ)
- **原子性**: 一時ファイルに書いて rename。途中状態をディスクに残さない

### 8.2 エラー方針

**全か無か**: 1箇所でもパースエラーがあれば、fmt はファイルに一切触れず
エラー位置(スパン)を報告して終了する。「半分だけ処理された状態」を作らない。

### 8.3 rowan / CST への乗り換えトリガ

次のいずれかが要件になった時点で再評価する。それまではパッチ方式を維持:

1. fmt に**整形機能**(表の桁揃え・インデント正規化)を持たせたくなった
2. **エディタ統合(LSP)** — 打鍵中の壊れた文書への増分再パースが必要になった
3. オフセット管理のバグが繰り返し出て負債化した

---

## 9. strata-core への波及(必要な拡張)

本仕様を実装するには strata-core に以下の拡張が要る(未実装の設計課題として明示):

1. **`Rel::RefersTo` の追加** — インラインのナビゲーション参照(§5.2)が materialise
   する弱参照。現行カタログには存在しない(現実装は `DependsOn` に誤って畳んでいる)
2. **セル参照の座標保持** — `Inline::Ref` は `{to, rel}` しか持てず、`cell:` 参照の
   座標(§5.3)を落としてしまう。`Inline::Ref` の拡張か専用バリアントが必要
3. **数量(数値+単位)の canonical 表現** — `45 ms` の構造化(D4)を受ける先。
   `CellValue` の拡張(例: `Quantity { v, unit }`)か、`Value` ノード + Ref かは実装時に決定
4. **エイリアス解決表** — build が保持する層1の道具。canonical グラフには入れない

---

## 10. 凍結 vs 保留

**凍結(本書の契約):** D1〜D6 の全決定、§3〜§8 の記法と fmt 契約。

**保留(後で決める):**

- 文書レベルのコメント構文(フェンス内 `#` のみ確定。`<!-- -->` が有力候補)
- member label 構文の最終形(`- key "label"` は初版案)
- `defines` エッジの SML 表現(用語定義ブロックの記法)
- ファイル横断のエイリアス(グローバルエイリアス表)
- リスト項目の中の複数ブロック(項目=段落1つ、を当面の制約とする)
- インラインテキスト中のリテラル `[` `$` 等のエスケープ

---

## 付録A. 文法スケッチ(EBNF 風・実装時に厳密化)

```ebnf
document    = block* ;
block       = attr-line? ( heading | fence | list-item+ | paragraph ) ;

attr-line   = "[" attr ( "," attr )* "]" NL ;
attr        = key "=" attr-value ;              (* id はプローズ用属性行のみ *)

heading     = "#"+ SP inline ( SP id-tag )? NL ;
list-item   = ( "-" | DIGITS "." ) SP inline ( SP id-tag )? NL ;
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
scheme      = "ref" | "term" | "table" | "fig" | "math" | "cell" ;
target      = ULID | alias | term-name ;        (* term-name は term: のみ *)
```

## 付録B. 総合例

ドラフト(人間/AI が書く):

```markdown
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

fmt 後(挿入と `{...}`/`[...]` 内置換のみ。参照は温存):

```markdown
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

build 後(canonical。要点のみ):

- Node: Section×2, Para×1, Table×1(セル値 `12 ms` は数量として構造化)
- Edge: `contains`(見出しネストから)、`supports`(key-finding → eval-table、属性行から)、
  `refers-to`(key-finding → eval-table、インライン cell 参照から)
- グラフ内に alias は存在しない(ULID のみ)
