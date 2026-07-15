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

### 1.1 fmt 実装後の精密化(2026-07-13 裁定)

M2(fmt)実装で顕在化した曖昧点への裁定。いずれも D1〜D6 の変更ではなく精密化:

| # | 論点 | 裁定 |
|---|---|---|
| P1 | 意味保存の「ID無視同型」の定義 | **属性行の `alias` エントリも ID 情報として無視する**(§8.1)。行末 `{#...}` タグを alias ごと無視するのと対称 |
| P2 | 属性行 `id` の値の字句 | **裸トークン(ULID またはラベル)のみ**。引用符・リストは診断 `BadIdValue`、ラベルの字句違反は `BadKeyCharset`(§3.2) |
| P3 | `{#label alias=x}`(非ULID + alias) | **パーサが診断 `AliasWithoutUlid` で弾く**。alias を書けるのは ULID の id だけ。ドラフトでは `{#label}` とだけ書く(§3.1) |
| P4 | CLI `fmt --check` の表示 | パッチを**文書順(行番号昇順)**で「行:列: delete N byte(s), insert "…"」形式で表示。読み取り失敗は exit 1、診断種別は enum 名表記 |

### 1.2 M3(build)設計決定(2026-07-14 対話にて確定)

| # | 論点 | 決定 |
|---|---|---|
| D7 | build の配置 | 新クレート **`strata-build`**(strata-sml + strata-core + tex2math に依存)。strata-sml の依存最小方針は不変 |
| D8 | strata-core 拡張 | `Rel::RefersTo` 追加 / `Inline::Ref` に `coord: Option<CellCoord>` 追加(serde は省略可能)/ `CellValue::Quantity { v, unit }` 追加(§9 の3課題を解決) |
| D9 | `term:` の行き先 | **用語名から決定的に導出した安定 ID** で Term ノードを build が自動生成(同名 → 同 ID、毎 build 安定、ファイル横断でも同一)。defines 記法(保留)が入ったら定義ブロックと接続する |
| D10 | コードフェンスの ID | **行型として開始行末尾の `{#id}` を解禁**(§2 の分類に追加)。fmt の注入対象に含める。§10 の保留を解消 |
| D11 | リスト全体の ID | **リスト前置属性行の `id=` を解禁**(プローズ同様の前置 `[id=...]`)。項目は従来どおり行内 `{#id}`。M1 の「常に置き場所違反」を改定 |
| D12 | 文書ルート | **フロントマター**(§2.1)を導入し、canonical に **Document ノード**を新設して全トップレベルブロックを contains する。フロントマターは必須ではない(無ければ Document ノードも無しのフォレスト)が、**fmt が無ければ生成する**ため fmt 済みファイルは常に持つ |
| D13 | build のエラー方針 | **全か無か**(fmt と同じ、§8.2)。パースエラー・未解決エイリアス・エイリアス重複定義・ULID 未付与ブロックをすべて収集して全件報告し、グラフを出力しない |

### 1.2b M3 実装後の裁定(2026-07-14 対話にて確定)

M3(build)実装で顕在化した曖昧点への裁定。D14 は D8/D13 の実装済み拡張の追認、
D15〜D17 は精密化。

| # | 論点 | 裁定 |
|---|---|---|
| D14 | build エラーの拡張の追認 | strata-build の `BuildError::RefTypeMismatch { span, msg }`(参照スキーム `table:`/`fig:`/`math:`/`cell:` と対象ノード型の不一致。黙認しない)と `BuildError::Invariant(Violation)`(build 後の `invariants::validate` 違反の包み。ソース位置なし)を正式承認。あわせて tex2math に `\hat`(`UnderOver` の over-only 形)を追加したことを M3 実装の追認として記録する |
| D15 | Term ID v0 の Unicode 正規化 | Term 安定 ID の導出式(D9)を「名前を **NFC 正規化**してから `strata:term:v0:{name}` を SHA-256」に改めて v0 凍結。視覚的に同じ用語名が NFC/NFD の違いで別 ID になる問題を解消する |
| D16 | Table.caption / Chart.depicts | strata-core に後方互換で追加: `Table.caption: Option<Vec<Inline>>`、`Chart.depicts: BTreeMap<String,String>`(`ImageFigure.depicts` と同形・同じキー畳み規則)。build がフェンス内属性行(`[caption=...]`・`[depicts=...]`/`[depicts.*]`)から写す |
| D17 | Diag に severity(Error/Warning)を導入 | 診断に `Error`/`Warning` の重大度を持たせる。新設2種別(`DuplicateFrontmatterKey`・`UnknownAttrKey`)のみ `Warning`、既存種別はすべて `Error`。「全か無か」(§8.2)は **`Error` にのみ適用**: `Warning` だけの入力は fmt/build とも成功し、`Warning` を結果と併せて返す |

### 1.3 M4(render)設計決定(2026-07-14 対話にて確定)

| # | 論点 | 裁定 |
|---|---|---|
| D18 | M4 のスコープと CLI | サブコマンド `render` を新設: `strata-cli render <file.sml> [-o out.typ]`。内部で build → render を直結し、中間 JSON は介さない(JSON 入力のパイプライン分割は将来の拡張として保留)。exit code は build と同じ 0/1/2、Warning は stderr 表示のうえ exit 0 |
| D19 | レンダラの主従 | **Typst を一次レンダラ**とし、M4 以降の新語彙の描画対応は strata-typst にのみ実装する。strata-html は現状機能のまま**凍結**(クレートは残置するが CLI からの導線は持たない)。Web 表示は将来の「グラフ UI」フェーズで再設計する — 意味グラフ文書の閲覧体験(ノード単位のナビゲーション)は Typst の HTML export では制御できないため、その時点で自前 HTML を再起動する。`render` の `--format` は当面 Typst のみ |
| D20 | vault の削除 | crates/strata-vault・`vault/`(YAML データ)・CLI の旧 YAML フロー(`run_legacy`)を**削除**する。履歴書は Strata 本線に持ち込まない(ユーザー裁定)。必要になれば git 履歴から採取できる |
| D21 | Document の描画 | `Document.title` は文書メタ(`#set document(title: ...)`)にのみ使い、本文には出さない(本文見出しは Section の H1 に任せ二重表示しない)。title 無しは最初の H1 のプレーンテキスト、それも無ければ入力ファイル名にフォールバック。`root: None`(フロントマター無し)の render はエラー(「strata fmt を先に実行してください」案内、exit 2) |
| D22 | 参照の描画 | 全ブロックノードに Typst ラベル `<ULID>` を付与する。`::table`/`::math`/`::figure` は Typst の `figure` 要素に包んで**自動番号付け**(「表 1」「図 1」「式 1」、`set text(lang: "ja")`)を得る。`Ref` は表示 `text` があれば `#link(<label>)[text]`、無ければ `@ULID`(自動番号参照)。番号を持たない対象(段落・リスト・コード)への text 無し参照は `#link` + 短い代替表記とする(詳細はハンドオフの裁量)。`Term` は `text` → Term ノードの `name` の順で表示(用語集・defines は引き続き保留) |

### 1.4 ドッグフーディング後の裁定(2026-07-14 対話にて確定)

M4 完了後、実履歴書(2文書・30プロジェクト規模)を SML 化するドッグフーディングで
顕在化した摩擦点への裁定。背景となる定性評価: 「誰に見せるか」の条件分岐は文書では
なくビューの仕事(PowerPoint の発表者ビューと同型)/ 文書と組版を一体化すると難しく
なる / 人はテンプレート先行で「埋める」のに現状はテキスト先行で「引き抜く」ため
頭の動きが合っていない(→ ビュー/テンプレート層、§10 保留)。

| # | 論点 | 裁定 |
|---|---|---|
| D23 | 出し分けの層分離 | 文書には「この情報が**何であるか**」の意味分類だけを書く: ブロック属性 **`class=<タグ,...>`**(タグの字句は key と同じ、複数可)。「誰に見せるか」(audience)は文書に書かず**ビュー側**で決める。v0 のビュー指定は CLI `render --hide <class>`(複数指定可)で、該当 class を持つブロックを**サブツリーごと**非描画にする。build は class 非依存で全ノードをグラフに格納する(ビューが変わっても真実は一つ)。非描画ノードへの `Ref` が残った場合はビュー側ポリシーとして Warning + リンクを剥がしてプレーンテキスト化。インラインの出し分け(見出し内の実名等)は保留 — 文書構造をねじ曲げる回避策(専用の小段落等)は採らない |
| D24 | ネストリスト | リスト項目の**子リスト**を正式対応する(パーサ・fmt の ID 注入・build の contains・render)。「項目=段落1つ」の制約は維持(項目内複数ブロックは引き続き保留 §10)。従来の無警告誤パース(インデントされた子項目が `- ` 混じりの別段落に化ける)は根絶し、リストとして解釈できないインデント行には診断を出す |
| D25 | 組版の改善方針 | **SML に組版ヒント(列幅・改ページ等)は導入しない**(意味と体裁の分離)。レンダラ既定の改善のみで対処する: 表 figure の breakable 化、列幅戦略の見直し(長文列の潰れ・1文字縦書き化の防止)、表前の空白ページ解消。合格基準は「30行ネスト表が読める・重ならない・空白ページなし」。体裁のカスタマイズは将来の「ビュー/テンプレート層」で文書外(ビュー定義)に持つ |

### 1.5 M4y(構造化データ語彙)設計決定(2026-07-15 対話にて確定)

ビュー v0(バインディング=コード)の棚卸しで定量化された語彙欠落への裁定
(年月分解の正規表現 26 件・「キー: 値」分解 13 件・メタ行の DRY 違反 30 件)。

**設計原理(正本と最終構造)**: 正本(SML)は「最も分解された事実」を記録する。
最終構造(テンプレートのスロット)はビューの都合であり、形の変換はバインディングの
仕事。ただしビュー群が要求する粒度は正本の分解粒度の**下限**として逆算してよい
(粒度は逆流させてよいが、形は逆流させない)。合成テキストから構造は復元できないが、
分解された事実から任意の粗い形は導出できる、という非対称性が根拠。

| # | 論点 | 裁定 |
|---|---|---|
| D26 | alias のグラフ出力 | build は解決済みエイリアスを graph JSON に出力する(ビューのアドレス規約の柱。v0 では caption 部分一致で代用する羽目になった)。表現形式は裁量 |
| D27 | 子 List ノードの ID | SML 上に子リスト全体の ID を書く場所が無いため、**親リスト項目の ULID+位置から決定的に導出**する(D9 の Term 安定 ID と同型)。毎 build 安定にし、ID 安定の不変条件との矛盾を解消 |
| D28 | `::record` フェンス | key-value ブロックを新設: `::record {#id alias=...}`、本体は「キー: 値」の行の列。**キーは自由テキスト(日本語可)** — 表の座標キー(D5、ASCII 限定)とは別物でパス構文に入らない。値は表セルと同じ型付きパース(Number/Quantity/Date/Period/Text/Ref)。core に `NodePayload::Record`。標準ビュー(render)では2列表相当で描画。ネスト record は保留 |
| D29 | 日付・期間のセル値型 | `CellValue::Date { y, m, d? }` / `Period { from, to? }`(to 無し=「現在」)。**既定の受理書式は ISO(`YYYY-MM-DD` / `YYYY-MM`)のみ**とし、書式スニッフィングはしない。フェンス属性 `date-format=` で当該ブロックの追加入力書式を明示宣言できる(例: `date-format="YYYY年M月"`)— 明示宣言でパースを容易にする(ユーザー裁定)。期間は「A 〜 B」「A 〜 現在」(`〜`/`~` 両可)。表示の日本語化・年齢等の導出値の計算はビュー側の仕事 |

### 1.6 ビュー v1(宣言的ビュー定義)設計決定(2026-07-15 対話にて確定)

バインディングを「コード」から「宣言的定義+決定的実行器」へ。dry-run 検証と
将来の LLM 提案(レビュー可能なバインディング案)の土台。

| # | 論点 | 裁定 |
|---|---|---|
| D30 | 実行形態 | 新サブコマンド **`strata view <file.sml> --view <def.yaml> [-o outdir]`**(新クレート strata-view)。入力は SML(内部 build、render と同じ流儀)。出力は**テンプレート消費用のデータファイル群**(YAML/JSON)。Typst を知らないデータ層に留める |
| D31 | セレクタ語彙 | **alias / class / セル座標 / 型+contains パス**を一級(v0 実測の頑健度順)。見出しテキスト一致は警告付きエスケープハッチ。**正規表現は入れない** — 表現できない時は SML 側の構造化が正解というシグナルとする |
| D32 | 変換語彙 | **固定コンビネータの小セットのみ**: rename/pick、rows(表→配列)、join(木→文字列)、date(書式)、age/as-of(導出)、literal、class フィルタ。**汎用式・スクリプト埋め込みは導入しない**(XSLT の轍)。不足時は「コンビネータを1個足す裁定」か「SML 側を直す」の二択 |
| D33 | テンプレート・マニフェスト | テンプレート側に必要スロットの宣言(YAML・手書きで起こす)。**`strata view --check`** の dry-run で未充足スロット/未使用ノードを診断(fmt/build と同じ表示流儀)。テンプレ内データハードコードの検出器を兼ねる |
| D34 | 出し分けプロファイル | ビュー定義内に profile(submit/check 等)を宣言し `--profile` で選択。1 定義から複数出力。D23 の class がフィルタ条件の語彙 |

ビュー定義ファイルの文法は実装時に `docs/view-def-v1.md` として起草し、次の対話で
批准する(v1 の品質基準: 定義ファイルが「読んで分かる」こと — LLM 提案・人間承認の
レビュー対象になれる可読性)。

### 1.7 M5-A(AI が読む: コンテキストビュー)設計決定(2026-07-15 対話にて確定)

M5(AI 連携)を「A: AI が読む / B: AI が書く / C: AI が設計する(ビュー定義提案)」に
分解し、A から着手(B の前提であり、C は既存材料の手順設計で足りるため)。

| # | 論点 | 裁定 |
|---|---|---|
| D36 | `strata context` | AI 向けコンテキストビューの専用サブコマンド `strata context <file.sml> [-o out.md]`。出力は **ULID 付き Markdown+意味エッジ一覧**(LLM が読みやすく、回答の根拠を ULID/alias で引用させられる形)。グラフの chunk 分割は固定長ではなく**意味の単位(ノード/サブツリー)**で行う。スコープ3形態: (1) 無指定=全文書、(2) `--node <alias|ULID>`(複数可)+ `--hops N`(既定1)— contains サブツリーが chunk 本体、意味エッジ(supports/depends-on/refers-to/term)を N ホップ辿った近傍を文脈として付加、(3) `--class <tag>` — 意味分類での横断抽出(例: note だけ集める)。エッジ種の選別パラメータは保留(D32 と同じ「使ってから裁定」運用)。exit code・Warning の流儀は他コマンドと同一 |

**批准(2026-07-15、修正2点付き)**:

| # | 論点 | 裁定 |
|---|---|---|
| D35 | view-def v1 の批准 | 起草文法を以下の修正の上で批准: (1) **糖衣構文** — 裸文字列 `alias.キー` を record フィールド抽出の略記とする(完全形へ機械的に脱糖。定義の8割を占める共通ケースの可読性が v1 の本丸のため)。(2) **`rename` を `pick` に改名**(実態は値の抽出でありリネームではない)。`extend-path` は強力だが非自明なため文書の説明強化で許容。**`template`/`concat`(複数値の糊付け)コンビネータは見送り**(§10 保留に登録 — D32 の「追加は裁定を経る」運用の初適用。実需が出た時点で再裁定) |

---

## 2. 文書モデルとブロック分類

SML ファイル1つ = ドキュメント1つ。ファイル内はブロックの列であり、各ブロックが
canonical の Node に対応する。見出しのレベルが `contains` のネストを作る
(`##` セクションは直前の `#` セクションの子)。

ブロックは ID 記法の観点から2クラスに分かれる(D2):

| クラス | ブロック | ID の書き方 |
|---|---|---|
| **行型** | 見出し、リスト項目、フェンスマーカー(`::table` 等)、コードフェンス開始行(D10) | 行内の `{#...}` タグ |
| **プローズ** | 段落、**リスト全体**(D11) | 直前の属性行 `[id=...]` |

リストは二層構造になる: リスト**全体**の ID は前置属性行(プローズ扱い)、各**項目**の
ID は行内 `{#...}`(行型)。canonical では List ノードが項目 Para を contains する。

### 2.1 フロントマター(D12)

ファイル先頭(1バイト目)が `---` 単独行で始まる場合、次の `---` 単独行までを
フロントマターとして解釈する。中身は YAML 風の `key: value` 行(自前の最小実装。
ネスト・リスト・引用符エスケープは持たない):

```markdown
---
id: 01J2T8Z0000000000000000000
title: 機械学習モデルの評価レポート
---
```

- キーは v0 では `id`(ULID、任意)と `title`(自由文字列、任意)のみ。未知キーは
  診断(「出たら足す」方針。`UnknownFrontmatterKey`、`Error`)
- 同一キー(`id` / `title`)が複数行で宣言された場合、挙動は**後勝ち**(最後の宣言が
  採用される)のまま変えず、診断 `DuplicateFrontmatterKey`(`Warning`、D17)を出す
- **`id` の値は ULID のみ**。人間ラベルは書けない(診断)。フロントマターは通常
  fmt が生成するものであり、ラベル → ULID+alias の置換系をここに持ち込まない
- fmt: フロントマターが**無ければ** `id` 入りで先頭に生成(挿入のみ)。**あって
  `id` が無ければ** `id: <ULID>` 行を `---` の直後に挿入
- build: フロントマターがあれば **Document ノード**(canonical)を作り、全トップ
  レベルブロック(H1 含む)を文書順の `contains` で繋ぐ。無ければ Document
  ノードは作らない(フォレスト)。ファイル → 文書の対応管理は将来の vault 層の仕事
- 閉じ `---` が無い場合は診断(全か無かにより fmt/build は動かない)

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
- **alias を併記できるのは ULID の id だけ**。`{#label alias=x}`(非 ULID + alias)は
  診断 `AliasWithoutUlid`(P3)。ドラフトでは `{#label}` とだけ書き、fmt がラベルを
  alias へ昇格させる
- エイリアスのスコープは**ファイル内**。ファイル横断の参照は ULID で行う
  (グローバルエイリアス表は保留 §10)

### 3.2 属性行での ID(プローズブロック用)

```markdown
[id=01J2T8Z6..., supports=eval-table]
予測精度はモデルの実用性を担保するために最も重要な指標であり、…
```

- ドラフトでは `[id=my-label]` と書いてよい。fmt が `[id=01J2..., alias=my-label]` に置換
- ID の無い段落には、fmt が直前に `[id=<新規ULID>]` 行を**挿入**する
- `id` の値は**裸トークン(ULID またはラベル)のみ**(P2)。引用符付き(`[id="..."]`)・
  リスト(`[id=[a, b]]`)は診断 `BadIdValue`。ラベルの字句は key と同じ
  `[A-Za-z0-9_-]+` で、違反は `BadKeyCharset`

### 3.3 リスト項目(行型)

```markdown
- 評価は2軸で行う {#01J2T8X0...}
- 再現性は別レポートで扱う {#01J2T8X1...}
```

fmt は ID の無い項目の行末に ` {#ULID}` を追記する。項目にも安定 ID を与えるのは
不変条件1(ID 安定)のため。§2.4 の需要駆動昇格は「anchor ノード化」の機構であり、
「ID を持つか」とは独立の話として切り分ける。

リスト**全体**の ID は前置属性行 `[id=...]` で与える(D11、2026-07-14 改定)。
canonical の List ノード(ordered 情報を持ち項目を contains する)の ID がこれに
対応する。fmt は ID の無いリストの直前に `[id=<新規ULID>]` 行を挿入する。

### 3.4 canonical との関係

エイリアスは**層1の道具**である。`strata build` はエイリアス→ULID の解決表を構築して
参照を解決するが、canonical グラフ(Node/Edge)には ULID しか入らない。

---

## 4. 属性行と意味エッジ(D1)

属性行 `[key=value, ...]` は**直後のブロック**に束縛される。どのブロッククラスにも
前置できるが、`id` を書けるのはプローズブロックの属性行だけ(行型は `{#}` を使う。重複はエラー)。

`id` の置き場所の規則を精密化する(v0.1 パーサ実装で確定):

- 行型ブロック(見出し・フェンス・コードフェンス)の前置属性行に `id=` を書くとエラー。
  自身の `{#...}` タグとの併記なら「重複」、単独でも「置き場所違反」として診断される
- **リスト**は前置属性行の `id=` を**許す**(D11、2026-07-14 改定。M1 実装の
  「常に置き場所違反」を廃止): リスト全体を指す単一行が存在しないため、プローズ同様に
  前置属性行で ID を与える。項目の ID は従来どおり行内 `{#id}`
- **フェンス内属性行**(`::table` 直後の `[caption=...]` 等の位置)にも `id` は書けない。
  フェンスの ID はマーカー行の `{#...}` のみ

### 4.1 意味エッジの宣言

エッジ関係カタログ(strata-spec §4)の rel 名をそのままキーに使う:

```markdown
[id=01J2..., supports=eval-table]
特に、Dataset-A における Opt-v2 のレイテンシは 12 ms であり、…

[supports=[claim-1, claim-2], cites=izenman-2008]
複数の主張を同時に支持する段落。
```

- 使えるキー: `supports` / `depends-on` / `cites`(+将来追加される rel)。加えて
  `id` / `alias`(§3.2、fmt が注入)と `class`(D23。エッジではなく意味分類。値は
  key 字句のタグ、単一または `[a, b]` リスト)も同じ属性行に共存できる
- 値: 単一ターゲット、または `[a, b]` のリスト
- ターゲット: ULID / エイリアス / `term:<用語名>`
- build 時に `Edge(このブロック → ターゲット, rel)` が materialise される
- 上記6キー(`supports` / `depends-on` / `cites` / `id` / `alias` / `class`)以外のキーは、
  エッジが張られないタイポの可能性として診断 `UnknownAttrKey`(`Warning`、D17)を
  出す。挙動そのものは従来どおり無視のまま(build はエッジを張らない)

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
- **外部リンク**(`[表示](https://...)` 等の URL)は v0 に存在しない。`://` を含む
  dest はスキーム未定義として**診断なしでプレーンテキスト扱い**になる(著者のリンクが
  静かに不活性化するので注意。`url:` スキームの導入は保留 §10)

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
| `45 ms`(数値 + 空白 + 単位トークン) | 数量: 値 45 / 単位 "ms" として構造化(canonical 表現は §9-3)。単位トークンの字句は `[A-Za-zµ%°]` で始まり `[A-Za-z0-9/^·%°-]*` が続く1トークン(v0.1 で確定) |
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

fmt が行う変更は4種類だけ(D10〜D12 で対象を拡張。2026-07-14 改定):

1. 行型ブロック(見出し・リスト項目・フェンスマーカー・コードフェンス開始行)の
   行末に ` {#ULID}` を**追記**
2. ID の無いプローズブロック(段落・リスト全体)の直前に `[id=ULID]` 行を**挿入**
3. 非 ULID ラベルの `{#label}` / `[id=label]` を `{#ULID alias=label}` 形式に**置換**
   (`{...}` / `[...]` の内側のみ)
4. フロントマターが無ければ先頭に `id` 入りで**生成**、あって `id` が無ければ
   `id: <ULID>` 行を**挿入**(§2.1。どちらも純粋な挿入)

契約(テストで固定する受け入れ条件):

- **冪等性**: `fmt(fmt(x)) == fmt(x)`
- **挿入のみ**: fmt 前後の diff は上記3種の追記・挿入・囲み内置換のみ
- **意味保存**: `build(fmt(draft))` ≅ `build(formatted)`(同型のグラフ)。
  build 未実装の間は「パース結果の **ID無視同型**」で代用する。**ID情報**の定義(P1)=
  行末 `{#...}` タグ全体(alias 含む)+ 属性行の `id`・`alias` エントリ + 全スパン値。
  これらを消した正規化構造が一致すること
- **原子性**: 一時ファイルに書いて rename。途中状態をディスクに残さない

### 8.2 エラー方針

**全か無か**: 1箇所でもパースエラーがあれば、fmt はファイルに一切触れず
エラー位置(スパン)を報告して終了する。「半分だけ処理された状態」を作らない。
build も同じ方針(D13)。

**Error のみが対象(D17、2026-07-14)**: 診断には severity(`Error`/`Warning`)があり
(§1.2b)、「全か無か」は **`Error` にのみ適用**する。`Warning` 診断(`DuplicateFrontmatterKey`・
`UnknownAttrKey`)だけの入力は fmt/build とも**成功**し、その `Warning` を処理結果と
併せて呼び出し側に返す。CLI(`fmt`/`build` 両方)は `Warning` を
「`行:列: warning: 種別: メッセージ`」形式で stderr に出しつつ exit 0 で終了する
(`Error` が1件でもあれば従来どおりファイルには一切触れず exit 2、全診断を報告)。

### 8.3 rowan / CST への乗り換えトリガ

次のいずれかが要件になった時点で再評価する。それまではパッチ方式を維持:

1. fmt に**整形機能**(表の桁揃え・インデント正規化)を持たせたくなった
2. **エディタ統合(LSP)** — 打鍵中の壊れた文書への増分再パースが必要になった
3. オフセット管理のバグが繰り返し出て負債化した

---

## 9. strata-core への波及(必要な拡張)

本仕様の実装に必要な strata-core 拡張。**方針は D8/D12 で確定済み(2026-07-14)、
実装は M3**:

1. **`Rel::RefersTo` の追加** — インラインのナビゲーション参照(§5.2)が materialise
   する弱参照。現行カタログには存在しない(現実装は `DependsOn` に誤って畳んでいる)
2. **セル参照の座標保持** — `Inline::Ref` に `coord: Option<CellCoord>` を追加
   (serde では省略可能に)。`CellCoord` は `{row_path, col_path}`(strata-core 側に
   定義。strata-sml とは依存しないため型は別)
3. **数量(数値+単位)の canonical 表現** — `CellValue::Quantity { v, unit }` を追加。
   SML の型付きパース結果(D4)と 1:1 対応。prose と共有したい値だけを明示的に
   `Value` ノード + `CellValue::Ref` にする道は従来どおり残す
4. **エイリアス解決表** — build が保持する層1の道具。canonical グラフには入れない
5. **`Document` ノードの追加**(D12) — フロントマターに対応する文書ルート。
   `title: Option<String>` を持ち、トップレベルブロックを contains する
6. **Term ノードの安定 ID 導出**(D9、D15) — build が用語名から決定的に導出した
   128bit を ID として Term ノードを自動生成する。導出式(v0 凍結、D15、
   2026-07-14): 名前を **NFC 正規化**してから `Sha256("strata:term:v0:{name}")` の
   先頭16バイトを読む
7. **参照の表示テキスト保持** — `Inline::Ref` / `Inline::Term` は現在 `to` しか持たず、
   `[レイテンシ](cell:...)` の表示テキストが落ちる。ロスレス原則の帰結として
   `text: String` を追加する(2026-07-14、D8 の一部として確定)
8. **`Table.caption` / `Chart.depicts`**(D16、2026-07-14) — `Table.caption:
   Option<Vec<Inline>>`(§6.1 の `[caption=...]`)、`Chart.depicts:
   BTreeMap<String,String>`(§6.3 の `[depicts=...]`/`[depicts.<key>=...]`。
   `ImageFigure.depicts` と同形・同じキー畳み規則: 裸の `depicts` は
   `"description"` キー、`depicts.<key>` はその `<key>` をキーにする)を追加する。
   両方とも後方互換フィールド(`default` + `skip_serializing_if`)

---

## 10. 凍結 vs 保留

**凍結(本書の契約):** D1〜D36・P1〜P4 の全決定、§2.1・§3〜§8 の記法と fmt/build 契約。

**解消済み(2026-07-14):** コードフェンスの ID 記法(→ D10)、文書ルート/
フロントマター(→ D12)、リスト全体の ID(→ D11)、ネストリスト(→ D24。
項目内複数ブロックは保留継続)、出し分けのブロック単位(→ D23)。

**保留(後で決める):**

- 文書レベルのコメント構文(フェンス内 `#` のみ確定。`<!-- -->` が有力候補)
- 外部リンク用スキーム(`url:` 等)の導入と、無診断フォールバックの是非(§5.2)
- member label 構文の最終形(`- key "label"` は初版案)
- `defines` エッジの SML 表現(用語定義ブロックの記法)— 当面は D9 の Term 自動生成で
  参照側だけ成立させる。定義ブロック記法が入った時点で `defines` エッジを接続
- ファイル横断のエイリアス(グローバルエイリアス表)
- リスト項目の中の複数ブロック(項目=段落1つ、を当面の制約とする。ネストは D24 で解禁)
- インラインテキスト中のリテラル `[` `$` 等のエスケープ
- フロントマターのキー追加(authors・date 等。v0 は id / title のみ)
- インラインの出し分け(見出し・段落内の一部スパンだけを class 分類する記法。D23 の
  ブロック単位では実名の 【実際: 〇〇】 のようなインライン情報を扱えない)
- **値のトランスクルージョン**(`cell:` 等の参照先の**値**を本文・セルに埋め込み表示
  する記法。現状の参照はリンクのみで、同じ事実の二重記述を防げない — v0 で
  メタ行 30 件の DRY 違反として定量化。当面は「表を正として重複側を削除」で回避)
- **エンティティ**(会社・人物など同一実体の表記ゆれを束ねるノード。`term:` は
  用語専用。v0 では正規化+前方一致のあいまい一致で代用した)
- ビュー定義の `template`/`concat` コンビネータ(複数値の糊付け、例:
  `"{details}({level})"`。D35 で見送り — 実需が出た時点で D32 の運用に従い再裁定)
- **ビュー/テンプレート層**(ドッグフーディング定性評価より): JIS 履歴書のような
  既存テンプレートへの pull 型流し込み(テンプレートが先にあり、グラフから埋める)、
  リスト/拡張表現をビュー側で表に組み替える変換、ビュー定義(体裁カスタム・class
  フィルタのプロファイル)を文書外ファイルとして持つ仕組み。M5(AI ビュー)と地続き。
  2026-07-15 の壁打ちでの骨格合意:
  - **バインディング層**の新設 — グラフの意味構造とテンプレートのスロットスキーマの
    対応表(ER 図的な2スキーマ間マッピング)。CSS(流れの装飾)では足りず、
    XSLT / headless CMS(クエリ+コンポーネント)の系譜。ミスマッチは
    アドレス・粒度・多重度の3種
  - バインディングは**データを持たない純粋な対応表**。テンプレートファースト=
    バインディングを逆向きに実行して SML 雛形(記入フォーム)を生成、
    ドキュメントファースト=既存両スキーマの対応を設計。両フローは同じ成果物を
    逆方向から作る。データは常に文書に着地(第二の真実の源を作らない)
  - 段階案: v0=バインディングをコードで(プロジェクトローカルなスクリプト、
    graph JSON → テンプレート入力データ、テンプレート無改変。セレクタと整形の
    語彙の棚卸しが目的)→ v1=宣言的ビュー定義に蒸留(`strata view` 的コマンド)→
    v2=バインディング=スキーマとして逆向き利用(記入検証・雛形生成)
  - 設計を軽くする3点: テンプレート・マニフェスト(必要スロットの宣言をデータで持つ)/
    グラフ側のアドレス規約(alias・class・セル座標で引きやすい書き方)/
    バインディングの検証ループ(dry-run で未充足スロット・未使用情報を診断)
  - LLM の使い所: バインディング定義(案)の提案。宣言的定義(v1)が前提 —
    提案(LLM)と実行(決定的な実行器)の分離で監査可能にする。M5 の入口

---

## 付録A. 文法スケッチ(EBNF 風・実装時に厳密化)

```ebnf
document    = frontmatter? block* ;
frontmatter = "---" NL ( fm-key ":" SP fm-value NL )* "---" NL ;   (* ファイル先頭のみ。D12 *)
fm-key      = "id" | "title" ;                       (* 未知キーは診断。id の値は ULID のみ *)
block       = attr-line? ( heading | fence | code-fence | list-item+ | paragraph ) ;
code-fence  = "```" lang? ( SP id-tag )? NL ... "```" NL ;          (* 開始行末尾に id 可。D10 *)

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
