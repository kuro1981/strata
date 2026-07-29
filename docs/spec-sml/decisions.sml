---
id: 01KXJR7K8EPXBEFCBZ0EXAK5Z2
title: SML 設計決定の記録
alias: decisions
---

# SML 設計決定の記録 {#01KXJR7K8EPXBEFCBZ0EXAK5Z3}

## 初期6決定(2026-07-13 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK5Z4 alias=initial-six}

[id=01KXJR7K8EPXBEFCBZ0EXAK5Z5]
本仕様の骨格は以下の6決定に基づく。変更する場合はここに追記して履歴を残すこと。

### D1 参照の意味をどの層で書くか {#01KXJR7K8EPXBEFCBZ0EXAK5Z6 alias=d1}

[id=01KXJR7K8EPXBEFCBZ0EXAK5Z7]
論点は、参照の意味をどの層で書くかである。

[id=01KXJR7K8EPXBEFCBZ0EXAK5Z8]
**ナビはインライン、意味はブロック属性**。インライン参照はナビゲーション(弱参照)のみ。`supports` 等の意味エッジはブロック属性行でのみ宣言する。

### D2 ID 記法 {#01KXJR7K8EPXBEFCBZ0EXAK5Z9 alias=d2}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZA]
論点は、ID 記法である。

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZB]
**行型ブロック**(見出し・リスト項目・フェンスマーカー)は行内 `{#id}`、**複数行プローズ**(段落)は直前の属性行 `[id=...]`。1ブロックに両方書いたらエラー。

### D3 エイリアスと ULID {#01KXJR7K8EPXBEFCBZ0EXAK5ZC alias=d3}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZD]
論点は、エイリアスと ULID である。

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZE]
**併存**。エイリアスは一級市民。fmt はエイリアスを消さず ULID を追記解決する。参照はエイリアス・ULID のどちらでも書ける。canonical グラフには ULID のみが入る。

### D4 表ブロック構文 {#01KXJR7K8EPXBEFCBZ0EXAK5ZF alias=d4}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZG]
論点は、表ブロック構文である。

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZH]
ネスト次元形(§6.1)を正とする。セル値は型付きパース: 裸の数値→Number、`<数値> <単位>`→数量として構造化、その他→Text。

### D5 member key の字句 {#01KXJR7K8EPXBEFCBZ0EXAK5ZJ alias=d5}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZK]
論点は、member key の字句である。

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZM]
key は `[A-Za-z0-9_-]+` に制限。表示名は `label` に逃がす。エスケープ文法は持たない。

### D6 fmt の実装方式 {#01KXJR7K8EPXBEFCBZ0EXAK5ZN alias=d6}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZP]
論点は、fmt の実装方式である。

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZQ]
**スパンパッチ方式**(CST/rowan は当面使わない)。fmt は挿入と `{...}` 内置換のみを行い、冪等・全か無か(§8)。

## fmt 実装後の精密化(2026-07-13 裁定) {#01KXJR7K8EPXBEFCBZ0EXAK5ZR alias=fmt-precision}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZS]
M2(fmt)実装で顕在化した曖昧点への裁定。いずれも [D1](ref:d1)〜[D6](ref:d6) の変更ではなく精密化。

### P1 意味保存の「ID無視同型」の定義 {#01KXJR7K8EPXBEFCBZ0EXAK5ZT alias=p1}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZV]
論点は、意味保存の「ID無視同型」の定義である。

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZW]
**属性行の `alias` エントリも ID 情報として無視する**(§8.1)。行末 `{#...}` タグを alias ごと無視するのと対称。

### P2 属性行 `id` の値の字句 {#01KXJR7K8EPXBEFCBZ0EXAK5ZX alias=p2}

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZY]
論点は、属性行 `id` の値の字句である。

[id=01KXJR7K8EPXBEFCBZ0EXAK5ZZ]
**裸トークン(ULID またはラベル)のみ**。引用符・リストは診断 `BadIdValue`、ラベルの字句違反は `BadKeyCharset`(§3.2)。

### P3 `{#label alias=x}`(非ULID + alias) {#01KXJR7K8EPXBEFCBZ0EXAK600 alias=p3}

[id=01KXJR7K8EPXBEFCBZ0EXAK601]
論点は、`{#label alias=x}`(非ULID + alias)の扱いである。

[id=01KXJR7K8EPXBEFCBZ0EXAK602]
**パーサが診断 `AliasWithoutUlid` で弾く**。alias を書けるのは ULID の id だけ。ドラフトでは `{#label}` とだけ書く(§3.1)。

### P4 CLI `fmt --check` の表示 {#01KXJR7K8EPXBEFCBZ0EXAK603 alias=p4}

[id=01KXJR7K8EPXBEFCBZ0EXAK604]
論点は、CLI `fmt --check` の表示である。

[id=01KXJR7K8EPXBEFCBZ0EXAK605]
パッチを**文書順(行番号昇順)**で「行:列: delete N byte(s), insert "…"」形式で表示。読み取り失敗は exit 1、診断種別は enum 名表記。

## M3(build)設計決定(2026-07-14 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK606 alias=m3-build}

### D7 build の配置 {#01KXJR7K8EPXBEFCBZ0EXAK607 alias=d7}

[id=01KXJR7K8EPXBEFCBZ0EXAK608]
論点は、build の配置である。

[id=01KXJR7K8EPXBEFCBZ0EXAK609]
新クレート **`strata-build`**(strata-sml + strata-core + tex2math に依存)。strata-sml の依存最小方針は不変。

### D8 strata-core 拡張 {#01KXJR7K8EPXBEFCBZ0EXAK60A alias=d8}

[id=01KXJR7K8EPXBEFCBZ0EXAK60B]
論点は、strata-core 拡張である。

[id=01KXJR7K8EPXBEFCBZ0EXAK60C]
`Rel::RefersTo` 追加 / `Inline::Ref` に `coord: Option<CellCoord>` 追加(serde は省略可能)/ `CellValue::Quantity { v, unit }` 追加(§9 の3課題を解決)。

### D9 `term:` の行き先 {#01KXJR7K8EPXBEFCBZ0EXAK60D alias=d9}

[id=01KXJR7K8EPXBEFCBZ0EXAK60E]
論点は、`term:` の行き先である。

[id=01KXJR7K8EPXBEFCBZ0EXAK60F]
**用語名から決定的に導出した安定 ID** で Term ノードを build が自動生成(同名 → 同 ID、毎 build 安定、ファイル横断でも同一)。defines 記法(保留)が入ったら定義ブロックと接続する。

### D10 コードフェンスの ID {#01KXJR7K8EPXBEFCBZ0EXAK60G alias=d10}

[id=01KXJR7K8EPXBEFCBZ0EXAK60H]
論点は、コードフェンスの ID である。

[id=01KXJR7K8EPXBEFCBZ0EXAK60J]
**行型として開始行末尾の `{#id}` を解禁**(§2 の分類に追加)。fmt の注入対象に含める。§10 の保留を解消。

### D11 リスト全体の ID {#01KXJR7K8EPXBEFCBZ0EXAK60K alias=d11}

[id=01KXJR7K8EPXBEFCBZ0EXAK60M]
論点は、リスト全体の ID である。

[id=01KXJR7K8EPXBEFCBZ0EXAK60N]
**リスト前置属性行の `id=` を解禁**(プローズ同様の前置 `[id=...]`)。項目は従来どおり行内 `{#id}`。M1 の「常に置き場所違反」を改定。

### D12 文書ルート {#01KXJR7K8EPXBEFCBZ0EXAK60P alias=d12}

[id=01KXJR7K8EPXBEFCBZ0EXAK60Q]
論点は、文書ルートである。

[id=01KXJR7K8EPXBEFCBZ0EXAK60R]
**フロントマター**(§2.1)を導入し、canonical に **Document ノード**を新設して全トップレベルブロックを contains する。フロントマターは必須ではない(無ければ Document ノードも無しのフォレスト)が、**fmt が無ければ生成する**ため fmt 済みファイルは常に持つ。

### D13 build のエラー方針 {#01KXJR7K8EPXBEFCBZ0EXAK60S alias=d13}

[id=01KXJR7K8EPXBEFCBZ0EXAK60T]
論点は、build のエラー方針である。

[id=01KXJR7K8EPXBEFCBZ0EXAK60V]
**全か無か**(fmt と同じ、§8.2)。パースエラー・未解決エイリアス・エイリアス重複定義・ULID 未付与ブロックをすべて収集して全件報告し、グラフを出力しない。

## M3 実装後の裁定(2026-07-14 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK60W alias=m3-post}

[id=01KXJR7K8EPXBEFCBZ0EXAK60X]
M3(build)実装で顕在化した曖昧点への裁定。[D14](ref:d14) は [D8](ref:d8)/[D13](ref:d13) の実装済み拡張の追認、[D15](ref:d15)〜[D17](ref:d17) は精密化。

### D14 build エラーの拡張の追認 {#01KXJR7K8EPXBEFCBZ0EXAK60Y alias=d14}

[id=01KXJR7K8EPXBEFCBZ0EXAK60Z]
論点は、build エラーの拡張の追認である。

[id=01KXJR7K8EPXBEFCBZ0EXAK610, revises=[d8, d13]]
strata-build の `BuildError::RefTypeMismatch { span, msg }`(参照スキーム `table:`/`fig:`/`math:`/`cell:` と対象ノード型の不一致。黙認しない)と `BuildError::Invariant(Violation)`(build 後の `invariants::validate` 違反の包み。ソース位置なし)を正式承認。あわせて tex2math に `\hat`(`UnderOver` の over-only 形)を追加したことを M3 実装の追認として記録する。

### D15 Term ID v0 の Unicode 正規化 {#01KXJR7K8EPXBEFCBZ0EXAK611 alias=d15}

[id=01KXJR7K8EPXBEFCBZ0EXAK612]
論点は、Term ID v0 の Unicode 正規化である。

[id=01KXJR7K8EPXBEFCBZ0EXAK613, depends-on=d9]
Term 安定 ID の導出式([D9](ref:d9))を「名前を **NFC 正規化**してから `strata:term:v0:{name}` を SHA-256」に改めて v0 凍結。視覚的に同じ用語名が NFC/NFD の違いで別 ID になる問題を解消する。

### D16 Table.caption / Chart.depicts {#01KXJR7K8EPXBEFCBZ0EXAK614 alias=d16}

[id=01KXJR7K8EPXBEFCBZ0EXAK615]
論点は、Table.caption / Chart.depicts である。

[id=01KXJR7K8EPXBEFCBZ0EXAK616]
strata-core に後方互換で追加: `Table.caption: Option<Vec<Inline>>`、`Chart.depicts: BTreeMap<String,String>`(`ImageFigure.depicts` と同形・同じキー畳み規則)。build がフェンス内属性行(`[caption=...]`・`[depicts=...]`/`[depicts.*]`)から写す。

### D17 Diag に severity(Error/Warning)を導入 {#01KXJR7K8EPXBEFCBZ0EXAK617 alias=d17}

[id=01KXJR7K8EPXBEFCBZ0EXAK618]
論点は、Diag に severity(Error/Warning)を導入することである。

[id=01KXJR7K8EPXBEFCBZ0EXAK619]
診断に `Error`/`Warning` の重大度を持たせる。新設2種別(`DuplicateFrontmatterKey`・`UnknownAttrKey`)のみ `Warning`、既存種別はすべて `Error`。「全か無か」(§8.2)は **`Error` にのみ適用**: `Warning` だけの入力は fmt/build とも成功し、`Warning` を結果と併せて返す。

## M4(render)設計決定(2026-07-14 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK61A alias=m4-render}

### D18 M4 のスコープと CLI {#01KXJR7K8EPXBEFCBZ0EXAK61B alias=d18}

[id=01KXJR7K8EPXBEFCBZ0EXAK61C]
論点は、M4 のスコープと CLI である。

[id=01KXJR7K8EPXBEFCBZ0EXAK61D]
サブコマンド `render` を新設: `strata-cli render <file.sml> [-o out.typ]`。内部で build → render を直結し、中間 JSON は介さない(JSON 入力のパイプライン分割は将来の拡張として保留)。exit code は build と同じ 0/1/2、Warning は stderr 表示のうえ exit 0。

### D19 レンダラの主従 {#01KXJR7K8EPXBEFCBZ0EXAK61E alias=d19}

[id=01KXJR7K8EPXBEFCBZ0EXAK61F]
論点は、レンダラの主従である。

[id=01KXJR7K8EPXBEFCBZ0EXAK61G]
**Typst を一次レンダラ**とし、M4 以降の新語彙の描画対応は strata-typst にのみ実装する。strata-html は現状機能のまま**凍結**(クレートは残置するが CLI からの導線は持たない)。Web 表示は将来の「グラフ UI」フェーズで再設計する — 意味グラフ文書の閲覧体験(ノード単位のナビゲーション)は Typst の HTML export では制御できないため、その時点で自前 HTML を再起動する。`render` の `--format` は当面 Typst のみ。

### D20 vault の削除 {#01KXJR7K8EPXBEFCBZ0EXAK61H alias=d20}

[id=01KXJR7K8EPXBEFCBZ0EXAK61J]
論点は、vault の削除である。

[id=01KXJR7K8EPXBEFCBZ0EXAK61K]
crates/strata-vault・`vault/`(YAML データ)・CLI の旧 YAML フロー(`run_legacy`)を**削除**する。履歴書は Strata 本線に持ち込まない(ユーザー裁定)。必要になれば git 履歴から採取できる。

### D21 Document の描画 {#01KXJR7K8EPXBEFCBZ0EXAK61M alias=d21}

[id=01KXJR7K8EPXBEFCBZ0EXAK61N]
論点は、Document の描画である。

[id=01KXJR7K8EPXBEFCBZ0EXAK61P]
`Document.title` は文書メタ(`#set document(title: ...)`)にのみ使い、本文には出さない(本文見出しは Section の H1 に任せ二重表示しない)。title 無しは最初の H1 のプレーンテキスト、それも無ければ入力ファイル名にフォールバック。`root: None`(フロントマター無し)の render はエラー(「strata fmt を先に実行してください」案内、exit 2)。

### D22 参照の描画 {#01KXJR7K8EPXBEFCBZ0EXAK61Q alias=d22}

[id=01KXJR7K8EPXBEFCBZ0EXAK61R]
論点は、参照の描画である。

[id=01KXJR7K8EPXBEFCBZ0EXAK61S]
全ブロックノードに Typst ラベル `<ULID>` を付与する。`::table`/`::math`/`::figure` は Typst の `figure` 要素に包んで**自動番号付け**(「表 1」「図 1」「式 1」、`set text(lang: "ja")`)を得る。`Ref` は表示 `text` があれば `#link(<label>)[text]`、無ければ `@ULID`(自動番号参照)。番号を持たない対象(段落・リスト・コード)への text 無し参照は `#link` + 短い代替表記とする(詳細はハンドオフの裁量)。`Term` は `text` → Term ノードの `name` の順で表示(用語集・defines は引き続き保留)。

## ドッグフーディング後の裁定(2026-07-14 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK61T alias=dogfooding}

[id=01KXJR7K8EPXBEFCBZ0EXAK61V]
M4 完了後、実履歴書(2文書・30プロジェクト規模)を SML 化するドッグフーディングで顕在化した摩擦点への裁定。背景となる定性評価: 「誰に見せるか」の条件分岐は文書ではなくビューの仕事(PowerPoint の発表者ビューと同型)/ 文書と組版を一体化すると難しくなる / 人はテンプレート先行で「埋める」のに現状はテキスト先行で「引き抜く」ため頭の動きが合っていない(→ ビュー/テンプレート層、§10 保留)。

### D23 出し分けの層分離 {#01KXJR7K8EPXBEFCBZ0EXAK61W alias=d23}

[id=01KXJR7K8EPXBEFCBZ0EXAK61X]
論点は、出し分けの層分離である。

[id=01KXJR7K8EPXBEFCBZ0EXAK61Y]
文書には「この情報が**何であるか**」の意味分類だけを書く: ブロック属性 **`class=<タグ,...>`**(タグの字句は key と同じ、複数可)。「誰に見せるか」(audience)は文書に書かず**ビュー側**で決める。v0 のビュー指定は CLI `render --hide <class>`(複数指定可)で、該当 class を持つブロックを**サブツリーごと**非描画にする。build は class 非依存で全ノードをグラフに格納する(ビューが変わっても真実は一つ)。非描画ノードへの `Ref` が残った場合はビュー側ポリシーとして Warning + リンクを剥がしてプレーンテキスト化。インラインの出し分け(見出し内の実名等)は保留 — 文書構造をねじ曲げる回避策(専用の小段落等)は採らない。

### D24 ネストリスト {#01KXJR7K8EPXBEFCBZ0EXAK61Z alias=d24}

[id=01KXJR7K8EPXBEFCBZ0EXAK620]
論点は、ネストリストである。

[id=01KXJR7K8EPXBEFCBZ0EXAK621]
リスト項目の**子リスト**を正式対応する(パーサ・fmt の ID 注入・build の contains・render)。「項目=段落1つ」の制約は維持(項目内複数ブロックは引き続き保留 §10)。従来の無警告誤パース(インデントされた子項目が `- ` 混じりの別段落に化ける)は根絶し、リストとして解釈できないインデント行には診断を出す。

### D25 組版の改善方針 {#01KXJR7K8EPXBEFCBZ0EXAK622 alias=d25}

[id=01KXJR7K8EPXBEFCBZ0EXAK623]
論点は、組版の改善方針である。

[id=01KXJR7K8EPXBEFCBZ0EXAK624]
**SML に組版ヒント(列幅・改ページ等)は導入しない**(意味と体裁の分離)。レンダラ既定の改善のみで対処する: 表 figure の breakable 化、列幅戦略の見直し(長文列の潰れ・1文字縦書き化の防止)、表前の空白ページ解消。合格基準は「30行ネスト表が読める・重ならない・空白ページなし」。体裁のカスタマイズは将来の「ビュー/テンプレート層」で文書外(ビュー定義)に持つ。

## M4y(構造化データ語彙)設計決定(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK625 alias=m4y-structured-data}

[id=01KXJR7K8EPXBEFCBZ0EXAK626]
ビュー v0(バインディング=コード)の棚卸しで定量化された語彙欠落への裁定(年月分解の正規表現 26 件・「キー: 値」分解 13 件・メタ行の DRY 違反 30 件)。

[id=01KXJR7K8EPXBEFCBZ0EXAK627]
**設計原理(正本と最終構造)**: 正本(SML)は「最も分解された事実」を記録する。最終構造(テンプレートのスロット)はビューの都合であり、形の変換はバインディングの仕事。ただしビュー群が要求する粒度は正本の分解粒度の**下限**として逆算してよい(粒度は逆流させてよいが、形は逆流させない)。合成テキストから構造は復元できないが、分解された事実から任意の粗い形は導出できる、という非対称性が根拠。

### D26 alias のグラフ出力 {#01KXJR7K8EPXBEFCBZ0EXAK628 alias=d26}

[id=01KXJR7K8EPXBEFCBZ0EXAK629]
論点は、alias のグラフ出力である。

[id=01KXJR7K8EPXBEFCBZ0EXAK62A]
build は解決済みエイリアスを graph JSON に出力する(ビューのアドレス規約の柱。v0 では caption 部分一致で代用する羽目になった)。表現形式は裁量。

### D27 子 List ノードの ID {#01KXJR7K8EPXBEFCBZ0EXAK62B alias=d27}

[id=01KXJR7K8EPXBEFCBZ0EXAK62C]
論点は、子 List ノードの ID である。

[id=01KXJR7K8EPXBEFCBZ0EXAK62D, depends-on=d9]
SML 上に子リスト全体の ID を書く場所が無いため、**親リスト項目の ULID+位置から決定的に導出**する([D9](ref:d9) の Term 安定 ID と同型)。毎 build 安定にし、ID 安定の不変条件との矛盾を解消。

### D28 `::record` フェンス {#01KXJR7K8EPXBEFCBZ0EXAK62E alias=d28}

[id=01KXJR7K8EPXBEFCBZ0EXAK62F]
論点は、`::record` フェンスである。

[id=01KXJR7K8EPXBEFCBZ0EXAK62G]
key-value ブロックを新設: `::record {#id alias=...}`、本体は「キー: 値」の行の列。**キーは自由テキスト(日本語可)** — 表の座標キー([D5](ref:d5)、ASCII 限定)とは別物でパス構文に入らない。値は表セルと同じ型付きパース(Number/Quantity/Date/Period/Text/Ref)。core に `NodePayload::Record`。標準ビュー(render)では2列表相当で描画。ネスト record は保留。診断: `RecordMissingColon`/`RecordEmptyKey` は Error、`DuplicateRecordKey` は **Warning**([D17](ref:d17) の Warning 系はこれを加えた3種 — 2026-07-15 記載漏れ追記)。

### D29 日付・期間のセル値型 {#01KXJR7K8EPXBEFCBZ0EXAK62H alias=d29}

[id=01KXJR7K8EPXBEFCBZ0EXAK62J]
論点は、日付・期間のセル値型である。

[id=01KXJR7K8EPXBEFCBZ0EXAK62K]
`CellValue::Date { y, m, d? }` / `Period { from, to? }`(to 無し=「現在」)。**既定の受理書式は ISO(`YYYY-MM-DD` / `YYYY-MM`)のみ**とし、書式スニッフィングはしない。フェンス属性 `date-format=` で当該ブロックの追加入力書式を明示宣言できる(例: `date-format="YYYY年M月"`)— 明示宣言でパースを容易にする(ユーザー裁定)。期間は「A 〜 B」「A 〜 現在」(`〜`/`~` 両可)。表示の日本語化・年齢等の導出値の計算はビュー側の仕事。

## ビュー v1(宣言的ビュー定義)設計決定(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK62M alias=view-v1}

[id=01KXJR7K8EPXBEFCBZ0EXAK62N]
バインディングを「コード」から「宣言的定義+決定的実行器」へ。dry-run 検証と将来の LLM 提案(レビュー可能なバインディング案)の土台。

### D30 実行形態 {#01KXJR7K8EPXBEFCBZ0EXAK62P alias=d30}

[id=01KXJR7K8EPXBEFCBZ0EXAK62Q]
論点は、実行形態である。

[id=01KXJR7K8EPXBEFCBZ0EXAK62R]
新サブコマンド **`strata view <file.sml> --view <def.yaml> [-o outdir]`**(新クレート strata-view)。入力は SML(内部 build、render と同じ流儀)。出力は**テンプレート消費用のデータファイル群**(YAML/JSON)。Typst を知らないデータ層に留める。

### D31 セレクタ語彙 {#01KXJR7K8EPXBEFCBZ0EXAK62S alias=d31}

[id=01KXJR7K8EPXBEFCBZ0EXAK62T]
論点は、セレクタ語彙である。

[id=01KXJR7K8EPXBEFCBZ0EXAK62V]
**alias / class / セル座標 / 型+contains パス**を一級(v0 実測の頑健度順)。見出しテキスト一致は警告付きエスケープハッチ。**正規表現は入れない** — 表現できない時は SML 側の構造化が正解というシグナルとする。

### D32 変換語彙 {#01KXJR7K8EPXBEFCBZ0EXAK62W alias=d32}

[id=01KXJR7K8EPXBEFCBZ0EXAK62X]
論点は、変換語彙である。

[id=01KXJR7K8EPXBEFCBZ0EXAK62Y]
**固定コンビネータの小セットのみ**: rename/pick、rows(表→配列)、join(木→文字列)、date(書式)、age/as-of(導出)、literal、class フィルタ。**汎用式・スクリプト埋め込みは導入しない**(XSLT の轍)。不足時は「コンビネータを1個足す裁定」か「SML 側を直す」の二択。

### D33 テンプレート・マニフェスト {#01KXJR7K8EPXBEFCBZ0EXAK62Z alias=d33}

[id=01KXJR7K8EPXBEFCBZ0EXAK630]
論点は、テンプレート・マニフェストである。

[id=01KXJR7K8EPXBEFCBZ0EXAK631]
テンプレート側に必要スロットの宣言(YAML・手書きで起こす)。**`strata view --check`** の dry-run で未充足スロット/未使用ノードを診断(fmt/build と同じ表示流儀)。テンプレ内データハードコードの検出器を兼ねる。

### D34 出し分けプロファイル {#01KXJR7K8EPXBEFCBZ0EXAK632 alias=d34}

[id=01KXJR7K8EPXBEFCBZ0EXAK633]
論点は、出し分けプロファイルである。

[id=01KXJR7K8EPXBEFCBZ0EXAK634, depends-on=d23]
ビュー定義内に profile(submit/check 等)を宣言し `--profile` で選択。1 定義から複数出力。[D23](ref:d23) の class がフィルタ条件の語彙。

[id=01KXJR7K8EPXBEFCBZ0EXAK635]
ビュー定義ファイルの文法は実装時に `docs/view-def-v1.md` として起草し、次の対話で批准する(v1 の品質基準: 定義ファイルが「読んで分かる」こと — LLM 提案・人間承認のレビュー対象になれる可読性)。

## M5-A(AI が読む: コンテキストビュー)設計決定(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK636 alias=m5a-context}

[id=01KXJR7K8EPXBEFCBZ0EXAK637]
M5(AI 連携)を「A: AI が読む / B: AI が書く / C: AI が設計する(ビュー定義提案)」に分解し、A から着手(B の前提であり、C は既存材料の手順設計で足りるため)。

### D36 `strata context` {#01KXJR7K8EPXBEFCBZ0EXAK638 alias=d36}

[id=01KXJR7K8EPXBEFCBZ0EXAK639]
論点は、`strata context` である。

[id=01KXJR7K8EPXBEFCBZ0EXAK63A, depends-on=d32]
AI 向けコンテキストビューの専用サブコマンド `strata context <file.sml> [-o out.md]`。出力は **ULID 付き Markdown+意味エッジ一覧**(LLM が読みやすく、回答の根拠を ULID/alias で引用させられる形)。グラフの chunk 分割は固定長ではなく**意味の単位(ノード/サブツリー)**で行う。スコープ3形態: (1) 無指定=全文書、(2) `--node <alias|ULID>`(複数可)+ `--hops N`(既定1)— contains サブツリーが chunk 本体、意味エッジ(supports/depends-on/refers-to/term)を N ホップ辿った近傍を文脈として付加、(3) `--class <tag>` — 意味分類での横断抽出(例: note だけ集める)。エッジ種の選別パラメータは保留([D32](ref:d32) と同じ「使ってから裁定」運用)。exit code・Warning の流儀は他コマンドと同一。

### D37 AI が書く(M5-B) {#01KXJR7K8EPXBEFCBZ0EXAK63B alias=d37}

[id=01KXJR7K8EPXBEFCBZ0EXAK63C]
論点は、AI が書く(M5-B)ことである。

[id=01KXJR7K8EPXBEFCBZ0EXAK63D]
作法の文書化+検証ループの規定であり新機能ではない。**正典は `docs/sml-agent-guide.md`**(どの AI エージェントでも読める自己完結の執筆ガイド。Claude Code スキル等は後日の薄いラッパで、正典は docs 配下 — .claude/ は gitignore 済みのため版管理に乗らない)。作法: (1) **AI は ULID を書かない**(fmt に任せる)、**alias は積極的に付ける**、既存ノード参照は context 出力のタグ(SML と同一記法)からコピー。(2) エッジは「**確信のあるものだけ張る、推測で張らない**」— 誤エッジは無エッジより害。迷ったら人間に提案として報告。(3) AI 下書きの専用 class は**付けない**(意味語彙を執筆プロセスの都合で汚染しない。レビューは git diff=人間のコミット指示が承認ゲート)。(4) 書き込み後の必須検証シーケンス: `fmt` → `build` →(該当時 `view --check`)、診断は AI が自分で解消してから人間レビューへ。

## MD レンダラ(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK63E alias=md-renderer}

### D39 MD 上位互換の原則 {#01KXJR7K8EPXBEFCBZ0EXAK63F alias=d39}

[id=01KXJR7K8EPXBEFCBZ0EXAK63G]
論点は、MD 上位互換の原則である。

[id=01KXJR7K8EPXBEFCBZ0EXAK63H]
「**MD でできることが全部できないと片手落ち**」(ユーザー裁定)。SML は素の Markdown の**上位互換**であることを原則とする — 素の .md ファイルがそのまま有効な SML ドラフトであり、`render --format md` で情報を失わず戻ること(純 MD 文書の round-trip)。具体的な対応範囲(CommonMark コア+GFM 拡張のどこまでか、inline HTML の扱い等)は**互換性監査の結果表を見て個別裁定**する。既知の違反: 外部リンクの無診断不活性化(§5.2)は本原則に反するため解消対象。

### D38 素の Markdown 出力 {#01KXJR7K8EPXBEFCBZ0EXAK63J alias=d38}

[id=01KXJR7K8EPXBEFCBZ0EXAK63K]
論点は、素の Markdown 出力である。

[id=01KXJR7K8EPXBEFCBZ0EXAK63M, depends-on=d19]
「シンプルな内蔵アウトプットは MD」(ユーザー裁定)。**`render --format md` を追加**([D19](ref:d19) 改定: `--format` は `typst`(既定・一次レンダラ)と `md`。人間向けの最小依存ビュー — GitHub・チャット・エディタプレビュー)。`--hide` 等のビュー機能は共有。**context との役割分担**: `context`=AI 向け(ULID タグ・エッジ一覧あり)/ `--format md`=人間向け清書(**`{#ULID}` タグは一切出さない**)。多次元表は GFM 表へ平坦化(ネスト行次元=親ラベル繰り返し、ネスト列=パス連結ヘッダ。GFM に rowspan/colspan が無いため)。record は2列表。数式は **MathNode → TeX 逆直列化**(`$...$`/`$$...$$`。core に生 TeX は保存しない — canonical は分解された事実のみの原理と一貫、round-trip が tex2math の検証を兼ねる)。参照は見出し=GFM アンカーリンク、番号参照不能なもの=「(表: キャプション)」形式のテキスト退化。figure は depicts プレースホルダ/`![alt](src)`。ファイル横断リンクはスコープ外(§10 ワークスペース層)。

## M6(CommonMark/GFM 互換)設計決定(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK63N alias=m6-commonmark}

[id=01KXJR7K8EPXBEFCBZ0EXAK63P]
[D39](ref:d39) の互換性監査(`docs/md-compat-audit.md` — 「静かに壊れる」9件を含む4分類)への裁定。

### D40 互換範囲と扱い {#01KXJR7K8EPXBEFCBZ0EXAK63Q alias=d40}

[id=01KXJR7K8EPXBEFCBZ0EXAK63R]
論点は、互換範囲と扱いである。

[id=01KXJR7K8EPXBEFCBZ0EXAK63S, depends-on=d19, revises=d39]
**Tier 1(CommonMark コアの穴埋め)+ Tier 2(GFM 実用拡張)を1マイルストーンで実装**(ユーザー裁定)。Tier 1: インラインエスケープ(`\*` 等)/ 外部リンク(`http(s)`・`mailto`)・autolink・インライン画像 `![alt](url)` / 参照スタイルリンク(定義行は非可視メタとして解決)/ ゆるいリスト統合(空行区切りの同種リストは1つの List)/ 順序リスト `start` 保存 / 強調ネスト(`***bold italic***`)修正 / Setext 見出し / `~~~` フェンス / blockquote / 代替記法(`*`・`+` 箇条書き、`1)`、`_em_`・`__strong__`)/ 見出し閉じ `#` 装飾。Tier 2: GFM パイプ表を**フラット2次元の Table ノードへブリッジ** / `~~取消線~~` / タスクリスト(チェック状態の構造化)。Tier 3(意図的非対応の明記): **HTML ブロック/インラインは非対応+Warning**(意味グラフに落ちない。リテラル扱いは維持)、**脚注は保留**(§10)。新語彙は [D19](ref:d19) に従い core→パーサ→fmt→build→**strata-typst→strata-context**の全層へ波及させる。未知スキームの `UnknownScheme` Error は真に未知のものに限定して維持。

## M7(ワークスペース層)設計決定(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK63T alias=m7-workspace}

[id=01KXJR7K8EPXBEFCBZ0EXAK63V]
ファイル横断の実害3件(resume→work_history 参照不能 / cv-basic-info record 複製 / licenses.yaml 越境出力)の解消。§2 の「ファイル→文書の対応管理は将来の vault 層」の実装第一歩。

### D41 ワークスペースの定義 {#01KXJR7K8EPXBEFCBZ0EXAK63W alias=d41}

[id=01KXJR7K8EPXBEFCBZ0EXAK63X]
論点は、ワークスペースの定義である。

[id=01KXJR7K8EPXBEFCBZ0EXAK63Y]
**明示的な定義ファイル `strata.toml`**(members のグロブ列挙)。ディレクトリ暗黙スキャンはしない(ユーザー裁定「スキャンは不要」— 生成物・下書きの誤取り込み防止、[D29](ref:d29) 以来の「スニッフィングせず宣言」の一貫)。フロントマターに **`alias` キー**(文書エイリアス)を追加(v0 の許可キーは id / title / alias の3つに)。

### D42 横断参照の記法と解決 {#01KXJR7K8EPXBEFCBZ0EXAK63Z alias=d42}

[id=01KXJR7K8EPXBEFCBZ0EXAK640]
論点は、横断参照の記法と解決である。

[id=01KXJR7K8EPXBEFCBZ0EXAK641]
**ULID 参照は無修飾のままワークスペース全体で解決**(ULID はグローバル一意)。**alias 参照は `ref:<文書alias>/<ブロックalias>`** のスラッシュ修飾(無修飾=同一文書。`#` はセル座標専用のまま)。解決は **store/index 分離**(設計対話ログ §12 の原則): store=.sml 群は探索しない、build が全メンバーを一括パースして作る**インメモリの ID→ノード表(index、使い捨て)**で O(1) 解決。「毎回探査」への答えは index であり、規模が小さいうちは毎回再構築で足りる。**index の永続化・インクリメンタル更新(redb 等)は保留**(§10 — 大規模化時に別裁定。永続 index も常に再構築可能な派生物とし、正本性を持たせない)。

### D43 workspace build とスコープ {#01KXJR7K8EPXBEFCBZ0EXAK642 alias=d43}

[id=01KXJR7K8EPXBEFCBZ0EXAK643]
論点は、workspace build とスコープである。

[id=01KXJR7K8EPXBEFCBZ0EXAK644]
**`strata build --workspace <strata.toml>`** で単一の統合グラフを出力(横断参照が普通の Edge に、Term は [D9](ref:d9) の安定 ID で自然合流)。**ファイル間 ULID 衝突検出**を診断に追加(コピペ事故)。既存の単一ファイル build・**fmt は不変**(fmt は単一ファイル・依存最小のまま。横断解決は build の仕事)。v0 スコープは build 統合+参照解決+**view の複数文書入力**(実害2・3の正規解)まで。context / render の横断(MD ページ間リンク含む)は v0.5 に分割。

## M7.5(ワークスペース v0.5 + 積み残し裁定)設計決定(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK645 alias=m7-5-workspace}

### D44 render / context の workspace 対応(v0.5) {#01KXJR7K8EPXBEFCBZ0EXAK646 alias=d44}

[id=01KXJR7K8EPXBEFCBZ0EXAK647]
論点は、render / context の workspace 対応(v0.5)である。

[id=01KXJR7K8EPXBEFCBZ0EXAK648]
**`render --workspace <strata.toml> [--doc <文書alias>] --format <typst|md>`** と **`context --workspace`** を追加。cross-doc 参照を含む文書の再生成不能(M7 の既知副作用)を解消。MD のページ間リンクは**相対 .md リンク+アンカー**(出力ファイル名はメンバーのファイル名 stem 由来)。Typst 単文書出力での他文書参照は退化テキスト(文書名付き)— 詳細裁量。

### D45 `concat` コンビネータ採用(D35 の再裁定) {#01KXJR7K8EPXBEFCBZ0EXAK649 alias=d45}

[id=01KXJR7K8EPXBEFCBZ0EXAK64A]
論点は、`concat` コンビネータ採用([D35](ref:d35) の再裁定)である。

[id=01KXJR7K8EPXBEFCBZ0EXAK64B, depends-on=[d35, d32]]
実需2件(tech-stack の details(level) 結合 / cv 氏名の姓名結合)が揃ったため [D32](ref:d32) の運用に従い採用。形は **`concat: { parts: [<コンビネータ...>], separator: "" }`**(parts の各要素は任意のコンビネータ、separator 既定は空)。**文字列テンプレート式(`"{a}({b})"`)は不採用**(式言語への滑り坂)。M7 で回避のため resume に足した冗長な氏名フィールドは撤去する。

### D46 class の実効セマンティクス統一 {#01KXJR7K8EPXBEFCBZ0EXAK64C alias=d46}

[id=01KXJR7K8EPXBEFCBZ0EXAK64D]
論点は、class の実効セマンティクス統一である。

[id=01KXJR7K8EPXBEFCBZ0EXAK64E, revises=d23]
**実効 class = 自身+祖先(contains 上流)の和集合**を全消費者(render --hide / context --class / view の class フィルタ)で保証する。複数ブロックにまたがる note はコンテナ(セクション・リスト・引用)に class を1回書けばよい([D23](ref:d23) 継承の明文化)。執筆ガイドに指針を追記し、実データの note 連打(1段落ごとに `[class=note]` を繰り返す形)はコンテナ形式へリライトする。

## M8(設計文書の自己 SML 化)設計決定(2026-07-15 対話にて確定) {#01KXJR7K8EPXBEFCBZ0EXAK64F alias=m8-self-sml}

### D47 自己適用の方針 {#01KXJR7K8EPXBEFCBZ0EXAK64G alias=d47}

[id=01KXJR7K8EPXBEFCBZ0EXAK64H]
論点は、自己適用の方針である。

[id=01KXJR7K8EPXBEFCBZ0EXAK64J, depends-on=d37]
本仕様書 §1 の裁定群([D1](ref:d1)〜[D46](ref:d46)・[P1](ref:p1)〜[P4](ref:p4))を SML 化する(`docs/spec-sml/`)。狙い: **意味エッジ(depends-on / refers-to)の実戦データ**(履歴書は論証密度が低くエッジがほぼ未使用だった)、仕様書という第二ジャンルでの摩擦収集、設計対話の `context` 武装(「[D23](ref:d23) に依存する裁定は?」を機械に聞ける)、そして将来のグラフ UI に映す実データ。**正典は当面 md のまま**(SML 版は派生実験。二重化の移行判断は摩擦レポート後に別途)。エッジは [D37](ref:d37) の確信原則(本文が明示的に依拠を語る箇所のみ depends-on、単なる言及は inline ref)。1裁定 = 1 Section(alias=d1〜d46/p1〜p4)、マイルストーン節が contains 階層。

[id=01KXJR7K8EPXBEFCBZ0EXAK64K]
**批准(2026-07-15、修正2点付き)**:

### D35 view-def v1 の批准 {#01KXJR7K8EPXBEFCBZ0EXAK64M alias=d35}

[id=01KXJR7K8EPXBEFCBZ0EXAK64N]
論点は、view-def v1 の批准である。

[id=01KXJR7K8EPXBEFCBZ0EXAK64P, depends-on=d32]
起草文法を以下の修正の上で批准: (1) **糖衣構文** — 裸文字列 `alias.キー` を record フィールド抽出の略記とする(完全形へ機械的に脱糖。定義の8割を占める共通ケースの可読性が v1 の本丸のため)。(2) **`rename` を `pick` に改名**(実態は値の抽出でありリネームではない)。`extend-path` は強力だが非自明なため文書の説明強化で許容。**`template`/`concat`(複数値の糊付け)コンビネータは見送り**(§10 保留に登録 — [D32](ref:d32) の「追加は裁定を経る」運用の初適用。実需が出た時点で再裁定)。

## グラフ UI G1 + rel 語彙(2026-07-15 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9AX alias=g1-graph-ui}

[id=01KYP2K28MRTEH7ZWSQN2CG9AY, alias=g1-graph-ui-intro]
ゴールの再確認(ユーザー): **AI も人も読めるドキュメント構造とリッチコンテンツの共存**の上に、**Obsidian の替わり**(ローカル vault・グラフを見ながら文書を読み、文書からまたグラフへ戻る往復体験・編集)を作る。差別化: Obsidian のグラフは無型リンク網、Strata は**型付きエッジ+型付きコンテンツ**。

### D48 rel `revises` 追加 {#01KYP2K28MRTEH7ZWSQN2CG9AZ alias=d48}

[id=01KYP2K28MRTEH7ZWSQN2CG9B0, alias=d48-topic]
論点は、rel `revises` 追加である。

[id=01KYP2K28MRTEH7ZWSQN2CG9B1, alias=d48-decision]
M8 の境界事例8件(追認・明文化・再裁定・実装裁定)への裁定。**`revises` を1個だけ追加**(A revises B = A は B を改定・追認・精密化・明文化する。方向は新→旧)。`ratifies` 等への細分・ユーザー定義 rel 機構は不採用(rel は全消費者が意味を理解すべき語彙なので最小に保つ — [D32](ref:d32) と同思想)。属性行キー `revises=`、build エッジ、context 表示、UI 描画に波及。描画方針は supports 等と同じ(紙面には出さない意味エッジ)。

### D49 グラフ UI フェーズの定義 {#01KYP2K28MRTEH7ZWSQN2CG9B2 alias=d49}

[id=01KYP2K28MRTEH7ZWSQN2CG9B3, alias=d49-topic]
論点は、グラフ UI フェーズの定義である。

[id=01KYP2K28MRTEH7ZWSQN2CG9B4, alias=d49-decision]
「グラフ UI」の正体は **Obsidian 代替の閲覧半分を、後で Tauri に載る形で先に作る**フェーズと再定義。**G1(今回)**: UI コア=**2ペインの往復**(グラフペイン ⇄ 文書ペイン、選択同期、型付きエッジを辿る)。読み専用。配布は静的エクスポート **`strata site`**(自己完結・サーバ不要)。**G2(次)**: Tauri シェル+編集 — バックエンドが Rust なので strata クレート群をプロセス内で直接呼べる(保存→fmt→再build→グラフ即時更新)。静的サイトは G2 以降も公開・共有形態として存続。v0 スコープ: decisions・履歴書の両ワークスペースが映る/エッジ辿り(revises 込み)/class 表示トグル/alias・ULID ジャンプ。検索・編集・力学レイアウトは以降。

### D50 UI 技術スタック {#01KYP2K28MRTEH7ZWSQN2CG9B5 alias=d50}

[id=01KYP2K28MRTEH7ZWSQN2CG9B6, alias=d50-topic]
論点は、UI 技術スタックである。

[id=01KYP2K28MRTEH7ZWSQN2CG9B7, alias=d50-decision]
**shadcn/ui(React + Tailwind + Vite)**(ユーザー裁定「shadcn でいいんじゃないか説」)。**動的 SPA**(ユーザー裁定: Astro 等の静的ページ生成ではなく、graph JSON をクライアントで読み込み描画・同期・辿りを行う動的アプリ。「静的」は配布形態=サーバ不要のファイル群、の意)。Tauri フロントエンドの定番構成で G2 に直結。**自己完結を維持**(CDN・外部フォント依存なし)。リポジトリに `ui/` を新設(Node ワークスペース。Rust クレートとは分離)。`strata site` はビルド済み UI 資産+graph JSON を出力ディレクトリに合成する。

### D51 グラフペインの LOD(G1.5) {#01KYP2K28MRTEH7ZWSQN2CG9B8 alias=d51}

[id=01KYP2K28MRTEH7ZWSQN2CG9B9, alias=d51-topic]
論点は、グラフペインの LOD(G1.5)である。

[id=01KYP2K28MRTEH7ZWSQN2CG9BA, alias=d51-decision]
G1 目視評価(ユーザー: 「関係グラフは良いが**小さい。長い文書だと読めない**」)への裁定。全ブロック常時描画をやめ、**詳細度の二層**にする: (1) **ローカルグラフ(既定)** — 選択ノード中心に意味エッジ近傍 1〜2 ホップ+contains 親パスのみを常に読めるサイズで描く(Obsidian のローカルグラフ相当、`context --node` のビジュアル版)。(2) **俯瞰(トグル)** — セクション粒度に集約し、ブロック間エッジはセクション間に束ねて本数を太さで表現。現行の全展開アウトラインはオプションに降格。あわせてユーザーの感触「**Zettelkasten 的な関係が作れそう**」(型付きリンク+安定アドレス+term 合流が既にカード間リンクの機構)を受け、**Zettelkasten ドッグフーディング**(notes ワークスペースでの実運用)を G2 の要件出しとして開始する。

## notes ドッグフーディング裁定(2026-07-16 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9BB alias=notes-dogfooding}

[id=01KYP2K28MRTEH7ZWSQN2CG9BC, alias=notes-dogfooding-intro]
Zettelkasten 検証(~/dev/strata-notes)で発見した摩擦2件への裁定。

### D52 リスト継続行(lazy continuation) {#01KYP2K28MRTEH7ZWSQN2CG9BD alias=d52}

[id=01KYP2K28MRTEH7ZWSQN2CG9BE, alias=d52-topic]
論点は、リスト継続行(lazy continuation)である。

[id=01KYP2K28MRTEH7ZWSQN2CG9BF, alias=d52-decision, depends-on=[d39, d40]]
**CommonMark 準拠で継続行を項目本文に併合する**(M6 監査②の取りこぼし。折り返し付きリストは素の MD の日常表現なので、[D39](ref:d39) により診断で弾く選択肢は無い)。「項目=段落1つ」の制約は維持 — その段落が複数ソース行にまたがれるようになるだけ。fmt の `{#id}` タグは項目の**最終行末尾**(Setext・[D40](ref:d40) と同型)。空行は従来どおりブロック境界。

### D53 `doc:` スキーム {#01KYP2K28MRTEH7ZWSQN2CG9BG alias=d53}

[id=01KYP2K28MRTEH7ZWSQN2CG9BH, alias=d53-topic]
論点は、`doc:` スキームである。

[id=01KYP2K28MRTEH7ZWSQN2CG9BJ, alias=d53-decision]
文書そのものを指す**新参照スキーム `doc:<文書alias>`**(Document ノード直指し)。`ref:<文書alias>` の裸解決に文書フォールバックを足す案は不採用 — ブロック alias との衝突時に暗黙の優先順位が生まれ [D42](ref:d42) の明快さを崩すため。既存スキーム族(`ref:`/`table:`/…)と同じ作法の明示的な1語彙。単一ファイル build では自文書 alias のみ解決可、他文書は CrossDocRef 同様の `--workspace` 案内。notes の「H1 に alias=top」規約はこれで不要になる(撤去)。

## G2(エディタ)設計決定(2026-07-16 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9BK alias=g2-editor}

### D54 エディタの分離(疎結合境界) {#01KYP2K28MRTEH7ZWSQN2CG9BM alias=d54}

[id=01KYP2K28MRTEH7ZWSQN2CG9BN, alias=d54-topic]
論点は、エディタの分離(疎結合境界)である。

[id=01KYP2K28MRTEH7ZWSQN2CG9BP, alias=d54-decision]
**エディタは別リポジトリ**(ユーザー裁定: 「密結合すると、代替フォーマットの意味なくなる」)。strata リポジトリ=フォーマット+ツールチェーン(クレート・CLI・リファレンスビューア ui/+site)で、エディタ無しで全機能が完結し続ける。エディタ(strata-editor、Tauri)は**公開境界のみ**に依存: Rust=クレートの path/git 依存、データ=graph JSON スキーマ、フロント=ui/ ビューアの外部参照(v0 は file 依存/submodule、将来パッケージ化)。Obsidian と Markdown の距離感。

### D55 編集ループ {#01KYP2K28MRTEH7ZWSQN2CG9BQ alias=d55}

[id=01KYP2K28MRTEH7ZWSQN2CG9BR, alias=d55-topic]
論点は、編集ループである。

[id=01KYP2K28MRTEH7ZWSQN2CG9BS, alias=d55-decision, depends-on=d42]
T1: **内蔵エディタ(CodeMirror 6)**+ファイルウォッチ併存。T2: 保存 → fmt を**インメモリ実行しエディタバッファへパッチ適用**(ID がその場で生える。fmt が直接ファイルを書いてバッファと衝突させない)→ ディスク書き込み → **デバウンス付きバックグラウンド workspace build** → グラフ即時更新。build 失敗時は **last-good グラフを表示し続け**、診断パネル(行ジャンプ付き)に出す。編集中の即時フィードバックは単一ファイル parse レベル(build 不要)の二段構え。**毎回フル build は思想的に義務ではない**([D42](ref:d42): index は派生・使い捨てで、再構築のタイミングは消費者の自由。大規模化したら §10 のインクリメンタル index を裁定 — API 境界は文書単位の再パースを最初から許す形に)。T3: vault=strata.toml を開く・最近使った vault・新規カード雛形・daily ショートカット。T5(v0 スコープ): 開く/読む(G1 UI)/編集/T2 ループ/新規カード/daily。検索・バックリンクパネル・コマンドパレットは v1。

## 検索とコマンドパレット(2026-07-20 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9BT alias=search-palette}

### D56 検索 {#01KYP2K28MRTEH7ZWSQN2CG9BV alias=d56}

[id=01KYP2K28MRTEH7ZWSQN2CG9BW, alias=d56-topic]
論点は、検索である。

[id=01KYP2K28MRTEH7ZWSQN2CG9BX, alias=d56-decision]
v0 の種類: ①クイックスイッチャー(文書・見出し・alias)②全文検索(ブロック単位ヒット+スニペット)③構造述語の最小形(`class:`/`term:`/`alias:` プレフィックス — 型付きグラフゆえ述語が本物)。**実装は strata 側のライブラリ+CLI `strata search`**(ユーザー裁定「search は CLI 側」)— エディタは Rust 直呼び([D54](ref:d54))、AI エージェントは grep の代わりに使える第二の読み取り接点(執筆ガイドに追記)。**index は v0 インメモリ**(現規模で ms オーダー。「使ってから裁定」の文化どおり永続化は実測で遅くなってから — ただし検索 API の境界は index 実装差し替え可能に切る)。ヒット単位=ブロック、表示は人間可読ラベル+スニペット(G1.7 ポリシー)、CJK は部分文字列一致。**グラフ探索クエリ(パス・パターン)と vector/埋め込み検索の扱いは将来検討**(§10 へ登録、ユーザー指示)。

### D57 コマンドパレット先行 {#01KYP2K28MRTEH7ZWSQN2CG9BY alias=d57}

[id=01KYP2K28MRTEH7ZWSQN2CG9BZ, alias=d57-topic]
論点は、コマンドパレット先行である。

[id=01KYP2K28MRTEH7ZWSQN2CG9C0, alias=d57-decision]
エディタは**コマンドパレットを検索より先に実装**(ユーザー裁定「コマンドもそこで育てないといけない」)。パレット=コマンド登録基盤(ID・表示名・キーバインド・実行)であり、既存操作(新規カード・今日のノート・保存・モード切替・class トグル・vault 切替等)をコマンド化して収容。クイックスイッチャーと全文検索は**パレットのモード**として実装(Obsidian と同型)。以後の新機能は原則コマンドとして追加し、パレットが機能の索引を兼ねる。

## ビュー定義の追補(2026-07-20 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9C1 alias=view-def-addendum}

### D58 rows の class フィルタ {#01KYP2K28MRTEH7ZWSQN2CG9C2 alias=d58}

[id=01KYP2K28MRTEH7ZWSQN2CG9C3, alias=d58-topic]
論点は、rows の class フィルタである。

[id=01KYP2K28MRTEH7ZWSQN2CG9C4, alias=d58-decision, depends-on=[d46, d32]]
ユーザーフィードバック(初のフィードバック運用: 受け皿は GitHub Issues、§10 は裁定後の保留正典): `rows: contains` に `include-only-class` / `exclude-class` が無く、join では可能な「note クラスのセクション全体を submit から除外」が行反復でできない。**join と同一の語彙・同一のセマンティクス([D46](ref:d46) 実効 class)で `rows` に両フィルタを追加**する(コンビネータ新設ではなく既存コンビネータの適用範囲の一貫化 — [D32](ref:d32) の運用に整合)。`rows: table` モードへの適用は行キーが class を持たないため対象外(裁量確認の上、対象外なら文書に明記)。

## Obsidian 互換(2026-07-29 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9C5 alias=obsidian-compat}

[id=01KYP2K28MRTEH7ZWSQN2CG9C6, alias=obsidian-compat-intro]
実 Obsidian vault(`kurochanBrainPrivate`、304ファイル)を Strata 統合するドッグフーディングで発見した致命的互換性ギャップへの裁定。

### D59 `[[wikilink]]` 対応 {#01KYP2K28MRTEH7ZWSQN2CG9C7 alias=d59}

[id=01KYP2K28MRTEH7ZWSQN2CG9C8, alias=d59-topic]
論点は、`[[wikilink]]` 対応である。

[id=01KYP2K28MRTEH7ZWSQN2CG9C9, alias=d59-decision, depends-on=d2]
Obsidian の `[[ページ名]]`(304ファイル中100で使用、無診断でリテラルテキスト化=「静かに壊れる」)を新規対応する。**解決方式はワークスペース内の文書タイトル/ファイル名との一致による自動解決**(ユーザー裁定)。alias 付与不要で既存ファイルを無修正のまま移行できる利点を優先。同名衝突(複数文書が同じタイトル)は曖昧エラー(diagnostic、全か無か)。パイプ形式 `[[target|表示テキスト]]` も対応。埋め込み `![[...]]` は別途(§10 保留、当面プレーンなリンクとして扱うか要検討)。canonical グラフでは他の Ref 同様、解決後は対象ノードへの通常の Ref(rel=refers-to)になる — wikilink というソース記法の情報は保持しない([D2](ref:d2) の ID 無視同型と同じ「表層記法は正規化される」原則)。

## Obsidian frontmatter 統合(2026-07-29 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9CA alias=obsidian-frontmatter}

[id=01KYP2K28MRTEH7ZWSQN2CG9CB, alias=obsidian-frontmatter-intro]
[D59](ref:d59) 実装で実 vault の frontmatter 実態を調査した結果、64/38/38/38/38/38/14+ 件など少数の一貫したスキーマ(Web クリッパー・タグ・Excalidraw プラグイン設定)に整理できることが判明。frontmatter は「分類」だけでなく「記述的事実」「実行条件(プラグイン設定)」の3種が混在する汎用プロパティ系である、という認識のもとに裁定。

### D60 未知 frontmatter キーの重大度 {#01KYP2K28MRTEH7ZWSQN2CG9CC alias=d60}

[id=01KYP2K28MRTEH7ZWSQN2CG9CD, alias=d60-topic]
論点は、未知 frontmatter キーの重大度である。

[id=01KYP2K28MRTEH7ZWSQN2CG9CE, alias=d60-decision, depends-on=d17]
`UnknownFrontmatterKey` を **Error → Warning** に格下げる(`UnknownAttrKey`([D17](ref:d17))と同型)。fmt/build は未知キーを持つファイルを拒否しなくなる(全か無かの対象外)。バイトは温存、frontmatter 自体は書き換えない([D6](ref:d6) の純挿入・冪等は不変)。実質的な安全網 — ここより先の [D61](ref:d61)/[D62](ref:d62) が無くても既存ファイルの取り込みが塞がれない。

### D61 frontmatter の意味付けは宣言的マッピングで — 固定変換はしない {#01KYP2K28MRTEH7ZWSQN2CG9CF alias=d61}

[id=01KYP2K28MRTEH7ZWSQN2CG9CG, alias=d61-topic]
論点は、frontmatter の意味付けは宣言的マッピングで行い、固定変換はしないことである。

[id=01KYP2K28MRTEH7ZWSQN2CG9CH, alias=d61-decision, depends-on=[d46, d28]]
frontmatter キーは**分類**(tags/status 等 → [D46](ref:d46) `class` 機構)・**記述的事実**(author/source/created 等 → [D28](ref:d28) `record`)・**実行条件**(プラグイン設定等 → 意味を持たせず無視)の3種に分かれるが、**Strata がどれかに決め打ちしない**。vault ごとに**宣言的マッピング仕様**(YAML、view-def-v1.md と同じ思想 — 固定コンビネータ `as: class` / `as: record` / `as: ignore` のみ、スクリプトなし)を書き、それに従って変換する。「ユーザーの自由度」はこのマッピングファイルの中に住む。frontmatter に4番目のキー `class:`([D46](ref:d46) 実効 class、文書全体に効く)を追加 — マッピングの `as: class` の着地点。マッピング仕様の文法は実装時に起草。

### D62 インポート能力の置き場所 — まずスキル、ツールは将来 {#01KYP2K28MRTEH7ZWSQN2CG9CJ alias=d62}

[id=01KYP2K28MRTEH7ZWSQN2CG9CK, alias=d62-topic]
論点は、インポート能力の置き場所であり、まずスキル、ツールは将来とすることである。

[id=01KYP2K28MRTEH7ZWSQN2CG9CM, alias=d62-decision, depends-on=d37]
Obsidian → SML の変換は**まず AI エージェント駆動のスキル**として提供する(新規 Rust バイナリを今は作らない)。既存の [D37](ref:d37)(AI が SML を書き fmt/build が検証)と M5-C(AI がバインディング案を出し人間が批准)をそのまま流用: AI が vault の frontmatter 実態を調査 → [D61](ref:d61) のマッピング仕様を提案 → 人間が批准 → その仕様に従って AI が各ファイルを SML ドラフトに変換 → fmt/build で検証。決定的ツールへの昇格は、view-def([D30](ref:d30)〜[D35](ref:d35))と同じく実運用で型が安定してから判断する(「使ってから裁定」)。

## パイロット移行(00_Daily 34件・Clippings 47件)の裁定(2026-07-29 対話にて確定) {#01KYP2K28MRTEH7ZWSQN2CG9CN alias=obsidian-pilot-migration}

[id=01KYP2K28MRTEH7ZWSQN2CG9CP, alias=obsidian-pilot-migration-intro]
[D62](ref:d62) のスキルを実運用で試したドッグフーディング(kurochanBrainPrivate から非破壊コピーした2バッチ、実 vault は無変更)で発見した不具合・仕様の穴への裁定。

### D63 マッピング仕様に `split` コンビネータを追加 {#01KYP2K28MRTEH7ZWSQN2CG9CQ alias=d63}

[id=01KYP2K28MRTEH7ZWSQN2CG9CR, alias=d63-topic]
論点は、マッピング仕様に `split` コンビネータを追加することである。

[id=01KYP2K28MRTEH7ZWSQN2CG9CS, alias=d63-decision, depends-on=d32]
Web クリッパー系プラグインは複数タグを1リスト項目に空白区切りで詰める(`tags: ["clippings RAG tools"]`)ことがあり、これを機械的に `as: class` へ流すと `class:` の字句規則(`[A-Za-z0-9_-]+`)に反し `BadClass` で build が落ちる(実データで確認)。**マッピング仕様(`docs/obsidian-import-mapping.md`)に `split: whitespace` オプションを追加**し、`as: class` と組み合わせて「1項目を空白分割してから複数 class として展開」を宣言できるようにする。固定コンビネータの追加は [D32](ref:d32) の運用どおり実需(今回)を経ての裁定。

[id=01KYP2K28MRTEH7ZWSQN2CG9CT, alias=obsidian-pilot-migration-completion]
**完了の定義**: 上記に加え、パイロットで発見した CLI バグ(`strata fmt` の一時ファイル名が元ファイル名に依存し NAME_MAX 超過で失敗する — 実データで4/47件が該当)を修正し、`docs/obsidian-import-skill.md`・`docs/obsidian-import-mapping.md`・`docs/sml-agent-guide.md` をパイロットのフィードバック(CRLF・wikilink未解決時の扱い・優先順位の明記・frontmatter4キー目の追随・H1合成指針・値のクォート規則差)で更新する。
