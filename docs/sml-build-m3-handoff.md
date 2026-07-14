# Milestone 3 実装ハンドオフ — `strata build`(SML → canonical グラフ)

本書は Milestone 3 の設計確定事項と、別セッション/サブエージェント向けの自己完結な
作業指示。設計決定は `sml-spec.md` §1.2 の D7〜D13(2026-07-14 対話にて確定)で
凍結済みであり、本書はその実装への落とし込みを定義する。

## 前提(Milestone 2 完了時点の状態)

- `strata-sml`: スパン付きパーサ(M1)+ fmt(M2)。契約テスト4本・全テスト green
- `strata-cli`: `fmt` サブコマンド実装済み。既存 YAML フロー温存
- `strata-core`: 層2スキーマ(Node/Edge/Rel/Table/MathNode/Figure、invariants)
- `tex2math`: TeX → 独自 `MathNode`(`parse_normalized`)。strata-vault は
  serde_json 経由で core の `MathNode` へ橋渡ししている(後述 WP-B4 では踏襲しない)
- ゴールデンペア `docs/sml_example_draft.sml` / `sml_example_formatted.sml`(M3 で改版する)

## 必読ドキュメント(この順で読むこと)

1. `AGENTS.md` — ルール: **git commit/push はユーザー指示なしに絶対しない**
2. `docs/sml-spec.md` — 正典。特に §1.2(D7〜D13)・§2/§2.1(フロントマター)・
   §3・§8・§9(core 拡張)
3. `docs/sml-parser-design.md` §4(AST)と `crates/strata-sml/src/`(ast.rs / block.rs / fmt.rs)
4. `crates/strata-core/src/lib.rs` 全体(拡張対象)
5. 本書の残り全部
6. ゴールデンペア2ファイル

## スコープ境界(やらないこと)

- HTML/Typst レンダラへの接続(render_to_html 等の変更・呼び出し)はやらない。
  M3 の納品は**グラフ構築と JSON 出力まで**
- strata-vault / strata-html / strata-typst に触れない(strata-core の拡張は
  後方互換 — serde は省略可能フィールド/新バリアント追加のみ — で行い、
  既存3クレートのビルドとテストを壊さない)
- `defines` エッジ・用語定義ブロック記法はやらない(保留。Term は D9 の自動生成のみ)
- ファイル横断(複数ファイル入力・グローバルエイリアス)はやらない
- 文書レベルコメント・`url:` スキーム等の保留事項(sml-spec §10)はやらない

## 設計確定事項(本書で凍結)

### D-B1: クレート構成(D7)

新クレート `crates/strata-build`。依存: `strata-sml`, `strata-core`, `tex2math`,
`ulid`, `serde`/`serde_json`, `sha2`(Term ID 導出用)。公開 API:

```rust
pub struct BuildOutput {
    pub graph: strata_core::Graph,
    /// フロントマター(Document ノード)があればその ID。レンダラ接続時のルート。
    pub root: Option<strata_core::NodeId>,
}

/// 全か無か(D13): パース診断・解決エラーが1件でもあれば Err。
pub fn build(src: &str) -> Result<BuildOutput, Vec<BuildError>>;

pub enum BuildError {
    /// パーサの診断(strata_sml::Diag)をそのまま包む。
    Parse(strata_sml::Diag),
    /// ULID 未付与ブロック(fmt 未実行)。「strata fmt を先に実行してください」と案内。
    MissingId { span: strata_sml::Span },
    /// 参照ターゲット(エイリアス)がファイル内に存在しない。
    UnresolvedAlias { alias: String, span: strata_sml::Span },
    /// 同名エイリアスが複数ブロックに定義されている。
    DuplicateAlias { alias: String, spans: Vec<strata_sml::Span> },
    /// ::figure の属性不足・不正(kind 欠落、chart の data-ref/mark/encode 不足等)。
    BadFigure { span: strata_sml::Span, msg: String },
    /// 数式が tex2math でパースできない(UnknownCommand 等)。
    Math { span: strata_sml::Span, msg: String },
}
```

エラーは**全件収集**して返す(fmt 同様、最初の1件で止まらない)。

### D-B2: strata-core 拡張(D8 + §9-7)

`crates/strata-core/src/lib.rs` に以下を追加。**すべて後方互換**(既存 JSON が
読めること。新フィールドは `#[serde(default, skip_serializing_if = ...)]`):

```rust
// Rel に追加
Rel::RefersTo,          // "refers-to"。ナビゲーション弱参照(§5.2)

// Inline::Ref を拡張
Inline::Ref {
    to: NodeId,
    rel: Rel,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    coord: Option<CellCoord>,           // cell: 参照の座標。他は None
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,                       // 表示テキスト(§9-7)
},
Inline::Term {
    to: NodeId,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    text: String,                       // 表示テキスト
},

pub struct CellCoord { pub row_path: Vec<String>, pub col_path: Vec<String> }
// (strata-sml の同名型とは別物。両クレートは依存しないため重複定義でよい)

// CellValue に追加
CellValue::Quantity { v: f64, unit: String },

// NodePayload に追加(D12)
NodePayload::Document(Document),
pub struct Document {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
}
```

既存の `Inline::Ref { to, rel }` 利用箇所(strata-vault 等)は struct バリアントの
フィールド追加でコンパイルエラーになるため、`..Default` は使えない。既存3クレートの
構築箇所は `coord: None, text: String::new()` を補う**最小修正のみ許可**(挙動不変)。

### D-B3: SML 記法拡張のパーサ対応(D10〜D12)

`strata-sml` の変更:

1. **フロントマター(D12、sml-spec §2.1)**
   - ファイル先頭(オフセット0)が `---\n` の場合のみ。次の `---` 単独行まで
   - `SmlDocument` に `frontmatter: Option<Frontmatter>` を追加:
     `Frontmatter { span: Span, id: Option<(RefTarget, Span)>, title: Option<String>, close_span: Span }`
     (フィールド構成は裁量。fmt が「id 行の挿入位置 = 開き `---` 行の直後」を
     計算できる情報を持つこと)
   - `key: value` 行のみ(コロン後の空白は任意)。キーは `id` / `title`。
     未知キー → 新 DiagKind `UnknownFrontmatterKey`。閉じ `---` 欠落 → 新 DiagKind
     `UnclosedFrontmatter`。`id` の値が ULID でない → 既存 `BadIdValue`
     (フロントマターにラベル/alias の置換系は持ち込まない。sml-spec §2.1)
   - スパン被覆不変条件のテストにフロントマターを組み込む
2. **コードフェンス ID(D10)**
   - `BlockKind::CodeFence` に `id_tag: Option<IdTag>` を追加。開始行
     (` ```lang {#id} `)の行末タグを見出しと同じ規則で抽出
   - `check_id_placement`: CodeFence を**行型**に変更(前置属性行の `id=` は
     DuplicateId / IdNotAllowedHere)。`check_id_value` の CodeFence 分岐は削除
3. **リスト前置属性行の id=(D11)**
   - `check_id_placement` の List 分岐を削除(プローズ扱い = `id=` 許可)。
     `check_id_value` の対象に List を追加
   - 項目の `{#id}` とリストの `[id=...]` は**別エンティティなので併記可**
     (DuplicateId にしない)ことをテストで固定

### D-B4: fmt 拡張と fixture 改版(D10〜D12、sml-spec §8.1 の4種)

`strata-sml/src/fmt.rs` の変更:

- **フロントマター生成/注入**(編集種別4): 無ければオフセット0に
  `---\nid: <ULID>\n---\n\n` を挿入(空行1つを含むこの形で凍結)。あって `id` が
  無ければ開き `---` 行の直後に `id: <ULID>\n` を挿入。`id` があれば何もしない
- **コードフェンス**: ID タグが無ければ開始行の行末に ` {#ULID}` を追記
- **リスト**: 前置属性行が無ければリスト先頭行の直前に `[id=ULID]\n` を挿入。
  属性行があって `id` が無ければ `[` の直後に `id=ULID, ` を挿入(段落と同じ規則)
- **ID 発行順**: フロントマターが常に最初。以降は従来どおり文書順
- 既存の fmt 単体テストは出力にフロントマターが付くようになるため全面的に
  期待値を更新する(検証の弱体化はしないこと)

**fixture 改版**: `docs/sml_example_formatted.sml` を新仕様の fmt 出力に更新する
(draft は変更しない)。決定的生成器の ULID は **18個**になる
(`01J2T8Z0000000000000000000` 〜 `01J2T8ZH000000000000000000`、末尾16文字目が
Crockford Base32 の `0-9,A-H`)。文書順: ①フロントマター ②H1 ③H2導入 ④導入段落
⑤リスト全体 ⑥⑦リスト項目×2 ⑧supports段落 ⑨H2評価結果 ⑩段落 ⑪::table ⑫H2分析
⑬⑭⑮段落×3 ⑯::math ⑰段落 ⑱::figure。契約テスト(fmt_contract.rs)の
ゴールデン完全一致・同型比較もこれに追従させる(同型比較では frontmatter の `id` は
ID情報として無視、`title` 等は比較対象)。

### D-B5: build の変換規則

ブロック → ノードの対応(ID はすべて SML 側の ULID をそのまま `NodeId` に):

| SML | canonical |
|---|---|
| フロントマター | `Document { title }`。**全トップレベルブロックを文書順の contains(ord 付き)で繋ぐ**。フロントマターが無ければ Document ノードなし(フォレスト、`root: None`) |
| 見出し | `Section { heading }`。**見出しレベルでネスト**: `##` は直前の `#` の子、レベル飛び(`#`→`###`)は直近の浅い方の子。見出しに続くブロック(次の同位以上の見出しまで)はその Section の contains 子 |
| 段落 | `Para { inline }` |
| リスト | `List { ordered }`(ID はリストの `[id=]`)。各項目は `Para` ノード(ID は項目の `{#id}`)にして List が ord 付き contains |
| ::table | `Table`。`DimNode/MemberNode` → `Dim/Member`(label は `Some(vec![Inline::Text])` に包む)。`CellRaw` → `CellValue`: Number→Number / Quantity→Quantity / Text→Text / Empty→Empty / Ref→Ref(エイリアス解決して NodeId) |
| ::math | `MathBlock`。本体 TeX を `tex2math::parse_normalized` でパースし、**構造的な変換関数**で core の `MathNode` へ写す(vault の serde_json 経由ハックは踏襲しない。全バリアント 1:1 の match) |
| ::figure | `Figure::Chart` / `Figure::Image`。`kind` 属性で分岐。chart: `data_ref`(解決)・`mark`・`encode-x/y/color`。image: `src`・`alt`・`depicts.*`(`depicts.subject` 形式のキーは `.` 以降を BTreeMap のキーに、裸の `depicts` は `"".into()` ではなく `"description"` キーに畳む)。caption は `Some(vec![Inline::Text])`。必須属性欠落は `BadFigure` |
| コードフェンス | `Code { lang, src }`(ID は開始行の `{#id}`) |

インライン変換(`SmlInline` → `Inline`):

- `Text(span)` → `Text { s }`、`Emph` → `Emph`(再帰)、`MathTex(span)` →
  `Math { tree }`(tex2math)
- `Ref { scheme, target, coord, text }` → `Inline::Ref { to: 解決済みNodeId,
  rel: Rel::RefersTo, coord, text }`。scheme(table:/fig:/math:)による対象ノード型の
  検証を行う(不一致は `UnresolvedAlias` ではなく `BadFigure` 相当の新エラーでも
  よい — 裁量。ただし黙認はしない)。`cell:` は表ノードを指し coord を保持
- `TermRef { name_or_id, text }` → `Inline::Term { to: TermのNodeId, text }`

エッジの materialise:

- `contains`: Document→トップレベル、Section ネスト、List→項目(すべて ord 付き)
- 属性行の意味エッジ(§4.1): `supports` / `depends-on` / `cites` →
  `Edge(ブロック → 解決済みターゲット, rel)`。値がリストなら各要素に1本。
  `term:<名前>` ターゲットは Term ノードへ
- インライン参照: `Ref` → `Edge(囲むブロック → to, RefersTo)`、
  `Term` → `Edge(囲むブロック → to, TermRef)`

エイリアス解決(2パス):

1. 第1パス: 全ブロック(+リスト項目)を走査して「ULID 登録」と
   「alias → ULID 表」を構築。alias の重複 → `DuplicateAlias`(全件)。
   ULID 未付与(`RefTarget::Label` の id、または ID を持たないブロック)→ `MissingId`
2. 第2パス: ノード構築とエッジ materialise。参照ターゲットの解決は
   「ULID ならそのまま / ラベルなら alias 表 → 無ければ `UnresolvedAlias`」

Term の安定 ID 導出(D9。この式で凍結):

```rust
// sha2 クレート。名前は書かれたままの UTF-8(正規化なし — 既知の制約として記録)
let hash = Sha256::digest(format!("strata:term:v0:{name}"));
let id = NodeId(Ulid(u128::from_be_bytes(hash[..16].try_into().unwrap())));
```

同名の term 参照は同一 Term ノードに集約する(ノードは1回だけ insert)。

build 後に `strata_core::invariants::validate` を必ず実行し、違反があれば
`BuildError` に変換して返す(build のバグ検出網。正しい実装では出ないはず)。

### D-B6: CLI(strata-cli)

`fmt` と同じ流儀でサブコマンド `build` を追加:

```
strata-cli build <file.sml>              # グラフ JSON を stdout へ
strata-cli build <file.sml> -o out.json  # ファイルへ(原子的書き込み)
```

- 出力: `serde_json::to_string_pretty(&BuildOutput)`(graph と root を含む)
- exit code: 成功 0 / 読み書き失敗 1 / BuildError あり 2(全件を
  「行:列: 種別: メッセージ」で stderr。Parse は Diag と同形式、他は span から変換)
- 既存の YAML フロー・`fmt` サブコマンドの挙動を変えない

## 作業パッケージ分割

依存: `WP-B1(パーサ拡張) → WP-B2(fmt 拡張+fixture 改版)`、
`WP-B3(core 拡張)` は WP-B1/B2 と並列可、`{WP-B2, WP-B3} → WP-B4(build 本体) → WP-B5(CLI)`

### WP-B1: パーサ拡張(strata-sml)

- D-B3 の3点(フロントマター/コードフェンスID/リストid)+ 新 DiagKind 2種
- 変更可: strata-sml の src(fmt.rs 以外)と tests。既存テストの更新は
  「仕様変更(D10/D11)に対応する箇所」のみ
- 単体テスト: フロントマター(正常2形・未知キー・非ULID id・閉じ欠落・
  ファイル途中の `---` は段落扱い)、コードフェンス `{#id}` 4形、リスト
  `[id=]` + 項目タグ併記、スパン被覆

### WP-B2: fmt 拡張 + fixture 改版(strata-sml)

- D-B4 の全部。fmt_core.rs / fmt_contract.rs / golden_isomorphism.rs /
  `docs/sml_example_formatted.sml` の更新を含む
- 契約4本+追加プロパティが新仕様で green であること(検証は弱めない。
  「全ブロックが ULID を持つ」はコードフェンス除外を**廃止**し全ブロック対象に)

### WP-B3: strata-core 拡張

- D-B2 の全部。既存3クレート(vault/html/typst)の最小修正(フィールド補完)込み
- `cargo test --workspace` が通ること(強い制約: 既存挙動を変えない)

### WP-B4: build 本体(strata-build 新クレート)

- D-B1 / D-B5 の全部
- テスト: ゴールデン(改版後 formatted → グラフ。ノード数/エッジ数/root と、
  代表ノード・エッジの内容を固定)、エラー全種(MissingId は draft fixture で発火)、
  term 集約と安定 ID(同名2参照→1ノード、導出式の期待値をハードコードで固定)、
  数量セル、cell: 参照の coord 保持、見出しネスト、invariants 通過

### WP-B5: CLI(strata-cli)

- D-B6。統合テスト: formatted fixture の build 成功と JSON の再パース、
  draft(ULID 無し)で exit 2 と MissingId 案内、`-o` の原子的書き込み、
  既存フロー退行なし

## 完了の定義

- 全 WP のテストを含め `cargo test --workspace` 通過
- `nix develop -c cargo clippy --workspace --all-targets` で新規・変更コードの警告ゼロ
  (既存の strata-html/typst/vault と examples の警告は対象外)
- 既存 CLI(YAML フロー・fmt)が退行していない
- **コミットはしない**。変更ファイル一覧・テスト消化状況・仕様の曖昧点
  (勝手に解釈せず報告)をまとめて終了

## 既知の注意点

- fixture 改版は WP-B2 に集約する(WP-B1 の段階ではゴールデンテストが旧 fixture の
  まま green であること — 記法拡張は後方互換なので旧 fixture もパースできる)
- Term ID の導出は名前の Unicode 正規化をしない(`予測精度` と NFD 分解形は別 ID)。
  既知の制約として受け入れ、問題になったら NFC 正規化を v1 として導入
- core の `Inline::Ref`/`Term` のフィールド追加は vault/html/typst のコンパイルを
  壊す(struct バリアント)。フィールド補完以上の変更をしないこと
- `BuildOutput` の JSON に `root` を含める都合上、単純な `Graph` のシリアライズとは
  形が違う。CLI テストは `BuildOutput` でラウンドトリップすること
