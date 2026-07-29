---
id: 01KYP2T290HENC27W868J8MNKA
title: "Strata — 意味グラフ文書フォーマット 仕様(叩き台 v0.1)"
alias: strata-spec
---

# Strata — 意味グラフ文書フォーマット 仕様(叩き台 v0.1) {#01KYP2T291A47GHVAJE9HW68YT}

[id=01KYP2T291A47GHVAJE9HW68YV]
> [id=01KYP2T291A47GHVAJE9HW68YW]
> **Strata は仮の名前です。** 自由に変えてください。「層(strata)」から取った置き字です。

[id=01KYP2T291A47GHVAJE9HW68YX]
この仕様は、人志向ドキュメントと機械志向ドキュメントを **1つの源(canonical)** から派生させるための、新しい文書表現を定義する。既存フォーマット(Markdown / HTML / その他)の上に乗せるのではなく、独立した表現として設計する。

---

## モチベーション(なぜ作るのか) {#01KYP2T291A47GHVAJE9HW68YY}

### 問題 {#01KYP2T291A47GHVAJE9HW68YZ}

[id=01KYP2T291A47GHVAJE9HW68Z0]
人が読むためのドキュメントと、機械が読むためのドキュメントの、両方が必要な時代になった。だが今この2つは分断している。人向けは可読性に最適化され構造を失い、機械向けは構造を持つが人には読めない。同じ内容を二重に持つか、どちらかを諦めるか、になっている。

[id=01KYP2T291A47GHVAJE9HW68Z1]
Markdown は人志向の代表だ。可読性に重きを置き、難解さを制限したのが美点。だがそれは表現力の放棄でもあった。文書の複雑さ — 数式、多段に束ねた表、相互参照、図 — を Markdown は持ちきれない。

### Markdown が「制約が多すぎる」ことの正体 {#01KYP2T291A47GHVAJE9HW68Z2}

[id=01KYP2T291A47GHVAJE9HW68Z3]
文書の複雑さは1つの量ではなく、独立した6つの軸に分解できる(各軸の階級づけは §0・各節で詳述)。

[id=01KYP2T291A47GHVAJE9HW68Z4]
1. **参照構造** — 線形 / 木 / DAG / グラフ {#01KYP2T291A47GHVAJE9HW68Z5}
2. **局所表記** — 数式や入れ子の表のような再帰的な木 {#01KYP2T291A47GHVAJE9HW68Z6}
3. **レイアウト束ね** — 結合セルが「概念の束ね」なのか「見栄え」なのか {#01KYP2T291A47GHVAJE9HW68Z7}
4. **文脈依存** — 「下の表」「前述」のような位置依存の指示語 {#01KYP2T291A47GHVAJE9HW68Z8}
5. **連続実体** — 画像(記号図 / 写真) {#01KYP2T291A47GHVAJE9HW68Z9}
6. **物理レイアウト** — 改ページ・段組・列幅 {#01KYP2T291A47GHVAJE9HW68ZA}

[id=01KYP2T291A47GHVAJE9HW68ZB]
Markdown を6軸で診断すると: 参照を木に制限し(①)、行内に木を埋められず(②)、束ねの意図を持てず(③)、物理を本文に混入させ(⑥)、画像は貧弱な外付けしか持てない(⑤)。**6軸のうち5軸で力不足**で、唯一の取り柄である「人が読める表面」さえ、物理レイアウトの混入で汚れている。これが「制約が多すぎる」の定量的な正体。

### 鍵となった気づき: 二項対立ではなく三者 {#01KYP2T291A47GHVAJE9HW68ZC}

[id=01KYP2T291A47GHVAJE9HW68ZD]
当初は「人志向 vs 機械志向」「人向けは Markdown、機械向けは隠蔽された HTML か?」という二項で考えていた。だが詰めると、**実体は2つではなく3つ**だった。

[id=01KYP2T291A47GHVAJE9HW68ZE]
- **オーサリング** — 人が書くための形(書きやすさに全振り) {#01KYP2T291A47GHVAJE9HW68ZF}
- **canonical** — 機械が読み、かつロスレスな形(真実の源。人の書きやすさ・読みやすさには媚びない) {#01KYP2T291A47GHVAJE9HW68ZG}
- **render** — 人が読む / 機械が食う形(組版・指示語・結合セルはここで初めて生成) {#01KYP2T291A47GHVAJE9HW68ZH}

[id=01KYP2T291A47GHVAJE9HW68ZJ]
Markdown の敗因は、この3役を1つのファイルで兼ねたこと。だから物理レイアウトが本文に混入し、構造が中途半端になった。「隠蔽された HTML」という当初の直感は、この **canonical と render の分離** を指していた — 方向は正しく、ただ HTML という既存形式に当てはめようとしたから収まりが悪かった(HTML は render 寄りで、軸3・軸4 で力不足)。

### だから新しい表現を作る {#01KYP2T291A47GHVAJE9HW68ZK}

[id=01KYP2T291A47GHVAJE9HW68ZM]
既存形式はどれも特定の層にコミットしている。Markdown はオーサリング寄り、HTML/CSS は render 寄り。**6軸すべてでロスレスで、かつ物理レイアウトを構造的に排除した clean な canonical** は、既存のどれにも存在しない。特に軸3(結合の意味づけ)を素で持てる形式はほぼ皆無で、ここは業界全体の空白地帯。

[id=01KYP2T291A47GHVAJE9HW68ZN]
よって、既存の枠に入れるのではなく、層2(canonical)を独立した表現として新規に設計する。設計の核心は2つ:

[id=01KYP2T291A47GHVAJE9HW68ZP]
- 表を「格子+colspan」ではなく **次元の木** で持つ。これで軸3と軸6が同時に解ける(結合プリミティブが消え、物理が入る隙間も消える)。 {#01KYP2T291A47GHVAJE9HW68ZQ}
- payload の型に **物理レイアウト語を一切持たせない**。漏洩が構文的に不可能になる。 {#01KYP2T291A47GHVAJE9HW68ZR}

### 成功の姿 {#01KYP2T291A47GHVAJE9HW68ZS}

[id=01KYP2T291A47GHVAJE9HW68ZT]
1つの canonical から:

[id=01KYP2T291A47GHVAJE9HW68ZV]
- 人は、任意の媒体で読める — 紙・画面・**音声・点字**。物理レイアウトは render 時に生成されるので、ページの概念が消える音声出力でも壊れない。 {#01KYP2T291A47GHVAJE9HW68ZW}
- 機械は、構造をロスレスに読める — 数式は木、表は階層インデックス、参照は型付きエッジ。 {#01KYP2T291A47GHVAJE9HW68ZX}
- 内容は単一ソース。定義を1箇所直せば、それを参照する全ドキュメントに伝播する。 {#01KYP2T291A47GHVAJE9HW68ZY}

[id=01KYP2T291A47GHVAJE9HW68ZZ]
人の世界の複雑なドキュメントを機械が読め、適切な parser を通せば人が読み戻せる — 当初の要件が、層の分離と次元の木によって満たされる。

### 非目標 {#01KYP2T291A47GHVAJE9HW6900}

[id=01KYP2T291A47GHVAJE9HW6901]
- canonical を人が直接手書きすることは想定しない(層1=オーサリングの仕事)。canonical は機械可読性とロスレス性に全振りし、人間工学は層1・層3 に委ねる。 {#01KYP2T291A47GHVAJE9HW6902}
- 組版エンジンそのものは作らない。物理レイアウトは render(層3)の関数であり、Typst 等の既存資産に委ねられる。 {#01KYP2T291A47GHVAJE9HW6903}

### 新規性(何が新しいのか) {#01KYP2T291A47GHVAJE9HW6904}

[id=01KYP2T291A47GHVAJE9HW6905]
正直に言えば、Strata の構成要素はほぼすべて先行例がある(詳細は §14)。3層分離はシングルソース出版(DITA/DocBook)、トランスクルージョンと単一ソースは Xanadu、文書をグラフで持つ発想はデジタル人文学の Text-as-Graph、機械可読な構造グラフは近年の GraphRAG。MyST や PreTeXt は「オーサリング → AST → 多出力」を実装済みで、PreTeXt は単一ソースから紙・EPUB・点字・触図まで現実に出している。

[id=01KYP2T291A47GHVAJE9HW6906]
ではどこが新しいか。Strata の貢献は「発明」ではなく **統合と、数手の鋭い定式化** に尽きる。

[id=01KYP2T291A47GHVAJE9HW6907]
1. **没交渉な4系統の統合**。シングルソース出版(技術文書)、ハイパーテキスト(Xanadu)、デジタル人文学(OHCO/TAG)、AI文書処理(GraphRAG)は普段ほとんど引用し合わない。これらを「個人ナレッジ + AIワークフロー」という現代の文脈で1つの canonical 設計に束ねた例は見当たらない。 {#01KYP2T291A47GHVAJE9HW6908}

2. **木ではなくグラフを canonical の背骨に据える**。既存の実用フォーマット(Markdown / MyST / PreTeXt / DocBook)はすべて OHCO=単一階層の木に相互参照を上乗せする方式で、1996年に反証された「テキスト=木」の前提に留まっている。Strata は反証後の Text-as-Graph を、学術的興味ではなく**実用の知識基盤**として採用する。これにより、木では原理的に持てない4つ — グラフ背骨・意味を持つ型付きエッジ・次元木の表・式のMathML木 — を canonical に持つ(§14H に精密な差分)。 {#01KYP2T291A47GHVAJE9HW6909}

3. **表を次元の木で持って軸3を「消す」**。MultiIndex/OLAP はデータ分析では常識だが、それを文書フォーマットの canonical な表モデルに据え、「結合セルは概念か装飾か」という長年の難問(W3C TAG も未解決と認める)を**問いごと消滅させる**定式化は、先行文献に明示的に見当たらない。アクセシビリティ研究の「音声・点字では次元情報が無いとセルの値が無意味」が、この設計の必要性を裏側から実証している。 {#01KYP2T291A47GHVAJE9HW690A}

4. **物理レイアウトを型システムから構文的に排除する**。コンテンツ/プレゼンテーション分離の原則自体は古い。新しいのは、それを「payload の型に物理語を持たせない=漏洩が構文的に不可能」という**型不変条件として強制する**点。原則を運用規約ではなく型で守る。 {#01KYP2T291A47GHVAJE9HW690B}

5. **グラフが事後抽出ではなくネイティブ**。GraphRAG は完成した文書から機械のためにグラフを**後から復元**する(ロスあり)。Strata はオーサリング時点からグラフが源(ロスレス)。「機械のために後で構造を起こす」時代から「最初から構造で書く」への反転。 {#01KYP2T291A47GHVAJE9HW690C}

[id=01KYP2T291A47GHVAJE9HW690D]
要するに新規性は **「反証済みの木前提を捨て、4系統を1つの実用 canonical に統合し、表と物理レイアウトに2手の鋭い定式化を加えたこと」**。逆に言えば、独立した複数分野が同じ結論(木では足りない/単一ソース/埋め込むな/源で構造を持て)に達しているのは、この設計が地に足のついている強い証拠でもある。

---

## 0. 設計思想(なぜこの形か) {#01KYP2T291A47GHVAJE9HW690E}

[id=01KYP2T291A47GHVAJE9HW690F]
文書の実体は **3層** に分かれる。1ファイルで3役を兼ねたのが Markdown の敗因だった。

``` {#01KYP2T291A47GHVAJE9HW690G}
  [1 オーサリング]  人が書く        → 書きやすさに全振り
        ↓ 変換
  [2 canonical]    機械が読む+ロスレス → 真実の源。両者に媚びない
        ↓ render(view, style, binding)
  [3 render]       人が読む / 機械が食う → 読みやすさ・機械可読に全振り
```

[id=01KYP2T291A47GHVAJE9HW690H]
Strata が定義するのは **層2(canonical)** である。層1・層3 は層2からの射影/逆射影であり、差し替え可能。設計リスクはすべて層2に集中する。

[id=01KYP2T291A47GHVAJE9HW690J]
文書の複雑さを 6 軸に分解し、各軸を canonical のどこで受けるかを決めてある。

[id=01KYP2T291A47GHVAJE9HW690K]
| 軸 | 内容 | Strataでの受け方 |
|---|---|---|
| 1 参照構造 | 線形→木→DAG→グラフ | 型付きエッジ(グラフ, L3) |
| 2 局所表記 | 数式・入れ子の再帰木 | `math`ノードの再帰木 payload (L3) |
| 3 レイアウト束ね | 結合セルは概念か装飾か | **次元の木**で持つ。結合プリミティブを廃止 (L2) |
| 4 文脈依存 | 「下の表」等の指示語 | ID参照のみ。指示語は禁止 (L3=解決済) |
| 5 連続実体 | 画像 | 記号図=データ宣言 / 写真=外部参照+構造化記述 |
| 6 物理レイアウト | 改ページ・段組・列幅 | **payload型に物理語を持たない**=構文的に書けない |

---

## 1. 不変条件(契約 / 破ってはいけない規則) {#01KYP2T291A47GHVAJE9HW690M}

[id=01KYP2T291A47GHVAJE9HW690N]
実装が何であれ、以下は常に保つ。これが Strata の同一性。

[id=01KYP2T291A47GHVAJE9HW690P]
1. **ID 安定**: ノードIDは ULID。不変・再利用しない。内容を編集してもIDは変わらない。 {#01KYP2T291A47GHVAJE9HW690Q}
2. **物理レイアウト不在**: いかなる payload も、ページ・段組・列幅・「見栄えのための改行」を表現しない。文法に該当トークンが存在しない。 {#01KYP2T291A47GHVAJE9HW690R}
3. **指示語禁止**: 本文に「下の表」「前述」「p.42」を書いてはならない。相互参照は ID への型付き参照のみ。表示文言(「左の表3」等)は render 時に生成される派生物。 {#01KYP2T291A47GHVAJE9HW690S}
4. **記号を焼かない**: グラフ・数式など記号で表せるものを画素(PNG等)に焼いてはならない。データ+宣言で持つ。 {#01KYP2T291A47GHVAJE9HW690T}
5. **単一ソース**: 同一ノードは複数の親を持てる(トランスクルージョン)。内容の複製を作らない。 {#01KYP2T291A47GHVAJE9HW690V}
6. **contains は DAG**: `contains` エッジは閉路を作らない(複数親は可)。それ以外の rel は一般グラフでよい。 {#01KYP2T291A47GHVAJE9HW690W}

### 1.1 強制手段の区別(正直な整理) {#01KYP2T291A47GHVAJE9HW690X}

[id=01KYP2T291A47GHVAJE9HW690Y]
6つの不変条件は等しく「契約」だが、**守らせる仕組みの強さは条件ごとに異なる**。「型で強制」と言えるのは実際には 2 と 4 の一部だけであり、残りは実行時検証・lint・運用規約の仕事である。ここを混同すると「型で守られているから安全」という誤った安心が生まれるので、明示的に区別する。

[id=01KYP2T291A47GHVAJE9HW690Z]
| # | 不変条件 | 強制手段 | 現状 |
|---|---|---|---|
| 1 | ID 安定 | **運用規約**。型は ULID であることまでは強制するが、「再利用しない」「編集で変えない」はフォーマッタ/コンパイラの実装規律でしか守れない | `strata fmt` 未実装 |
| 2 | 物理レイアウト不在 | **型で強制**。payload に物理語のバリアントが存在しないため、漏洩は構文的に不可能 | 達成済(strata-core) |
| 3 | 指示語禁止 | **型では守れない**。`Inline::Text` は自由文字列なので「下の表」と書くこと自体は構文的に可能。禁止フレーズ検出などの **lint** が必要 | lint 未実装 |
| 4 | 記号を焼かない | **一部型で強制**。`Chart` は `data_ref` 必須でデータの埋め込み(焼き込み)を型が許さない。ただし記号図を最初から画素(`Image`)として持ち込むことは型では防げず、lint/運用の領域 | Chart 側は達成済 / Image 側は未検討 |
| 5 | 単一ソース | **型は可能にするだけ**。複数親 contains を型が許す(=トランスクルージョンできる)が、「複製を作らない」こと自体は強制できない。重複・コピペ検出はツールの仕事 | 未検討 |
| 6 | contains は DAG | **実行時検証**。`invariants::validate` の閉路検出で担保 | 達成済(strata-core) |

[id=01KYP2T291A47GHVAJE9HW6910]
したがって「物理レイアウトを型システムから構文的に排除する」(モチベーション節・新規性4)という主張は不変条件 2 に限った話であり、6条件すべてが型で守られているわけではない。lint 層(3・4後半・5)は未実装の設計課題として残っている。

---

## 2. コアデータモデル {#01KYP2T291A47GHVAJE9HW6911}

[id=01KYP2T291A47GHVAJE9HW6912]
canonical は **2種類のレコード** だけで成る。ファイルは存在しない。あるのはノードの集合とエッジの集合。

### 2.1 Node {#01KYP2T291A47GHVAJE9HW6913}

``` {#01KYP2T291A47GHVAJE9HW6914}
Node {
  id:      ULID                 // 一級市民。安定・不変
  type:    NodeType             // §3 参照
  payload: object               // type 固有。物理レイアウト語を含まない
}
```

### 2.2 Edge {#01KYP2T291A47GHVAJE9HW6915}

``` {#01KYP2T291A47GHVAJE9HW6916}
Edge {
  from: NodeId
  to:   NodeId
  rel:  EdgeRel                  // §4 参照
  ord?: number                  // contains の子順序づけ用
}
```

[id=01KYP2T291A47GHVAJE9HW6917]
`contains` が木の背骨を作り、それ以外のエッジが木を横断してグラフ化する。

### 2.3 設計判断: インラインは「ノード」ではなく「payload」 {#01KYP2T291A47GHVAJE9HW6918}

[id=01KYP2T291A47GHVAJE9HW6919]
段落の中の太字や数式・参照を全部ノードにすると node-soup になる。よって:

[id=01KYP2T291A47GHVAJE9HW691A]
- **ブロックレベル**(section/para/table/figure/…)= グラフ Node(ID付き) {#01KYP2T291A47GHVAJE9HW691B}
- **インライン内容** = ブロックの payload 内の **インライン AST**(後述) {#01KYP2T291A47GHVAJE9HW691C}
- 相互参照(ref/term-ref/cites)は、インラインAST に標的IDを直接持たせ、**同時に Edge を materialise** する(グラフ照会用の非正規化。内容から導出可能に保つ) {#01KYP2T291A47GHVAJE9HW691D}

[id=01KYP2T291A47GHVAJE9HW691E]
つまり「読むための内容」はインラインAST、「照会するためのグラフ」はエッジ。両者は常に整合させる。

### 2.4 粒度: レベル1 既定 + 需要駆動の昇格(レベル2) {#01KYP2T291A47GHVAJE9HW691F}

[id=01KYP2T291A47GHVAJE9HW691G]
粒度には3段階ある。

[id=01KYP2T291A47GHVAJE9HW691H]
- **レベル0**: ノート1枚 = 1ノード(Obsidian) {#01KYP2T291A47GHVAJE9HW691J}
- **レベル1**: 段落/ブロック = 1ノード、インライン = payload(Roam/LogSeq 相当。Strata の既定) {#01KYP2T291A47GHVAJE9HW691K}
- **レベル2**: インラインの全スパン = ノード(超細粒度。普段使いには重すぎる) {#01KYP2T291A47GHVAJE9HW691M}

[id=01KYP2T291A47GHVAJE9HW691N]
Strata は **レベル1を既定**とする。全スパンをノード化(レベル2)すると、500段落で5000〜15000オブジェクトに膨れ、かつ「文の途中に入力するとテキストノードを分割・再採番」という編集地獄になる。段落は人が一息に書く自然な編集単位なので、その中身は payload の AST を1個書き換えるだけで済むレベル1が素直。

[id=01KYP2T291A47GHVAJE9HW691P]
ただし **identity(指される必要)が生じたスパンだけ、その時にノードへ昇格する**(需要駆動)。これはレベル2の利益(句レベルのリンク/トランスクルージョン/注釈/履歴)を、コストを全段落に払わずに必要箇所だけ得るための機構。

[id=01KYP2T291A47GHVAJE9HW691Q]
**昇格ルール:**

[id=01KYP2T291A47GHVAJE9HW691R]
1. 既定では、強調や地の文は `emph`/`text` として para の payload に留まる(レベル1)。視覚的な強調はノードを要しない。 {#01KYP2T291A47GHVAJE9HW691S}
2. あるスパンが**指される必要**(他ブロックからの `supports`/`term-ref`/`cites`、または複数ドキュメントへのトランスクルージョン)を持った瞬間、そのスパンを `anchor` ノードへ昇格する。 {#01KYP2T291A47GHVAJE9HW691T}
  - スパンの内容(InlineAST)を `anchor` ノードへ移し、para の payload では `{ t: "anchor", to: <anchorId> }` で位置を示す。 {#01KYP2T291A47GHVAJE9HW691V}
  - `Edge(para → anchor, rel: contains, ord)` を materialise。これで anchor は contains DAG に乗り、複数親(=句のトランスクルージョン)と被参照が可能になる。 {#01KYP2T291A47GHVAJE9HW691W}
3. 逆操作(降格)も可能: 誰からも指されなくなった anchor は payload へ畳み戻してよい。 {#01KYP2T291A47GHVAJE9HW691X}

[id=01KYP2T291A47GHVAJE9HW691Y]
これにより granularity は固定値ではなく **動的な量** になる。最初の凍結を間違えても、後から昇格で吸収できる。

---

## 3. ノード型カタログ {#01KYP2T291A47GHVAJE9HW691Z}

### ブロック型 {#01KYP2T291A47GHVAJE9HW6920}

[id=01KYP2T291A47GHVAJE9HW6921]
| type | payload(要点) |
|---|---|
| `section` | `{ heading: InlineAST }`。子は contains で持つ |
| `para` | `{ inline: InlineAST }` |
| `list` | `{ ordered: bool }`。各項目は contains された `para`/`section` |
| `table` | §5。次元の木 + セル |
| `math` | `{ tree: MathNode }` ブロック数式(§6) |
| `figure` | §7。記号図 or 写真 |
| `code` | `{ lang: string, src: string }` |
| `term` | `{ name: string }` 概念の宣言。定義は `defines` エッジで結ぶ |
| `anchor` | `{ inline: InlineAST }` 段落より細かいスパンを昇格させたノード(§2.4)。被参照・トランスクルージョン可 |
| `value` | `{ v: any, unit?: string }` 参照可能な単一値(任意/将来) |

### インライン AST {#01KYP2T291A47GHVAJE9HW6922}

``` {#01KYP2T291A47GHVAJE9HW6923}
Inline =
  | { t: "text",  s: string }
  | { t: "emph",  kind: "strong" | "em" | "code", children: Inline[] }
  | { t: "math",  tree: MathNode }            // インライン数式(再帰木)
  | { t: "ref",   to: NodeId, rel: EdgeRel }  // 相互参照 → Edge を materialise
  | { t: "term",  to: NodeId }                // 用語使用 → term へのリンク
  | { t: "anchor", to: NodeId }               // 昇格したスパンの位置(§2.4)。中身は anchor ノード側
```

---

## 4. エッジ関係カタログ {#01KYP2T291A47GHVAJE9HW6924}

[id=01KYP2T291A47GHVAJE9HW6925]
| rel | 向き(from → to) | 意味 | 軸 |
|---|---|---|---|
| `contains` | 親 → 子 | 構造の背骨(順序つき) | 1 |
| `defines` | 定義ブロック → term | 「このブロックがこの用語を定義する」 | 4 |
| `term-ref` | 使用ブロック → term | 用語の参照(インラインから materialise) | 4 |
| `supports` | ブロック → ブロック | 論拠・根拠 | 1(DAG) |
| `depends-on` | ブロック → ブロック | 依存(補題→定義 等) | 1(DAG) |
| `cites` | ブロック → 出典ノード | 引用 | 1 |
| `refers-to` | ブロック → 任意ノード | 弱い参照(ナビゲーション)。SML のインライン参照から materialise。論証的な意味は持たない | 1 |
| `instance-of` | ノード → スキーマ | 型付け(任意/将来) | — |

[id=01KYP2T291A47GHVAJE9HW6926]
語彙は最小から始め、足りなければ追加する。**まず凍結すべきは contains / defines / term-ref / supports / depends-on の5つ + ナビゲーション用 `refers-to`。** 意味エッジ(supports 等)はブロック属性で、ナビゲーションはインラインで書く、という層1側の役割分担は[D1](ref:decisions/d1)を参照。

---

## 5. 表 — 次元の木(Strata の心臓) {#01KYP2T291A47GHVAJE9HW6927}

[id=01KYP2T291A47GHVAJE9HW6928]
表を **セルの格子 + colspan** で持つのをやめる。これが軸3と軸6を同時に解く核心。

[id=01KYP2T291A47GHVAJE9HW6929]
表は3要素で持つ:

``` {#01KYP2T291A47GHVAJE9HW692A}
payload (type: table) {
  rows:  DimTree                       // 行軸の次元の木
  cols:  DimTree                       // 列軸の次元の木
  cells: { [coord: string]: CellValue }  // coord = 行の葉path × 列の葉path
}

DimTree = Dim[]
Dim    = { name: string, members: Member[] }      // name = 束ねる概念名(例 "year")
Member = { key: string, label?: InlineAST, children?: DimTree }  // children → 入れ子
CellValue = number | string | { ref: NodeId }     // 値、または value ノード参照
```

[id=01KYP2T291A47GHVAJE9HW692B]
**「結合された見出しセル」= children を持つ Member。** これ以上でも以下でもない。

[id=01KYP2T291A47GHVAJE9HW692C]
- 紙へ render → 葉の数だけ span するセルとして描画(物理は render 時に発生 = 軸6) {#01KYP2T291A47GHVAJE9HW692D}
- スクリーンリーダ → 「2025年のうち Q1」と読み下す {#01KYP2T291A47GHVAJE9HW692E}
- 機械 → ネストしたキーで渡る {#01KYP2T291A47GHVAJE9HW692F}

[id=01KYP2T291A47GHVAJE9HW692G]
「結合は意味か装飾か」という問いが **消滅** する。結合プリミティブが無く、あるのは次元の階層だけ。階層は本質的に意味だから。

### 例: 2階建てヘッダの表 {#01KYP2T291A47GHVAJE9HW692H}

``` {#01KYP2T291A47GHVAJE9HW692J}
              2025          2026
          Q1     Q2     Q1     Q2
Revenue   100    120    130    150
Cost       60     70     75     80
```

```yaml {#01KYP2T291A47GHVAJE9HW692K}
type: table
payload:
  rows:
    - name: metric
      members: [{ key: Revenue }, { key: Cost }]
  cols:
    - name: year
      members:
        - key: "2025"
          children:
            - name: quarter
              members: [{ key: Q1 }, { key: Q2 }]
        - key: "2026"
          children:
            - name: quarter
              members: [{ key: Q1 }, { key: Q2 }]
  cells:
    "Revenue|2025.Q1": 100
    "Revenue|2025.Q2": 120
    "Revenue|2026.Q1": 130
    "Revenue|2026.Q2": 150
    "Cost|2025.Q1": 60
    "Cost|2025.Q2": 70
    "Cost|2026.Q1": 75
    "Cost|2026.Q2": 80
```

[id=01KYP2T291A47GHVAJE9HW692M]
> [id=01KYP2T291A47GHVAJE9HW692N]
> pandas を知っているなら、これは MultiIndex(行・列の階層インデックス)そのもの。OLAP キューブの考え方。

---

## 6. 数式 — 再帰木(MathML サブセット) {#01KYP2T291A47GHVAJE9HW692P}

[id=01KYP2T291A47GHVAJE9HW692Q]
`math` の payload は再帰木。**人は TeX で書き、ロスレスにパースして木へ。** 木が canonical。

``` {#01KYP2T291A47GHVAJE9HW692R}
MathNode =
  | { op: "num",    v: string }
  | { op: "ident",  v: string }
  | { op: "row",    items: MathNode[] }
  | { op: "frac",   num: MathNode, den: MathNode }
  | { op: "sup",    base: MathNode, sup: MathNode }
  | { op: "sub",    base: MathNode, sub: MathNode }
  | { op: "sum",    sub?: MathNode, sup?: MathNode, body: MathNode }
  | { op: "fenced", open: string, close: string, body: MathNode }
  // … MathML Presentation のサブセットに準拠して拡張
```

[id=01KYP2T291A47GHVAJE9HW692S]
例: `\sum_{i=1}^{n} x_i`

```yaml {#01KYP2T291A47GHVAJE9HW692T}
op: sum
sub: { op: row, items: [ {op: ident, v: i}, {op: ident, v: "="}, {op: num, v: "1"} ] }
sup: { op: ident, v: n }
body: { op: sub, base: { op: ident, v: x }, sub: { op: ident, v: i } }
```

[id=01KYP2T291A47GHVAJE9HW692V]
**凍結(決定)**: canonical は **MathML Presentation のサブセット**とする。独自 MathNode は作らず、MathML の要素に写像する(`mfrac`/`msub`/`msup`/`munderover`/`mrow`/`mi`/`mn`/`mo` …)。これで既存レンダラ資産に乗れる。サブセットは最初は最小集合とし、必要な演算子が出たら随時追加する(本当に必要なら独自拡張も可)。
**オーサリング表面は TeX**。人は TeX で書き、パーサが MathML サブセットの木へロスレス変換する。日常の数式は TeX 相性が良いのでこちらに寄せる。

---

## 7. 図 — 記号図 と 写真 を分ける(軸5) {#01KYP2T291A47GHVAJE9HW692W}

### 7.1 記号図(グラフ・チャート): データを焼かない {#01KYP2T291A47GHVAJE9HW692X}

```yaml {#01KYP2T291A47GHVAJE9HW692Y}
type: figure
payload:
  kind: chart
  data-ref: <table か value ノードの NodeId>   # 既存データを再利用(焼かない)
  mark: line                                    # line | bar | point | area ...
  encode: { x: "quarter", y: "value", color: "metric" }
  caption: <InlineAST>
```

[id=01KYP2T291A47GHVAJE9HW692Z]
`data-ref` が既存の `table` ノードを指す → **同じデータがグラフにも表にも使われる**(データのトランスクルージョン)。元データが常に残るので軸5=L0(記号=データ)。

[id=01KYP2T291A47GHVAJE9HW6930]
> [id=01KYP2T291A47GHVAJE9HW6931]
> 宣言の語彙は Vega-Lite を参考にすると車輪の再発明を避けられる。

### 7.2 写真(画素に意味がある): 外部参照 + 構造化記述 {#01KYP2T291A47GHVAJE9HW6932}

```yaml {#01KYP2T291A47GHVAJE9HW6933}
type: figure
payload:
  kind: image
  src: "asset://photos/2026-ski-hakuba.jpg"   # 不透明な外部実体
  depicts:                                      # 単なる alt ではない構造化記述
    subject: "..."
    setting: "..."
  alt: "..."                                    # フォールバック文字列
  caption: <InlineAST>
```

[id=01KYP2T291A47GHVAJE9HW6934]
写真は意味層の外(別レイヤー)。`depicts` という外付け意味層でのみ機械可読化する。

---

## 8. トランスクルージョン(単一ソースの自動帰結) {#01KYP2T291A47GHVAJE9HW6935}

[id=01KYP2T291A47GHVAJE9HW6936]
「次元削減」の定義ノードは世界に1つだけ存在し、教科書ノートにもブログにもスライドにも `contains` で参照される(同一ノードに複数の親 = 不変条件5)。1箇所直せば全箇所に伝播する。

``` {#01KYP2T291A47GHVAJE9HW6937}
para#defn-dimred  ←contains─  section#textbook
        ↑contains
   section#blog
```

[id=01KYP2T291A47GHVAJE9HW6938]
Markdown/Obsidian では原理的にできない(ファイルにコピーが散る)。Strata では `contains` が DAG を許すことの自然な配当。

---

## 9. ドキュメント = グラフへのビュー {#01KYP2T291A47GHVAJE9HW6939}

[id=01KYP2T291A47GHVAJE9HW693A]
ドキュメントは実体ではなく **ビュー**。

``` {#01KYP2T291A47GHVAJE9HW693B}
View {
  id:        ULID
  root:      NodeId
  traversal: "contains-tree" | <query>   // 既定は contains を辿る木
  title:     InlineAST
}
```

[id=01KYP2T291A47GHVAJE9HW693C]
render はパートBの三層合成:

``` {#01KYP2T291A47GHVAJE9HW693D}
出力 = render(View, Style, Binding)
   View    … どのノード群を(意味)
   Style   … フォント/余白/結合セルの見た目化規則/図の回り込み(別ファイル)
   Binding … ページサイズ/画面幅/メディア種別(print|screen|braille|audio)
```

[id=01KYP2T291A47GHVAJE9HW693E]
同じ canonical から Binding を差し替えるだけで 紙 / 画面 / 点字 / 音声 が出る。音声出力では軸6が完全消滅する — これが「物理は意味の外」の証明。

---

## 10. オーサリング表現 (SML) とストア設計 {#01KYP2T291A47GHVAJE9HW693F}

[id=01KYP2T291A47GHVAJE9HW693G]
> [id=01KYP2T291A47GHVAJE9HW693H]
> **正典の委譲**: SML の詳細仕様(記法・fmt の契約・設計決定 D1〜D6 の記録)は
> **[grammar.sml](doc:grammar)+[decisions.sml](doc:decisions)** に移した。本節は概要のみを示す。両者が食い違う場合は
> そちらを正とする。

[id=01KYP2T291A47GHVAJE9HW693J]
真実の源 (source of truth) は、ドキュメント単位のプレーンテキストファイルである。このファイルを記述するためのフォーマットを **SML (Strata Markup Language)** と呼ぶ。

[id=01KYP2T291A47GHVAJE9HW693K]
SML は「人が書きやすく、かつAIが厳密に解釈できること」を目的として設計されており、人間（およびAI）の執筆体験（層1）と、データベースに構築される canonical グラフ（層2）の橋渡しを行う。

### 10.1 SML の処理パイプラインと ID インジェクション {#01KYP2T291A47GHVAJE9HW693M}

[id=01KYP2T291A47GHVAJE9HW693N]
人間やAIが執筆する際には、ノードの安定IDである `ULID` を意識する必要はない。IDの生成と参照の解決はすべて機械（フォーマッタ / コンパイラ）が自動で行う。

``` {#01KYP2T291A47GHVAJE9HW693P}
[執筆段階: ドラフトSML] (ID未付与、ラベル参照)
         │
         ▼ (strata fmt)
[保存・管理用SML] (ULID埋め込み済、逆注入後の状態)  <--- Git 等でバージョン管理
         │
         ▼ (strata build)
[Strata canonical グラフ (層2)] (Node と Edge のメモリ表現)
```

[id=01KYP2T291A47GHVAJE9HW693Q]
1. **フォーマッタ (`strata fmt`)**: 
   SMLファイルを静的解析し、IDが存在しないブロック（見出し、段落、表、図等）に対してユニークな `ULID` を自動生成して `{#ULID}` や `[id=ULID]` としてファイルへ**直接逆注入（インプレース上書き）**する。
   また、`[リンク名](cell:table-alias#coord)` のような人間用のエイリアス参照を、解決された `ULID` 参照（例: `cell:01J2...#coord`）へ書き換える。 {#01KYP2T291A47GHVAJE9HW693R}
2. **コンパイラ (`strata build`)**: 
   IDが注入されたSMLファイルを読み込み、`Node`（ID + Payload）および `Edge`（親子関係の `contains` や、アノテーションから抽出された `Ref` / `TermRef` / `Supports` 等）を構築し、インメモリのグラフインデックスに読み込む。 {#01KYP2T291A47GHVAJE9HW693S}

### 10.2 SML 構文規則 of 要点 {#01KYP2T291A47GHVAJE9HW693T}

[id=01KYP2T291A47GHVAJE9HW693V]
SML は基本的に Markdown (CommonMark) と互換性を持つが、Strata特有の高度な構造を記述するために以下のアノテーション記法を持つ。

#### A. ブロックアノテーション {#01KYP2T291A47GHVAJE9HW693W}
[id=01KYP2T291A47GHVAJE9HW693X]
ブロックの直前に `[id=..., supports=...]` のような属性リストを置くことで、ブロック間の意味的関係（エッジ）を宣言する。
```markdown {#01KYP2T291A47GHVAJE9HW693Y}
[id=01J2..., supports=01J2...]
段落のテキスト...
```

#### B. 多次元表の記述 (次元木 ＋ フラットセル) {#01KYP2T291A47GHVAJE9HW693Z}
[id=01KYP2T291A47GHVAJE9HW6940]
表は `::table` ブロックとして定義し、行/列の「次元の木」と「セルの座標値」を厳密に分離して記述する。
```markdown {#01KYP2T291A47GHVAJE9HW6941}
::table {#01J2...}
[caption="財務データ"]
@rows:
  - metric: [Revenue, Cost]
@cols:
  - year: [2025, 2026]
@cells:
  Revenue | 2025 : 100
  Cost    | 2025 : 60
::
```

#### C. セマンティック・リンケージ {#01KYP2T291A47GHVAJE9HW6942}
[id=01KYP2T291A47GHVAJE9HW6943]
文中から図表や、さらには表の中の特定の「セル」を明示的に指し示す。これにより、非正規化された `Ref` エッジが自動的にマテリアライズされる。
[id=01KYP2T291A47GHVAJE9HW6944]
*   図表への参照: `[表示名](fig:01J2...)` {#01KYP2T291A47GHVAJE9HW6945}
*   セルへの参照: `[表示名](cell:01J2...#RowPath|ColPath)` {#01KYP2T291A47GHVAJE9HW6946}

#### D. AI/マルチモーダルのための `depicts`（構造記述） {#01KYP2T291A47GHVAJE9HW6947}
[id=01KYP2T291A47GHVAJE9HW6948]
画像や図などの連続実体（画素）には、それが表現している意味内容（コンテキスト）をアノテーション `[depicts="..."]` として付与することを推奨・義務付ける。これにより、AIがマルチモーダル的にグラフ構造を理解するのを助ける。

---

## 11. Obsidian との根本的な違い(中身のデータの持ち方) {#01KYP2T291A47GHVAJE9HW6949}

[id=01KYP2T291A47GHVAJE9HW694A]
| | Obsidian | Strata |
|---|---|---|
| ノード粒度 | ノート1枚 | ブロック / 用語 / 表の1次元 |
| エッジ | 無型 wikilink | 型付き(defines / supports / depends-on …) |
| 表・数式・図 | 本文中の生 Markdown/LaTeX 文字列 | 構造化ノード(照会可・再render可) |
| 真実の源 | Markdown ファイル | プレーンテキスト SML (ID自動逆注入) |
| index | (なし。都度パース) | 派生・使い捨て(インメモリ Graph / 必要なら redb) |
| 重複 | コピペで散る | 単一ノード + 複数参照。編集が伝播 |
| 物理レイアウト | 本文に混入 | モデルに不在。render 時のみ発生 |
| 出力 | 基本1種 | 1ソースから 紙/画面/音声/機械 |

[id=01KYP2T291A47GHVAJE9HW694B]
見た目(ノート・リンク・グラフビュー)は Obsidian に寄せられる。下のデータの持ち方を「ファイルのグラフ」から「型付きノードのグラフ」に入れ替える。

---

## 12. 凍結 vs 保留 {#01KYP2T291A47GHVAJE9HW694C}

[id=01KYP2T291A47GHVAJE9HW694D]
**いま凍結する(契約):**
[id=01KYP2T291A47GHVAJE9HW694E]
- Node/Edge の2レコード構造(§2) {#01KYP2T291A47GHVAJE9HW694F}
- 不変条件 1〜6(§1) {#01KYP2T291A47GHVAJE9HW694G}
- rel 6種: contains / defines / term-ref / supports / depends-on / refers-to(§4) {#01KYP2T291A47GHVAJE9HW694H}
- 表 = 次元の木(§5)。結合プリミティブは永久に持たない {#01KYP2T291A47GHVAJE9HW694J}
- 物理レイアウトを payload に持たない(§1-2) {#01KYP2T291A47GHVAJE9HW694K}
- 数式: canonical = MathML Presentation サブセット、オーサリング = TeX(§6) {#01KYP2T291A47GHVAJE9HW694M}
- 粒度: レベル1 既定 + `anchor` への需要駆動の昇格(§2.4) {#01KYP2T291A47GHVAJE9HW694N}
- ストア・オーサリング: 真実の源 = SML プレーンテキスト(ID自動逆注入)、SML処理パイプライン(§10) {#01KYP2T291A47GHVAJE9HW694P}

[id=01KYP2T291A47GHVAJE9HW694Q]
**当面保留(後で決める):**
[id=01KYP2T291A47GHVAJE9HW694R]
- ID 方式(ULID か content-hash か。Xanadu の「局所名 + 不変 leaf」が先例) {#01KYP2T291A47GHVAJE9HW694S}
- MathML サブセットの正確な要素集合(最小から開始し随時拡張) {#01KYP2T291A47GHVAJE9HW694T}
- `value` ノードの粒度(prose と表で値を共有するか) {#01KYP2T291A47GHVAJE9HW694V}
- バージョニング / 編集履歴 {#01KYP2T291A47GHVAJE9HW694W}
- SML の詳細な EBNF 文法定義 ([付録A](ref:grammar/appendix-ebnf)のスケッチを実装時に厳密化。保留項目一覧も[grammar.sml の保留一覧](ref:grammar/frozen-vs-pending)へ移管) {#01KYP2T291A47GHVAJE9HW694X}
- index の永続化に redb を使うか(当面はインメモリ再構築) {#01KYP2T291A47GHVAJE9HW694Y}

---

## 13. 着手ロードマップ {#01KYP2T291A47GHVAJE9HW694Z}

[id=01KYP2T291A47GHVAJE9HW6950]
1. **層2スキーマ凍結** — §2・§5・§6 を確定。全ての契約。(→ strata-core.rs で型起こし済・cargo test パス) {#01KYP2T291A47GHVAJE9HW6951}
2. **SML パーサ・フォーマッタ実装** — ドラフトSMLからのID逆注入、およびSML ⇄ インメモリ Graph の load/save。 {#01KYP2T291A47GHVAJE9HW6952}
3. **render 1本** — canonical → HTML。ビュー=traversal を実証。結合セルが次元の木から正しく span することを確認。 {#01KYP2T291A47GHVAJE9HW6953}
4. **オーサリング統合** — SML 編集ツールの構築、またはエディタ連携。 {#01KYP2T291A47GHVAJE9HW6954}
5. 以降: 第2 render(Typst→PDF)、ノードベクトル検索、Obsidian 風UI。 {#01KYP2T291A47GHVAJE9HW6955}

[id=01KYP2T291A47GHVAJE9HW6956]
設計の全リスクは 1 に集中。1 が固まれば 2〜5 は差し替え可能。

---

## 14. 先行研究と位置づけ {#01KYP2T291A47GHVAJE9HW6957}

[id=01KYP2T291A47GHVAJE9HW6958]
Strata の個々の構成要素は、3つの没交渉なコミュニティ(技術文書/シングルソース出版、ハイパーテキスト/Xanadu、デジタル人文学/テキスト符号化)と、近年のAI文書処理に、ほぼ先行例がある。設計が独立に何度も再発見されているのは「地に足がついている」強い証拠。ここでは系統ごとに、Strata のどの判断に対応するかを添えて整理する。

### A. コンテンツ/プレゼンテーション分離・シングルソース出版 → 3層分離(§0) {#01KYP2T291A47GHVAJE9HW6959}

[id=01KYP2T291A47GHVAJE9HW695A]
- **概念**: 視覚・デザインを中核の素材・構造から切り離す「関心の分離」。XMLが内容を一度書いて複数形式へ出し分ける手法を確立(1990年代、Boeing/Ford マニュアル)。DITA / DocBook がその直系。 {#01KYP2T291A47GHVAJE9HW695B}
- Strata の層1/層2/層3 はこの延長。新規性なし。 {#01KYP2T291A47GHVAJE9HW695C}
- W3C TAG の警告「純粋な意味内容と単なる提示の間に明確な境界は無い」は、軸3(束ねは意味か装飾か)の難しさと同じ論点。 {#01KYP2T291A47GHVAJE9HW695D}
- 参考: https://en.wikipedia.org/wiki/Separation_of_content_and_presentation / https://en.wikipedia.org/wiki/Single-source_publishing / https://www.w3.org/2001/tag/doc/contentPresentation-26.html {#01KYP2T291A47GHVAJE9HW695E}

### B. Project Xanadu / トランスクルージョン → 単一ソース・物理排除・ID安定性(§1, §8, §12) {#01KYP2T291A47GHVAJE9HW695F}

[id=01KYP2T291A47GHVAJE9HW695G]
- **概念**: トランスクルージョンは内容を複製せず参照で取り込み、単一の真実の源を作る。Nelson の原則「マークアップは埋め込むな」「階層やファイルは文書の心的構造であってはならない」「リンクは双方向」。 {#01KYP2T291A47GHVAJE9HW695H}
- Strata の §8(単一ソース+複数参照)、軸6排除(物理を payload 型に持たせない=埋め込まない)、不変条件6(双方向に辿れる型付きエッジ)に対応。 {#01KYP2T291A47GHVAJE9HW695J}
- **ID安定性の解**: Xanadu系プロトタイプ Commonplace は、変更を許す「局所の未公開名」と不変な「公開済み leaf」を分離する。これは §12 保留の「ULID か content-hash か」の直接の先例 → **両方使う**(編集中はULID、確定時にcontent-hash leaf を発行)という解が示唆される。 {#01KYP2T291A47GHVAJE9HW695K}
- 参考: https://en.wikipedia.org/wiki/Project_Xanadu / https://en.wikipedia.org/wiki/Transclusion / https://github.com/complexitycollapse/commonplace-prototype / https://maggieappleton.com/transcopyright-dreams {#01KYP2T291A47GHVAJE9HW695M}

### C. OHCO論争 / Text As Graph (TAG) → 軸1(木からグラフへ)(§2, 不変条件6) {#01KYP2T291A47GHVAJE9HW695N}

[id=01KYP2T291A47GHVAJE9HW695P]
- **最重要の理論的裏付け**。1990年の OHCO説(テキスト=順序づけられた内容オブジェクトの階層=木)は、1996年に「一貫して維持できない」と反証された。原因は**重複階層**(詩の韻律行と文がまたがる等)。2008年に Tennison は重複を「マークアップに残された主要な問題領域」と評した。 {#01KYP2T291A47GHVAJE9HW695Q}
- 到達点の **TAG (Text As Graph)** は「有向プロパティ・ハイパーグラフ」で、ノードとエッジが型付きプロパティを持つ。XMLを「複数親を禁じた順序つきDAG」と批判する論拠が Strata の出発点と同一。Strata の「contains は複数親可のDAG、他rel は一般グラフ」はこの限界を外す設計。 {#01KYP2T291A47GHVAJE9HW695R}
- **standoff / LMNL** は範囲(range)に identity を与える非階層マークアップ。Strata の §2.4 anchor 昇格はこの系統。 {#01KYP2T291A47GHVAJE9HW695S}
- 参考: https://en.wikipedia.org/wiki/Overlapping_markup / https://www.balisage.net/Proceedings/vol19/html/Dekker01/BalisageVol19-Dekker01.html / "Refining our Notion of What Text Really Is" (Renear et al. 1996) {#01KYP2T291A47GHVAJE9HW695T}

### D. 形式意味論: A Core Calculus for Documents (arXiv 2023) → render(層3)の落とし穴 {#01KYP2T291A47GHVAJE9HW695V}

[id=01KYP2T291A47GHVAJE9HW695W]
- 文書言語を「ドメイン(文字列/木)×構成子(リテラル/プログラム/テンプレート)」の2軸8段で形式化。テンプレートと補間が**内容と計算を合成すると予期しない挙動を生む**と警告。 {#01KYP2T291A47GHVAJE9HW695X}
- Strata の render に動的な値の埋め込み(計算)を入れる段で直接効く。層3設計の指針。 {#01KYP2T291A47GHVAJE9HW695Y}
- 参考: https://arxiv.org/pdf/2310.04368 {#01KYP2T291A47GHVAJE9HW695Z}

### E. 抽象構文木としてのマークアップ → canonical = AST という発想 {#01KYP2T291A47GHVAJE9HW6960}

[id=01KYP2T291A47GHVAJE9HW6961]
- 「良いマークアップ言語は抽象的な文書ツリーを記述し、出力への適応は別プログラムに任せる。HTML/PDF/DocBook を直接吐くのは悪い。Markdown はこれに失敗(末尾二重スペース等)」という議論は §モチベーションの Markdown 批判と一致。 {#01KYP2T291A47GHVAJE9HW6962}
- 参考: https://matklad.github.io/2022/10/28/elements-of-a-great-markup-language.html {#01KYP2T291A47GHVAJE9HW6963}

### F. 現代の実装: MyST / PreTeXt / ブロックツール → 最も近い生きた先例 {#01KYP2T291A47GHVAJE9HW6964}

[id=01KYP2T291A47GHVAJE9HW6965]
実装前に研究すべき3つ。

[id=01KYP2T291A47GHVAJE9HW6966]
- **PreTeXt** — Strata の主張をほぼそのまま実現済み。厳密なXML文書構造に組版の強みを組み合わせ、**単一ソースから** PDF・EPUB・完全アクセシブルHTML・**Nemeth点字・触図** を生成する。数式は MathML、図はソースから自動生成(=焼かない)。Strata の「1ソース→紙/画面/音声/点字」が現実に動く証拠。最重要参照。 https://pretextbook.org/ {#01KYP2T291A47GHVAJE9HW6967}
- **MyST (Markedly Structured Text)** — CommonMark上位互換のオーサリング表面 + パース済みAST + 複数出力。`.json` でASTをそのまま公開し、数式・図・表・段落・式すべてを横断参照でき、出力形式間で参照が可搬。「オーサリング → AST(canonical) → 多出力」を実装した生きた層モデル。差分は: MyST のASTは木寄りで、Strata のような型付きグラフ/次元木テーブルまでは行かない。 https://mystmd.org/ {#01KYP2T291A47GHVAJE9HW6968}
- **ブロックツール (Roam / Logseq / Notion / Tana)** — ブロック参照=ブロック粒度の単一ソース(Strata レベル1相当)。中でも **Tana** は「より構造化されたデータ層(supertags)」を持ち、型付きノードという点で Strata に最も近い。ただしどれも構造化された table/math ノードや型付きエッジのグラフには至らない。 https://logseq.com/ / Tana {#01KYP2T291A47GHVAJE9HW6969}

### G. AIネイティブ: GraphRAG / 知識グラフ → 機械が読む層の今日的意義(§10.1) {#01KYP2T291A47GHVAJE9HW696A}

[id=01KYP2T291A47GHVAJE9HW696B]
- 近年の **GraphRAG** は、チャンク類似検索の限界を超えるため、文書からエンティティと関係を抽出して知識グラフを構築し、グラフ走査で検索する。ベクトルとグラフを統合した記憶層(Cognee 等)も登場。 {#01KYP2T291A47GHVAJE9HW696C}
- **Strata との決定的な差**: GraphRAG はオーサリング後の非構造テキストから**事後に**グラフを抽出する(ロスあり・誤りうる)。Strata はオーサリング時点から**グラフがネイティブ**(ロスレス)。「機械のためにグラフを後から復元する」のではなく「最初からグラフで書く」。ノード単位のベクトルを持てば、GraphRAG が事後抽出で目指すものを源で達成できる。Qdrant/ローカルLLM スタックと直結。 {#01KYP2T291A47GHVAJE9HW696D}
- 参考: GraphRAG関連(Microsoft GraphRAG, LightRAG, Cognee 等)。アクセシブル数学の知見「印刷優先で組版後にMathML化すると、大規模では再変換が高コストで誤りやすい。XML優先なら源で解決し、全下流出力が構造を自動継承する」は Strata の「焼かない/canonical が源」原則の実務的裏付け。 {#01KYP2T291A47GHVAJE9HW696E}

### H. 木モデル(MyST / PreTeXt)に対して Strata が足すもの — 精密な差分 {#01KYP2T291A47GHVAJE9HW696F}

[id=01KYP2T291A47GHVAJE9HW696G]
MyST と PreTeXt の実際のデータモデルを読んだ上での差分。**両者は OHCO(順序つき木)システム**である。MyST は unist/mdast の木(`{type, children, value, position}`)に directive/role を足したもの。PreTeXt は XML 木。どちらも単一階層の木に、ID/IDREF の相互参照を**後段で解決する**形で上乗せしている。Strata は §14C の OHCO 反証を受けて、**Text-as-Graph(型付きグラフ)** に踏み出す。これが一言での差分。

[id=01KYP2T291A47GHVAJE9HW696H]
具体的には4点を足す。

[id=01KYP2T291A47GHVAJE9HW696J]
1. **背骨がグラフ(木ではない)**。MyST/PreTeXt の構造の骨格は単一親の木で、相互参照は木に乗せた IDREF ポインタ(MyST では `crossReference` を解決フェーズで結ぶ)。Strata は `contains` 自体を複数親可の DAG にし(=トランスクルージョンがネイティブ。ディレクティブ機能ではなく構造)、相互参照を一級の型付きエッジに昇格する。グラフは「木への上乗せ」ではなく構造そのもの。 {#01KYP2T291A47GHVAJE9HW696K}

2. **エッジが意味を持つ**。MyST の `crossReference` は種別が `eq | numref | ref` で、実質「番号つきか否か」の**ナビゲーション**でしかない。「何が何を支持するか」という意味は持たない。Strata の `supports` / `depends-on` / `defines` は論証関係を持つので、グラフを**意味で照会**できる。 {#01KYP2T291A47GHVAJE9HW696M}

3. **表が次元の木(セル格子ではない)**。MyST の table は mdast/GFM の `tableRow → tableCell` のグリッドで、結合セルや多段見出しは一級市民ではない。PreTeXt の tabular も結合は表示上の span。どちらも「この列群は共通の次元に属する」という意味を持てない。Strata は表を行/列の次元の木 + 座標セルで持ち(§5)、軸3=L2 を達成、「結合は意味か装飾か」が消滅する。 {#01KYP2T291A47GHVAJE9HW696N}

4. **数式が MathML 木(LaTeX 文字列ではない)**。MyST の math ノードは LaTeX を `value` 文字列として持ち、MathML は描画時に生成する。PreTeXt も `<m>` 内は LaTeX で MathML はビルド生成物。**構造が源ではなく文字列が源**。Strata は **パース済み MathML 木を canonical** とし、TeX は単なるオーサリング表面(§6)。 {#01KYP2T291A47GHVAJE9HW696P}

[id=01KYP2T291A47GHVAJE9HW696Q]
(補足: MyST には `Target` による要素IDと `attrs_inline` のスパンID があり、Strata の §2.4 anchor 昇格に近い機構は**既にある**。また MyST の directive は `class`/`options` に表示ヒントが混じり、軸6が部分的に漏れる。Strata は型レベルで物理を排除する点が違う。)

[id=01KYP2T291A47GHVAJE9HW696R]
| 観点 | MyST / PreTeXt(木) | Strata(グラフ) |
|---|---|---|
| 背骨 | 単一親の木 + IDREF上乗せ | 複数親可DAG + 一級の型付きエッジ |
| 参照の意味 | ナビゲーション(ref/numref/eq) | 論証関係(supports/depends-on/defines) |
| トランスクルージョン | ディレクティブ等の機能 | 構造そのもの(複数親 contains) |
| 表 | セル格子(GFM/tabular) | 次元の木 + 座標セル |
| 数式 | LaTeX 文字列(MathMLは描画生成) | MathML 木が canonical |
| 物理レイアウト | directive の class 等に一部漏れ | 型レベルで排除 |

[id=01KYP2T291A47GHVAJE9HW696S]
**逆に Strata に無く彼らにあるもの(正直に)**: 成熟したオーサリング体験、巨大なレンダラ生態系、実働する EPUB/点字/触図出力、Jupyter による計算実行、大規模での実証。Strata は今のところ紙上設計。差分は双方向であり、Strata の貢献は「木でできることを置き換える」ことではなく「木では原理的に持てない4点(グラフ背骨・意味エッジ・次元木表・式の木)を canonical に持つ」ことにある。

### 新規性の正直な評価 {#01KYP2T291A47GHVAJE9HW696T}

[id=01KYP2T291A47GHVAJE9HW696V]
個々の要素はほぼ既出。Strata に独自性があるとすれば次の2点に絞られる。

[id=01KYP2T291A47GHVAJE9HW696W]
1. **3つの没交渉なコミュニティ + AI文脈の統合**。シングルソース出版・Xanadu・OHCO/TAG は普段引用し合わない。これらを「個人ナレッジ + AIワークフロー」で1つの canonical 設計に束ねた例は見当たらない。 {#01KYP2T291A47GHVAJE9HW696X}
2. **表を次元の木で持って軸3を「消す」手**。MultiIndex/OLAP はデータ分析の常識だが、それを文書フォーマットの canonical な表モデルに据えて「結合は意味か装飾か」という問い自体を消滅させる定式化は、上記文献群に明示的には見当たらない。物理レイアウトを型システムから構文的に排除し「漏洩を不可能にする」のも、原則は古いが型不変条件として強制する formulation は鋭い。 {#01KYP2T291A47GHVAJE9HW696Y}

[id=01KYP2T291A47GHVAJE9HW696Z]
結論: Strata は「ゼロからの発明」ではなく「確立した諸概念の統合 + 数手の鋭い定式化」。実装前に **PreTeXt と MyST の設計を読む**のが最短の学習路。
