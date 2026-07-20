# ビュー定義 v1 文法(D30〜D35)

策定: 2026-07-15(実装から起草)。同日、修正2点(糖衣構文・`rename`→`pick` 改名)
付きで批准(D35)。本書はその批准後の姿を正とする。
実装: `crates/strata-view`(パース・適用・`--check`)、`strata view` サブコマンド
(`crates/strata-cli`)。設計決定の出典は `docs/sml-spec.md` §1.6(D30〜D35)。
複数文書入力(`--workspace`)とセレクタの `doc` スコープは M7 WP-W3(D41〜D43、
§3.1.1)で追加。

v1 の品質基準は「読んで分かる」こと — ビュー定義は LLM 提案・人間承認の
レビュー対象になる(M5 の入口)。このドキュメントは実例付きで、実際に
`~/dev/strata-my-resume/sml/views/*.view.yaml` で使われている書き方をそのまま
引用する。

## 1. 全体像

```
SML ファイル ─(strata build)─▶ canonical グラフ ─(strata view --view def.yaml)─▶
    テンプレート消費用の YAML ファイル群
```

複数文書(ワークスペース)入力の場合(M7 WP-W3):

```
strata.toml ─(strata build --workspace)─▶ 統合 canonical グラフ ─
    (strata view --workspace strata.toml --view def.yaml)─▶ テンプレート消費用の YAML ファイル群
```

`file.sml` 位置引数と `--workspace <strata.toml>` は排他(どちらか一方が必須)。
`--view`/`--check`/`--profile`/`-o` は両モードで共通。

ビュー定義は「グラフのどこを見るか」(**セレクタ**)と「見た値をどう整形するか」
(**コンビネータ**)の宣言だけでできている。スクリプトも正規表現も式言語も無い
(D31/D32)。同一グラフ・同一定義は常にバイト同一の出力になる(決定的)。

## 2. トップレベル構造

```yaml
version: 1

profiles: [submit, check] # このビュー定義が持つ profile 名(D34)

manifest: ../manifests/resume-jis.manifest.yaml # --check が読むマニフェスト(任意)

files:
  build_resume/content/profile.yaml: # -o 起点の相対パス
    profiles: [submit] # 省略時はすべての profile で出力
    content: <コンビネータ> # このファイルの中身を作るコンビネータ1つ
```

- `files` の各キーが1つの出力ファイル。`-o <outdir>` と結合される
  (例: `-o sml` なら `sml/build_resume/content/profile.yaml` に書かれる)。
  1つの SML ファイルから複数テンプレート・複数ディレクトリへ出力できる
  (「各スロットは出力先ファイルを宣言できる」)。
- `content:` は1つの**コンビネータ**(§4)。ファイルの中身がオブジェクトなら
  `fields:`、配列なら `rows:` を直接置く。
- `manifest:` はこのビュー定義ファイル自身のディレクトリを基準にした相対パス。
  `--check` でのみ使う(§6)。

## 3. セレクタ(D31)— グラフのどこを見るか

一級セレクタは4種: **alias** / **class** / **セル座標** / **型+contains パス**。
それぞれ D31 の実測(v0 セレクタ頑健度)に基づく。**正規表現は無い** —
表現できない時は SML 側の構造化が正解、というシグナルとして扱う。

セレクタは基本的に「マッピング1つに、種類を表すキー1つだけ」の形で書く
(`{alias: basic-info}` のように)。`self` だけは例外で、bare な文字列として書く。

### 3.1 `alias` — もっとも頑健。まず alias を検討する

```yaml
{ alias: basic-info }
```

`{#ULID alias=basic-info}` のように SML 側で明示的に付けたエイリアス(D26)から
直接ノードを引く。ファイル内で一意。

#### 3.1.1 `doc` — 文書スコープ修飾(M7 WP-W3、D41〜D43)

`strata view --workspace <strata.toml>` で複数文書を1つのグラフに統合したとき、
`alias` に `doc` を添えると特定の文書のブロック alias を明示的に指せる:

```yaml
{ alias: licenses, doc: resume }
```

`doc` の値は対象文書のフロントマター `alias`(D41)。SML 側の doc 修飾参照
`ref:<文書alias>/<ブロックalias>`(D42)と対称の記法・対称の解決規則
(`ワークスペース内で該当文書 alias を持つメンバーが無ければ「文書 alias 不明」、
文書は見つかったがそのブロック alias が無ければ「ブロック alias 不明」を区別して
エラーにする)。

**糖衣構文にも `doc/` 接頭辞を書ける**(§4.1.1 の拡張): `resume/basic-info.氏名`
は `{ pick: { of: { record-field: { of: { alias: basic-info, doc: resume } }, key: 氏名 } } }`
へ脱糖される。`/` は alias 字句(`[A-Za-z0-9_-]+`)に出ないため、`doc/` の分割
(最初の `/`)と `alias.key` の分割(最初の `.`)は互いに干渉しない。

**無指定 doc の解決規則**(裁量): `doc` を省略した無修飾 `alias` は、

- 単一文書モード(`--workspace` 無し): 従来どおりその1文書内で解決(不変)。
- ワークスペースモード: その alias 名が**ワークスペース全体で一意**なら
  そのまま解決する(1つの文書にしか無い alias を毎回 `doc:` 修飾するのは
  冗長なため)。**2つ以上の文書が同じ alias を宣言していれば曖昧エラー**になる
  (D42「無修飾 alias = 同一文書」がブロック build 時点では 1 ファイル内でしか
  一意性を保証しないため、ワークスペース全体では同名 alias の再利用が正当に
  起こりうる — 黙ってどちらかを選ばず、`doc:` での明示を要求する)。

`class`/`heading-text`/`alias-from-row` は本 v0 では doc スコープ化していない
(グラフ全体を対象にした挙動のまま)。ワークスペースでこれらを使う場合、
複数文書にまたがって一致する可能性がある点に注意(裁量、残る摩擦として
§7 に追記)。

### 3.2 `class` — class を持つ唯一のノード

```yaml
{ class: marker }
```

そのクラスを持つノードがグラフ全体でちょうど1つの場合に使う。複数あれば
Warning を出しつつ先頭(NodeId 昇順)を使う。0件ならエラー。

### 3.3 `record-field` — record ノードのキーから値を引く

```yaml
{ record-field: { of: { alias: basic-info }, key: 姓 } }
```

`of` で `::record` ノードを指し、`key` でそのエントリの値(`CellValue`)を取る。
セレクタの中で唯一「ノードではなく値」を返す(コンビネータ側でそのまま
消費できる)。`of` が単純な `{ alias: X }` の場合、`pick` と組み合わせた
定義全体を裸文字列 `X.キー` に短縮できる(§4.1.1 の糖衣構文)。

### 3.4 `cell` — 表のセル座標(D31 のセル座標セレクタ)

```yaml
{ cell: { col: date } } # rows: table のスコープ内(§4.2)。現在の行の date 列
{ cell: { of: { alias: project-index }, col: role } } # 他の表を明示 + 現在の row_path
{ cell: { of: { alias: tech-stack }, col: details, row: [languages] } } # row_path を明示
```

- `col`: 列キー(セルの `col_path` の先頭セグメント)。
- `of`: 対象の表ノードを指すセレクタ。**省略すると、直近の `rows: table`
  (§4.2)が確立した「現在の表」を使う**。
- `row`: row_path をリテラルで明示する。**省略すると「現在の row_path」**
  (`rows: table` の反復、または `rows: contains` の `extend-path` が積んだもの。
  §4.2/4.3)を使う。`rows` のスコープ外から特定の1行だけを直接引きたいとき
  (上の `tech-stack` の例)に `row` を明示する。

### 3.5 `alias-from-row` — 現在の row_path から別ノードへ橋渡しする

```yaml
{ alias-from-row: { prefix: "co-", segment: 0 } }
```

現在の row_path の `segment` 番目のセグメントに `prefix` を連結した文字列を
alias として引く。表の行キー(例: `career-overview` 表の `dentsu`)と、
別のノードの alias(例: `co-dentsu` という会社セクション)を結びつける
アドレス規約(D26 の応用)。

### 3.6 `first-child-of-type` — 型で絞った最初の子

```yaml
{ first-child-of-type: { of: { alias: sec-skills-leverage }, type: list } }
```

`of` の直接の子のうち、指定 type(`section` / `para` / `list` / `table` /
`record` / `code` / `term` / `anchor` / `value` / `document`)に最初に一致する
1つ。「セクションの最初のリスト」のような、型で辿るだけの単純な contains
パスに使う。

### 3.7 `heading-text` — 見出しテキスト一致(Warning 付きエスケープハッチ)

```yaml
{ heading-text: "タイトル" }
```

見出しの平文テキストに一致する Section を探す。**頑健性が低い**(見出しの
言い換えで壊れる)ため、使うたびに Warning を出す。alias を付けられるなら
alias を使うこと(D31)。

### 3.8 `self` — 現在のスコープノード

```yaml
of: self
```

`rows: contains` の `item`(§4.3)の中で、今まさに反復している子ノード自身を
指す。bare な文字列で書く(唯一の例外)。

## 4. コンビネータ(D32)— 値をどう整形するか

固定セットのみ: **pick**・**rows**・**join**・**date**・**age**・
**literal**・**concat**・**class フィルタ**(`join` の `include-only-class`/
`exclude-class` として実装。D58 で `rows: contains` にも同じ語彙・同じ
セマンティクスで追加)。**このセット以外は実装しない** —
足りない時は「コンビネータを1個足す裁定」か「SML 側を直す」の二択(D32)。
`concat` は D35 で一度見送られたが、実需2件(cv 氏名の姓名結合・tech-stack の
details+level 結合)が揃ったため D45(sml-spec.md §1.11)で採用された(§4.9)。
文字列テンプレート式(`"{a}({b})"` のような式言語)は D45 でも不採用のまま
(D32 の「式言語への滑り坂」原則、§7 参照)。

### 4.1 `pick` — 値をそのまま取り出す(旧称 `rename`。D35 で改名)

```yaml
性: { pick: { of: { record-field: { of: { alias: basic-info }, key: 姓 } } } }
```

セレクタが指す値(`record-field`/`cell` の場合)またはノードの本文テキスト
(`alias`/`self` などノードを指す場合、見出し・段落のテキストを平文化する)を
そのまま返す。`as: int` を付けると数値文字列を YAML の整数として出力する
(既定は `as: text`)。**旧名は `rename` だったが、実態は値の抽出でありリネーム
ではないため `pick` に改名した(D35)。旧名は残していない**(pre-1.0)。

```yaml
year: { date: { of: { cell: { col: date } }, format: "YYYY", as: int } }
```

#### 4.1.1 糖衣構文: 裸文字列 `alias.キー`(D35)

`pick`+`record-field` の組は実測でビュー定義の**8割**を占める最頻出パターンの
ため、専用の糖衣構文を持つ。値位置に裸文字列 `alias.キー` を書くと、

```yaml
性: basic-info.姓
```

は次の完全形へ機械的に脱糖される:

```yaml
性: { pick: { of: { record-field: { of: { alias: basic-info }, key: 姓 } } } }
```

- 分割規則: **最初の `.` で** `alias` と `キー` に分ける。alias の字句は
  `[A-Za-z0-9_-]+`(sml-spec.md §7 の key 字句と同じ、ドットを含まない)なので、
  `キー` 側にドットが含まれていても(record のキーは自由テキストで日本語可、
  sml-spec.md D28)誤分割しない。
- 糖衣が作るのは常に `as: text` の `pick`(既定と同じ)。数値化したい場合や、
  `record-field` 以外のセレクタ(`cell`・`self`・`alias-from-row` 等)を対象に
  したい場合は完全形で書く — 糖衣が対応するのは「alias で引いた record から
  1キーを平文抽出する」ケースだけである。
- **`self` との違い**: セレクタの `self`(§3.8)は「値位置」ではなく `of:` の
  中に書くキーワードであり、この糖衣とは文法上の位置が異なるので衝突しない。
  ただし値位置に誤って `self`(ドット無し)を書くと、糖衣の分割規則
  (`.` が必須)に合わず明示的なパースエラーになる — `self` を暗黙に読み替えて
  黙って動く、ということはしない。ノード自身のテキストを取り出したい場合は
  完全形 `{ pick: { of: self } }` を書くこと。
- 糖衣が使えるのはあくまで**コンビネータの値位置**(`fields:` の各エントリの
  右辺、`rows`/`join` の `of:` ではない)。`of:` はセレクタの位置であり、
  `pick` コンビネータそのものが無いため糖衣は展開されない
  (`date`/`age` の `of:` に `alias.キー` は書けない。§3.3 参照)。

### 4.2 `rows` — 表 → 行 dict 配列(表モード)

```yaml
content:
  rows:
    table: { alias: education }
    item:
      fields:
        年: { date: { of: { cell: { col: date } }, format: "YYYY" } }
        月: { date: { of: { cell: { col: date } }, format: "M" } }
        学歴: { pick: { of: { cell: { col: text } } } }
```

`table` が指す表の**葉行**(ネスト次元があれば深さ優先で葉まで辿る。
宣言順=決定的)を1つずつ `item` で評価し、配列にする。`item` の中では
その行の row_path が「現在の row_path」になり、`of` を省略した `cell`
セレクタがそのまま列を引ける(§3.4)。

**class フィルタは非対応**(D58、sml-spec.md §1.17)。`rows: contains`
(§4.3)には `include-only-class`/`exclude-class` があるが、`rows: table`
には無い。表の行キー(`@rows` の member)はグラフ上のノードではなく
class を持たない値なので、class 判定の対象が存在しないため。両者の
併用(`rows.table` + `include-only-class`/`exclude-class`)は
パースエラーになる。

### 4.3 `rows` — ノードの子 → 配列(contains モード)

```yaml
content:
  rows:
    contains: { alias: sec-self-pr }
    type: section
    item:
      fields:
        title: { pick: { of: self } }
        content: { join: { of: self, separator: "\n\n", exclude-class: note } }
```

`contains` が指すノードの直接の子を文書順(`ord` 昇順)で辿り、`type` が
指定されていればその型だけに絞り、`item` で評価する。`item` の中では
`self`(§3.8)がその子ノード自身を指す。

#### class フィルタ(D58)

`join`(§4.4)と同一の語彙・同一のセマンティクスで `include-only-class`/
`exclude-class` を指定できる。判定は D46 の実効 class(自身+祖先の
和集合、`contains` 上流をコンテナ単位で継承)。**フィルタに落ちた子は
その行ごとスキップされる**(行が生成されない。子孫の行だけを間引く
のではなく、その子が生む部分木そのものが反復対象から外れる)。

D23/D46 で確立した submit/check の2 profile 運用(cv-jis.view.yaml
などが実例)に対応する典型例: 職務経歴の案件セクションのうち、
`[class=note]` を付けた「下書きメモ」セクションだけを提出用の行反復から
除外する。

```yaml
projects:
  rows:
    contains: { alias: co-acme }
    type: section
    exclude-class: note
    item:
      fields:
        name: { pick: { of: self } }
        summary: { join: { of: self, separator: "\n" } }
```

SML 側:

```
## Widget案件 {#... alias=proj-widget}
提出する案件の概要。

[class=note]
## 下書きメモ {#... alias=proj-draft}
まだ提出用に整えていない走り書き。
```

`proj-widget` は `projects` 配列に1行として出力されるが、`proj-draft`
は `exclude-class: note` によって行ごとスキップされ配列に現れない
(note セクションの子に段落があってもまとめて消える — `join` の
「コンテナに1回書けばよい」と同じ D46 の継承)。`include-only-class`
を使えば逆に note セクションだけを抽出できる(§10 の note/レビュー用途
向け view を書く場合など)。

`rows: table`(§4.2)にはこのフィルタは無い(D58: 行キーは class を
持たないため対象外)。

#### `extend-path` — なぜ必要か、何をするか

**動機**: SML の文書構造(見出しのネスト = `contains`)と、表の行次元(会社×
プロジェクトのようなネスト `@rows`)は**別々の木**である。職務経歴書は
「会社セクションの下に案件セクションが並ぶ」という文書構造と、「会社×案件で
期間・役割・技術を格子状に持つ」という表(`project-index`)の**両方**を必要と
する — 前者は読み物としての構造(概要・見出し)、後者は各セルに複数の属性
(期間・役割・使用技術)を持たせるための構造化データで、文書のネストだけでは
表現しづらい。両者は「同じ会社・同じ案件」を指しているはずだが、SML の中では
別々の木として存在するため、`rows: contains` で文書構造を辿るだけでは
`project-index` 表の**行 (row_path) を再現できない**。`extend-path` はこの
2つの木を**alias を接着剤にして**橋渡しする機構である。

**動作**: `rows: contains` が今訪れている子ノード自身の `alias` から、
`extend-path.alias-suffix.prefix` を取り除いた残り(接尾辞)を、現在の
`row_path` に1要素として**追加**する。

```yaml
projects:
  rows:
    contains: { alias-from-row: { prefix: "co-", segment: 0 } }
    type: section
    extend-path: { alias-suffix: { prefix: "proj-" } }
    item:
      fields:
        name: { pick: { of: self } }
        period:
          date:
            of: { cell: { of: { alias: project-index }, col: period } }
            format: "YYYY-MM"
            period-separator: " ~ "
            period-open: "現在"
```

**接頭辞剥がしの動作例**: 子セクションの alias が `proj-isid_azure_ml_prod`
で、`prefix: "proj-"` なら、`alias-suffix` は先頭の `"proj-"` を切り落として
`"isid_azure_ml_prod"` を row_path に積む。子ノードに alias が無い、または
`prefix` で始まっていない場合はエラーになる(黙って読み飛ばさない — alias
規約が破られていることの検出でもある)。

**cv-jis.view.yaml の3方向 join(図解)**: `build_cv/content/companies.yaml`
は3つの独立した構造を row_path という1本の座標系の上でつなぎ合わせる。

```
① rows: table                  ② rows: contains                ③ cell (extend-path 後)
   career-overview 表              文書構造(見出しネスト)             project-index 表
   (会社の一覧)                                                       (会社×案件のセル)

   row: acme            ──alias-from-row("co-",0)──▶  # 会社 {alias=co-acme}
   row_path=[acme]                                        │
                                                            ├─ ## Widget案件 {alias=proj-widget}
                                                            │     │
                                                            │     └─ extend-path: alias "proj-widget"
                                                            │        から "proj-" を剥がして "widget"
                                                            │        row_path に追加
                                                            │        → row_path=[acme, widget]
                                                            │                              │
                                                            │                              ▼
                                                            │        project-index 表の row_path=[acme, widget]
                                                            │        のセル(period/role/tech)を cell セレクタで引く
                                                            └─ ## 他の案件 {alias=proj-...} (同様に繰り返す)
```

流れを言葉で言うと: ①外側の `rows: table`(career-overview)が
`row_path = [company_key]`(`acme`)を確立する → ②内側の `rows: contains`
(alias-from-row で `co-acme` という文書構造上の会社セクションへジャンプし、
その子の案件セクション列を文書順に辿る)が各案件セクション自身の
`name`/`summary` を `self` で取り出しつつ、`extend-path` でセクション自身の
alias(`proj-widget`)から接頭辞を剥がした `widget` を row_path に継ぎ足して
`row_path = [acme, widget]` にする → ③この拡張された row_path を使い、
`item` 内の `cell` セレクタが**別の表**(`project-index`、会社×案件のネスト
行次元を持つ)の同じ座標のセルを引く。これにより、「文書としての読みやすい
案件セクション(概要文・箇条書き)」と「構造化された属性(期間・役割・
技術)」を、行を二重に書き下すことなく1つの出力行にまとめられる(v0 の
Python バインディングが素朴な文字列突合で行っていた3方向 join を、alias
接尾辞という決定的な規約で代替する。§7 の「残摩擦」も参照)。

### 4.4 `join` — 子ノード列・record エントリ列 → 文字列連結

**木モード**(既定): 子ノードを歩いて連結する。

```yaml
summary:
  join:
    of: self
    separator: "\n"
    nested-prefix: "・"
    exclude-class: note
```

`of` が指すノードの直接の子を歩き、`exclude-class`/`include-only-class`
(class フィルタ)で絞りながら:

- リスト以外の子(段落など)は、その本文テキストをそのまま1行にする。
- リスト型の子は、その各項目を1行にし、さらに項目がネストしたリストを
  持てば、その孫項目を `nested-prefix` を前置して1行ずつ追加する
  (「トップ項目1つ + ネストされた内訳」という職務経歴の記法パターンに
  対応)。

`separator` で行を連結する。

**record モード**(`keys` を指定): `of` が record ノードのとき、指定した
キーだけを宣言順に `"キー: 値"` 形式の行にし(値が空文字列のキーは省略)、
`separator` で連結する。

```yaml
志望動機:
  join:
    of: { alias: other-info }
    separator: "\n\n"
    keys: [健康状態, 趣味, 特技・スポーツ]
```

### 4.5 `date` — Date/Period → 書式文字列

```yaml
生年月日:
  date:
    of: { record-field: { of: { alias: basic-info }, key: 生年月日 } }
    format: "YYYY年M月D日"
```

`format` はトークン置換のテンプレート: `YYYY`(4桁年) `YY`(下2桁)
`M`(月、0埋めなし) `MM`(月、0埋め) `D`(日、0埋めなし、値が無ければ何も
出さない) `DD`(日、0埋め)。それ以外の文字はリテラルとして通過する
(`"YYYY年M月"` `"YY/M"` など、v0 が実際に出力していた書式をすべて賄う)。

`of` が指す値が `Period`(期間)なら、`period-separator`(from と to の間)と
`period-open`(to が無い=継続中のときの表示)が必要:

```yaml
period:
  date:
    of: { cell: { col: period } }
    format: "YYYY年M月"
    period-separator: "～"
    period-open: "現在"
```

`of` が `Text`(型付きパースに失敗しテキストへフォールバックした値。例:
複数区間 `"2020-09 ~ 2021-03 / 2021-09 ~ ..."`)ならそのまま通す。

`as: int` を付けると結果を整数として出力する(既定 `as: text`)。

### 4.6 `age` — 生年月日+基準日 → 満年齢

```yaml
満年齢:
  age:
    birth: { record-field: { of: { alias: basic-info }, key: 生年月日 } }
    as-of: { record-field: { of: { alias: basic-info }, key: 作成日 } }
    as: text
```

`birth`/`as-of` はどちらも Date(または Period の場合は `from`)を指す
セレクタ。満年齢の計算式(誕生日未到来なら1引く)。**as-of はハードコード
せず、必ずグラフ側(record の「作成日」等)から引く**(既知の注意点)。

### 4.7 `literal` — 固定値

```yaml
environments: { literal: "Linux, Windows, macOS" }
tech_skills: { literal: [] }
Email2: { literal: null }
```

グラフに対応するデータが無いスロット(v0 の環境フォールバック値・案内文言
など)や、常に空であるべきスロット(第2連絡先など)を**明示的に**宣言する。
「バインディングはデータを持たない」原則に反して残っていた v0 の暗黙の
デフォルトを、ここで宣言として可視化する(sml-spec.md D30〜D34 節末尾の
既知の注意点)。

### 4.8 `fields` — 名前付きサブコンビネータの集まり

```yaml
fields:
  性: basic-info.姓 # 糖衣(§4.1.1)。完全形なら { pick: { ... } }
  名: basic-info.名
```

`rows` の `item` や、ファイルの `content` 直下で使う「まとめ役」。**YAML の
宣言順がそのまま出力の列順になる**(決定性・可読性のため、辞書順には
並び替えない)。

### 4.9 `concat` — 複数コンビネータの文字列連結(D45)

```yaml
name:
  concat:
    parts:
      - resume/basic-info.姓
      - resume/basic-info.名
    separator: " "
```

`parts` に列挙した**任意のコンビネータ**(pick/date/literal/concat 自身の
入れ子も可、`alias.キー` 糖衣も可)を順に評価し、それぞれの結果を文字列化して
`separator` で連結する。`separator` は省略すると空文字列(D45)。

`literal` と組み合わせて、値と値の間に固定の記号(改行・括弧等)を挟める:

```yaml
languages:
  concat:
    parts:
      - tech-stack.details # 「Python, R, SQL, ...」
      - literal: "\n("
      - tech-stack.level # 「Python/R: 5年（データ分析・...）...」
      - literal: ")"
    separator: ""
```

`cv-jis.view.yaml`(`~/dev/strata-my-resume/sml/views/`)の実例2件:

- **氏名の姓名結合**(`build_cv/content/profile.yaml.fields.name`): M7 で
  resume.sml の `::record {alias=basic-info}` に足していた冗長な `氏名` フィールド
  (姓+名の合成テキストを SML 側に複製していた)を撤去し、`concat` で
  `resume/basic-info.姓` + `" "`(半角スペース、既存表示と同値になるよう選んだ
  区切り)+ `resume/basic-info.名` へ置き換えた。
- **tech-stack の details(level) 復元**(`experience.languages`/`frameworks`/
  `infrastructure`): v0 が `"{details}\n({level})"` で連結していたテンプレート
  文字列を、`concat`(details の pick + `literal: "\n("` + level の pick +
  `literal: ")"`)で宣言的に再現した。

`separator` は parts 全体を貫く一様な区切りのみ(「1番目と2番目の間だけ別の
区切り」のような per-pair 指定は無い)。そこまで凝った連結が要る場合は
`literal` を parts に挟み込む(上記の details/level 例)か、「SML 側の構造化を
見直す」方を検討する(D32 の原則どおり)。

## 5. profile(D34)

```yaml
profiles: [submit, check]

files:
  build_cv/content/companies.yaml:
    profiles: [submit]
    content: { ... }
  build_cv/content/companies_check.yaml:
    profiles: [check]
    content: { ... }
```

`files.<path>.profiles` を省略したファイルは常に出力される。`strata view
--profile <name>` を指定すると、その profile を含まないファイルは書かれない。
**`--profile` を省略すると、宣言されている全 profile のファイルが出力される**
— これは実際のテンプレートビルドが submit/check 両方の content YAML を
同時に必要とする現実(main.typ と main_check.typ が別ファイルを読む)に
合わせた既定値(裁量)。「これは絶対に提出版に含めない」と確信したい時だけ
`--profile submit` を明示する。

## 6. マニフェストと `--check`(D33)

マニフェストはテンプレートが実際に読むスロット(ファイル+フィールド)の
宣言。ビュー定義から `manifest:` で参照する。

```yaml
version: 1
files:
  build_resume/content/profile.yaml:
    shape: fields # fields(ファイル全体が1つの dict) or rows(配列)
    fields: [性, 名, 性読み, 名読み, 生年月日, 満年齢, 写真]
  build_resume/content/education_history.yaml:
    shape: rows
    fields: [年, 月, 学歴] # 各行(dict)の必須キー
  build_cv/content/skills_leverage.yaml:
    shape: rows
    fields: [] # 行がスカラ(文字列)そのものなのでサブフィールド無し
notes:
  - "テンプレ内データハードコードの注記など、自由記述。"
```

`strata view <file.sml> --view <def.yaml> --check` は2種類の診断を出す:

- **未充足スロット**(`MissingSlot`): マニフェストが要求するファイル/
  フィールドが、ビュー定義側の `fields` に無い。
- **未使用ノード**(`UnusedNode`): グラフのノードのうち、alias が付いた
  **Table/Record/List**(構造化データ)でありながら、どのセレクタからも
  一度も参照されなかったもの(粗い定義。裁量: 見出し・地の文の alias は
  「案内・narrative」であることが多く対象から外している。§7 参照)。

診断はどちらも fmt/build と同じ `行:列: 種別: メッセージ` 流儀([-:-]
はソース上の位置を持たない診断であることを示す)で stderr に出る。1件でも
あれば exit 1、無ければ exit 0。

**ワークスペースモードの未使用ノード判定(M7 WP-W4)**: `strata view --workspace
--check` は「未使用ノード」の走査対象を、**このビュー定義が実際に触れた文書**に
絞る(裁量)。単一の統合グラフをそのままフィルタ無しで走査すると、1つのビュー
定義が意図的に対象としていない他文書の alias 付きノードが常に「未使用」として
鳴ってしまうため。さらに、`{ alias: X, doc: Y }` で**明示的に**借りてきたノードは
「文書 Y 全体がこのビューの対象になった」根拠にしない — 1フィールドだけ
`doc:` で他文書から引いた場合(例: cv-jis.view.yaml が `doc: resume` で
氏名・免許資格だけを借りる)、その他文書側の無関係なノードまで未使用判定に
巻き込まれない。「その文書を無修飾 alias で(=実質的な home document として)
使っている」場合だけ、その文書全体を走査対象にする。

## 7. 既知の限界(v1 スコープ外・拡張裁定候補)

- ~~**クロスドキュメント合成不可**~~(→ M7 WP-W3 で解消。`strata view --workspace
  <strata.toml>` + セレクタの `doc` スコープ(§3.1.1)で複数 SML ファイルに
  またがる合成ができる — 履歴書の氏名を職務経歴書側のビュー定義から直接引く、
  というまさにこのユースケースが受け入れ基準だった)。ただし `class`/
  `heading-text`/`alias-from-row` は doc スコープ化していない(グラフ全体対象の
  まま、§3.1.1 末尾)。~~また `context`/`render` の横断は v0.5 スコープだった~~
  (→ M7.5 WP-Z1 で解消。sml-spec.md §1.11 D44: `render --workspace [--doc]`・
  `context --workspace [--doc]` を追加。MD のページ間リンクは相対 `.md` リンク+
  アンカー、Typst は文書名付き退化テキスト。詳細は docs/workspace-m75-handoff.md)。
- ~~**多フィールドの自由な連結(テンプレート文字列)は書けない**~~(→ D45 で
  `concat` コンビネータを採用して解消。§4.9 参照。実需2件(cv 氏名の姓名結合・
  tech-stack の details+level 結合)が揃ったため、D32 の「コンビネータの追加は
  裁定を経る」運用に従って再裁定した — D35 での一度目の見送りからの方針転換)。
  なお**文字列テンプレート式**(`"{a}({b})"` のような式言語)は D45 でも不採用の
  まま: `concat` は「コンビネータの列を順に連結するだけ」で、式の評価・条件分岐は
  持たない(D32「式言語への滑り坂」原則の一貫)。

## 8. 実例

完全な実例は次の2ファイル(`~/dev/strata-my-resume/sml/views/`)を参照:

- `resume-jis.view.yaml` — JIS 履歴書(1 profile: submit)。record/表からの
  素朴な pick(大半は `alias.キー` 糖衣)・date・age・literal の使用例。
- `cv-jis.view.yaml` — 職務経歴書(2 profile: submit/check)。`rows: contains`
  + `extend-path` による3方向 join、`join` の木モード/record モード両方、
  profile 分岐の実例。

対応するマニフェストは `~/dev/strata-my-resume/sml/manifests/` 配下。
