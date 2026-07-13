# SML パーサ設計書 (Milestone 1)

対象: `docs/sml-spec.md` v0.1 に準拠した SML パーサの実装設計。
スコープは **パース(テキスト → スパン付き SML-AST)まで**。fmt(Milestone 2)と
build(Milestone 3)は本パーサの出力を消費する別フェーズだが、両者の要件
(スパン記録・エラー方針)が本設計の制約として先取りされている。

---

## 1. 方針決定: 手書きのライン指向パーサ

Plans.md §2.1 の選択肢(pulldown-cmark 拡張 / pest / カスタム)に対する結論。

**手書き(ライン指向ブロックスキャナ + 再帰下降)を採る。**

| 選択肢 | 不採用/採用の理由 |
|---|---|
| pulldown-cmark 拡張 | `into_offset_iter` でスパンは取れるが、SML の拡張(属性行・`::` フェンス・行末 `{#id}`)がイベント列への前後処理ハックになる。CommonMark の全機能(setext 見出し・遅延継続行など)は fmt の決定性にとってむしろ**害**であり、載った瞬間にサブセット化の戦いが始まる |
| pest (PEG) | `::table` 本体のようなインデント文法は PEG が苦手。エラー位置の質とエラー収集(複数報告)の制御もしにくい。文法が今後も動く段階では生成器のリグレッションが重い |
| **手書き** | SML は意図的にライン指向(ブロック境界 = 空行、マーカー = 行頭)なので、行ベースのスキャナが最短。スパンの完全な制御、fmt が要求する「全バイトの帰属」を自前で保証できる。リポジトリには tex2math(手書き Pratt)の前例がある |

判断の核: **fmt のスパンパッチ方式(sml-spec D6)は「パーサがスパンを正確に持つこと」に
全体重を掛けている**。スパンが一級市民でないパーサ基盤は、この時点で選択肢から落ちる。

## 2. クレート構成

```
crates/strata-sml/
├── src/
│   ├── lib.rs        # 公開API: parse(), ParseOutput
│   ├── span.rs       # Span, 行/列変換
│   ├── scan.rs       # 層A: ブロックスキャナ(行 → ブロックスパン列)
│   ├── block.rs      # 層B: ブロック内パース(IDタグ・属性行)
│   ├── inline.rs     # 層B: インラインパース(強調・参照・数式スパン)
│   ├── table.rs      # 層B: ::table 本体(次元木・セル・値の型付け)
│   └── error.rs      # Diag(種別+スパン)、エラー収集
└── tests/
    ├── golden.rs     # sml_example_draft/formatted を使うゴールデンテスト
    └── ...
```

**依存方針: strata-core にも tex2math にも依存しない。**

- SML-AST は canonical(strata-core::Graph)とは別物。変換は build(M3)の仕事
- インライン/ブロック数式は **TeX ソース文字列+スパンのまま保持**し、tex2math は
  build 時に呼ぶ(遅延パース)。fmt は数式の中身を見る必要が一切ないため、
  M1/M2 の依存グラフが最小になる

## 3. 二層アーキテクチャ

### 層A: ブロックスキャナ(`scan.rs`)

行単位の1パスで、ファイルを**ブロックスパンの列**に分割する。インラインの中身は見ない。

- 空行 = ブロック区切り
- 行頭パターンでブロック種別を判定: `#`+ → 見出し / `- `・`N. ` → リスト項目 /
  `::` → フェンス開始 / ```` ``` ```` → コードフェンス / `[` で始まり `]` で終わる行 → 属性行 /
  その他 → 段落(連続行を束ねる)
- フェンスは対応する閉じ(`::` 単独行 / ```` ``` ````)まで本体を**不透明なスパン**として飲む
- 属性行は**直後に空行を挟まず**ブロックが続く場合のみ、そのブロックに束縛。
  続かなければ診断(孤立属性行)

**この層だけで fmt が成立する**のが設計の要。fmt が必要とする情報 —
「各ブロックの種別・開始/終了オフセット・IDタグの有無と内側スパン」— は層Aで
すべて確定する。インラインパース(層B)のバグが fmt の安全性に波及しない。

**スパン被覆不変条件**: 層Aの出力ブロック列はオフセット昇順・非重複で、
隙間は空行のみ。`Σ(ブロック+隙間) = ファイル全体` をテストで固定する(§7)。

### 層B: ブロック内パーサ

層Aのスパンを入力に、種別ごとのパーサが中身を解釈する:

- `block.rs`: 行末 `{#id}` / `{#id alias=x}` タグの抽出(内側スパン付き)、
  属性行の `key=value` リスト(`supports=[a,b]` のリスト値含む)
- `inline.rs`: 再帰下降。`**`・`*`・`` ` ``・`$...$`(スパンのみ)・
  `[text](scheme:target)`・`[text](cell:target#path|path)`。未対応構文は
  プレーンテキストにフォールバック(インラインは寛容、ブロック構造は厳格)
- `table.rs`: `@rows:`/`@cols:`/`@cells:` セクション。インデント(2スペース)で
  次元⇄メンバーの交互ネストを構築。セル値の型付きパース(D4)

## 4. データ構造(SML-AST)

```rust
pub struct Span { pub start: usize, pub end: usize }   // バイトオフセット [start, end)

pub struct SmlDocument {
    pub blocks: Vec<SmlBlock>,
    pub src_len: usize,
}

pub struct SmlBlock {
    pub span: Span,                    // 属性行を含むブロック全体
    pub attrs: Option<AttrLine>,       // 前置属性行(あれば)
    pub kind: BlockKind,
}

pub enum BlockKind {
    Heading { level: u8, inline: Vec<SmlInline>, id_tag: Option<IdTag> },
    Paragraph { inline: Vec<SmlInline> },
    List { ordered: bool, items: Vec<ListItem> },      // 項目ごとに id_tag
    Fence(FenceBlock),                                  // ::table / ::math / ::figure
    CodeFence { lang: String, body: Span },
}

pub struct IdTag {
    pub id: RefTarget,                 // Ulid(Ulid) | Label(String)  ← fmt が Label を検出
    pub alias: Option<String>,
    pub inner_span: Span,              // {#...} の内側。fmt の置換対象
}

pub struct AttrLine {
    pub span: Span,
    pub entries: Vec<(String, AttrValue, Span)>,   // id / supports / caption / ...
}

pub enum RefTarget { Ulid(Ulid), Label(String) }   // 解決は build の仕事。ASTは未解決のまま

pub enum SmlInline {
    Text(Span),
    Emph { kind: EmphKind, children: Vec<SmlInline> },
    MathTex(Span),                                  // TeX ソースのまま(遅延パース)
    Ref { scheme: RefScheme, target: RefTarget, coord: Option<CellCoord>, text: Span },
    TermRef { name_or_id: RefTarget, text: Span },
}

pub enum RefScheme { Ref, Table, Fig, Math, Cell }

pub struct FenceBlock {
    pub fence_kind: FenceKind,          // Table | Math | Figure
    pub id_tag: Option<IdTag>,
    pub fence_attrs: Vec<AttrLine>,     // フェンス内属性行(caption 等)
    pub body: FenceBody,
}

pub enum FenceBody {
    Table(TableBody),                   // DimNode 木 + Vec<CellEntry>
    MathTex(Span),
    Figure,                             // 属性行のみで完結(本体なし)
}

pub enum CellRaw {                      // D4 の型付きパース結果
    Number(f64),
    Quantity { v: f64, unit: String },
    Text(String),
    Ref(RefTarget),
    Empty,
}
```

設計上の非対称に注意: **ULID か人間ラベルかは AST が区別する**(fmt が注入対象を
列挙するため)が、**ラベル→ULID の解決は AST では行わない**(エイリアス表の構築は
build の仕事。sml-spec §3.4)。

## 5. サポートする Markdown サブセット(v0 凍結)

CommonMark 全体は載せない。v0 で解釈するのは:

- ATX 見出し(`#`〜`######`)のみ。**setext 見出し(下線式)は非対応**
  (行末 `{#id}` 追記と相性が悪く、fmt の決定性を壊す)
- 段落(空行区切り。遅延継続などの CommonMark 特例なし)
- フラットなリスト(`- ` / `N. `)。項目 = 1行 = 1段落(ネストは保留、sml-spec §10)
- フェンスコード(```` ```lang ````)
- インライン: `**strong**` `*em*` `` `code` `` `$tex$` `[text](scheme:target)`
- 非対応(出たらプレーンテキスト扱い): blockquote、GFM テーブル(`::table` を使う)、
  HTML ブロック、水平線、画像記法(`::figure` を使う)

「Markdown 互換」の正確な意味は **「このサブセットの範囲で CommonMark と同じ見た目に
レンダリングされる」** であり、任意の Markdown ファイルが SML として有効という意味では
ない。この線引きを README 等でも明示する。

## 6. エラー設計

```rust
pub struct Diag { pub kind: DiagKind, pub span: Span, pub msg: String }

pub struct ParseOutput {
    pub doc: SmlDocument,       // エラー箇所は Unknown ブロックとして保持
    pub diags: Vec<Diag>,
}
```

- パーサは**最初のエラーで止まらず収集する**(1回の実行で全エラーを報告)
- 「全か無か」(sml-spec §8.2)の裁定は**呼び出し側**が行う:
  fmt/build は `diags` が非空なら何もしない。将来の LSP は部分 AST を使える
- `DiagKind` は enum で型付け(tex2math の配当の再現): `UnclosedFence`,
  `OrphanAttrLine`, `DuplicateId`(`{#}` と `[id=]` の併記), `BadKeyCharset`(D5違反),
  `BadCellCoord`, `InconsistentIndent`, `UnknownScheme` など

## 7. テスト戦略

1. **ゴールデンペア**: `docs/sml_example_draft.sml` と `sml_example_formatted.sml` を
   `include_str!` で読み、両方がエラーゼロでパースできること。さらに
   **両者の AST が「IDタグ・id属性を無視すれば同型」であること** — これが
   fmt 契約「意味保存」の検証をパーサ側から挟むテストになる
2. **スパン被覆**: 任意の入力に対し、層Aの出力が昇順・非重複・全被覆(隙間は空行のみ)
3. **単体**: 構文ごとの最小ケース(IDタグ4形 × 位置、属性行のリスト値、
   ネスト次元、セル値6型、参照5スキーム、各 DiagKind を1つずつ発火)
4. **フォールバック**: 非対応 Markdown(blockquote 等)がエラーでなく
   プレーンテキストになること

## 8. 受け入れ条件(Milestone 1 完了の定義)

- [ ] `strata-sml` クレートがワークスペースに追加され、`cargo test` 全通過・警告ゼロ
- [ ] ゴールデンペア2ファイルがエラーゼロでパースできる(Plans.md M1 の成果物)
- [ ] draft と formatted の AST が ID 無視で同型
- [ ] スパン被覆不変条件のテストが通る
- [ ] 全 DiagKind に対応する失敗ケーステストが存在する

## 9. 実装順(提案)

1. `span.rs` + `scan.rs`(層A)+ スパン被覆テスト — **fmt の土台はここで完成**
2. `block.rs`(IDタグ・属性行)— ゴールデンペアの構造が取れる状態に
3. `table.rs`(次元木・セル)— Strata の心臓部
4. `inline.rs`(参照・強調)
5. ゴールデン同型テストで締める

## 10. 未決事項(実装中に決める)

- `Span` に行/列のキャッシュを持つか、都度計算か(エラー表示の頻度次第)
- リスト項目の `id_tag` 位置と `—` 等の全角記号を含む行末検出の正確な規則
  (「行末の ` {#...}` 」の「行末」判定: 末尾空白は許すか → 許す方向)
- `@cells` の座標に**存在しない葉パス**を書いた場合をパースエラーとするか
  build エラーとするか → 葉の検証は次元木とセルの突き合わせなので **build**(パーサは字句まで)
