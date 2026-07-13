# Strata Markup Language (SML) 設計ノート (v0.3)

今回のブレインストーミングで合意されたコア原則：
*   **人間が書く層（層1: オーサリング）** と **AIが読み書き・活用するデータ層（層2: canonical / ストア）** を明確に切り離す。
*   IDの書き戻し（インジェクション）や関係性（エッジ）の解決といった複雑な処理は、**すべて機械（コンパイラ・フォーマッタ）が自動で行う**。
*   層2（ストアデータ）はAIが解釈することに特化するため、必ずしも人間が直接読んで快適である必要はない（厳密さとロスレス性を最優先する）。

これに基づき、SMLファイルが「人間による執筆」から「AI向けの厳密なグラフ表現」へと変換され、ソースにIDが逆注入される処理パイプラインを定義します。

---

## 1. パイプライン設計：SML から Graph への変換と ID インジェクション

SMLファイルを処理するプログラムは、**「フォーマッタ（Formatter）」** と **「コンパイラ（Compiler）」** の2つの役割（またはモード）を持ちます。

```
[人間/AIが執筆: ドラフトSML] (ID未付与、ラベル参照)
         │
         ▼
 ┌──────────────┐
 │ フォーマッタ │  <--- 自動でULIDを発行し、SMLファイルに直接書き戻す
 └──────────────┘
         │
         ▼
[保存・管理用SML] (ULID埋め込み済、バージョン管理対象)
         │
         ▼
 ┌──────────────┐
 │ コンパイラ   │
 └──────────────┘
         │
         ▼
[Strata canonical グラフ (層2)] (Node と Edge の海) -> AIが読み込み・推論
```

### 1.1 フォーマッタの挙動（ID書き戻し）
フォーマッタ（`strata fmt`）は、SMLファイルのAST（抽象構文木）を解析し、以下の変換をインプレース（ファイル上書き）で行います。

1.  **ブロックIDの生成**: IDが指定されていないブロック（見出し、段落、表、図など）の前に、新規の `ULID` を生成して `{#ULID}` を挿入します。
2.  **エイリアスリンクの解決**: 人間が書いた `[売上](cell:revenue-table#Revenue|2025.Q1)` のようなラベル指定を、ターゲットのブロックID（ULID）に書き換えます。元のラベルはメタデータ（エイリアス）としてソースに保存されるか、コンパイラがマッピングテーブルを保持します。

### 1.2 コンパイラの挙動（グラフ構築）
コンパイラ（`strata build`）は、IDが注入されたSMLファイルを読み込み、Rustの `Graph` 構造を構築します。

1.  **ノードの生成**: 各ブロックを `Node`（IDとPayload）に変換。段落内のインライン要素もASTにパース。
2.  **エッジの自動抽出（マテリアライズ）**:
    *   SMLの構造（ネスト関係）から `contains` エッジを生成。
    *   インラインの `{term:...}` から `defines` エッジを生成。
    *   リンク記法（`[表示](cell:...)` や `[表示](fig:...)`）から、該当ブロックへの `Ref` や `TermRef` エッジを抽出し、`edges` テーブルに非正規化して格納。

---

## 2. 具体的なコード表現（SMLからRustモデルへのマッピング）

人間が書いたドラフトSMLが、フォーマットを経て、どのように `strata-core` の Rust 構造体にマップされるかの具体例です。

### 2.1 変換前のドラフト（人間/AIが記述）
```markdown
::table {#revenue-table}
[caption="財務データ"]
@rows:
  - metric: [Revenue, Cost]
@cols:
  - year: [2025, 2026]
@cells:
  Revenue | 2025 : 100
  Cost    | 2025 : 60
::

この結果は、[2025年の売上](cell:revenue-table#Revenue|2025) を見ればわかる。
```

### 2.2 フォーマット後のSML（ファイルに上書き保存される状態）
```markdown
::table {#01J2T8V...}
[caption="財務データ"]
@rows:
  - metric: [Revenue, Cost]
@cols:
  - year: [2025, 2026]
@cells:
  Revenue | 2025 : 100
  Cost    | 2025 : 60
::

{#01J2T8W...}
この結果は、[2025年の売上](cell:01J2T8V...#Revenue|2025) を見ればわかる。
```

### 2.3 コンパイル後の canonical グラフ（AIが読み込むメモリ表現）
コンパイラによって、以下のRust構造体（`Graph`）が生成されます。

```rust
// 1. ノードの海
let table_node = Node {
    id: NodeId(ulid_table),
    payload: NodePayload::Table(Table {
        rows: vec![Dim { name: "metric".into(), members: vec![/* Revenue, Cost */] }],
        cols: vec![Dim { name: "year".into(), members: vec![/* 2025, 2026 */] }],
        cells: vec![
            Cell { row_path: vec!["Revenue".into()], col_path: vec!["2025".into()], value: CellValue::Number { v: 100.0 } },
            Cell { row_path: vec!["Cost".into()], col_path: vec!["2025".into()], value: CellValue::Number { v: 60.0 } },
        ]
    })
};

let para_node = Node {
    id: NodeId(ulid_para),
    payload: NodePayload::Para(Para {
        inline: vec![
            Inline::Text { s: "この結果は、".into() },
            Inline::Ref { to: NodeId(ulid_table), rel: Rel::InstanceOf }, // tableへの参照
            Inline::Text { s: " を見ればわかる。".into() },
        ]
    })
};

// 2. エッジの海（リンケージの可視化）
let edge_ref = Edge {
    from: NodeId(ulid_para),
    to: NodeId(ulid_table),
    rel: Rel::InstanceOf, // あるいは特定のデータ参照エッジ
    ord: None,
};
```

---

## 3. 今後の展開とロードマップ

この設計合意により、開発のロードマップが明確になりました。

1.  **SML パーサ・フォーマッタの開発**:
    SML記法をパースし、ID注入および AST/Graph へのビルドを行う Rust クレート `strata-parser`（あるいは `sml2graph`）の作成。
2.  **AIインターフェースの構築**:
    LLMがこのSML（またはコンパイルされたGraph）を直接読み書きし、ドキュメントのグラフ構造を損なわずに推論・編集を行うエージェントツールの開発。
3.  **HTML/Typst レンダラの作成（層3）**:
    多次元表（次元の木）から、結合セル（`colspan`/`rowspan`）を自動計算してブラウザやPDFにレンダリングするビューアの構築。
