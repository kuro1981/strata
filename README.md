# Strata — 意味グラフ文書フォーマット

人志向ドキュメントと機械志向ドキュメントを **1つの源(canonical)** から派生させる、新しい文書表現。
Markdown の制約を越え、既存フォーマットの上に乗せず、独立した表現として設計する。

このリポジトリは設計対話から起こした初期成果物。Claude Code + git 管理への中間地点。

---

## このプロジェクトの出発点(対話の要約)

### 問い
人志向(Markdown のような可読性重視)と機械志向の両方が要る時代。機械が複雑さ(数式・多段結合表など)を読め、parser を通せば人も読める表現は何か。当初は「隠蔽された HTML か?」と考えた。

### 辿り着いた骨子

1. **複雑さは6軸に分解できる**
   - 軸1 参照構造(線形→木→DAG→グラフ)
   - 軸2 局所表記(数式・入れ子の再帰木)
   - 軸3 レイアウト束ね(結合セルは概念か装飾か)
   - 軸4 文脈依存(「下の表」等の指示語)
   - 軸5 連続実体(画像: 記号図 / 写真)
   - 軸6 物理レイアウト(改ページ・段組・列幅)
   - Markdown は6軸中5軸で力不足。

2. **二項対立ではなく三層**
   - 層1 オーサリング(人が書く / 書きやすさに全振り)
   - 層2 canonical(機械が読む + ロスレス / 真実の源)
   - 層3 render(人が読む・機械が食う / 紙・画面・音声・点字)
   - Markdown の敗因は1ファイルで3役を兼ねたこと。「隠蔽された HTML」の直感は層2と層3の分離を指していた。

3. **canonical の核心**
   - **Node + Edge の2レコードだけ**。ファイルは無く、グラフがある。
   - 表は格子+colspan ではなく **次元の木**(MultiIndex 的)。これで軸3が消え、軸6 も同時に排除される。
   - 物理レイアウト語を payload 型に持たせない → 漏洩が構文的に不可能。
   - 粒度はレベル1(段落=ノード)既定 + 需要駆動の `anchor` 昇格。
   - 数式は MathML サブセットが canonical、TeX はオーサリング表面。

4. **ストア設計**
   - store(真実の源) = **プレーンテキストの vault**(ドキュメント単位ファイル + ID 埋め込み)。可搬・git 可・grep 可。
   - index = 派生・使い捨て。当面 **DB なし**(インメモリ Graph を起動時再構築)。必要なら redb。
   - データはグラフだが、クエリは1〜2ホップなので **グラフDBは不要**。メモリ上は graph 指向、ディスクはフラット。
   - Qdrant(ベクトル検索)は重いので後回し。

5. **新規性(正直な評価)**
   - 構成要素はほぼ先行例あり(シングルソース出版 / Xanadu / OHCO-TAG / GraphRAG / MyST / PreTeXt)。
   - 独自性は (a) 没交渉な4系統 + AI文脈の統合、(b) 表を次元の木で持ち軸3を問いごと消す、(c) 物理排除を型不変条件として強制、の3点。
   - 既存実用フォーマット(Markdown/MyST/PreTeXt)は皆 OHCO=木。Strata は反証済みの木前提を捨て Text-as-Graph を実用基盤に採る。

詳細は `docs/strata-spec.md` を参照。

---

## リポジトリ構成(想定)

```
strata/
├── README.md                  # これ
├── docs/
│   └── strata-spec.md         # 仕様 v0.1(凍結 vs 保留、先行研究、精密な差分まで)
├── crates/
│   ├── strata-core/           # canonical スキーマ(§2/§4/§5/§6)+ 不変条件チェック
│   │   └── src/lib.rs         # = strata-core.rs
│   └── tex2math/              # TeX → MathNode(MathML サブセット)Pratt パーサ
│       └── src/lib.rs         # = tex2math.rs
└── CONVERSATION.md            # 設計対話の全文ログ(時系列)
```

現状のファイル(このアーカイブ内):
- `strata-spec.md`  — 仕様本体
- `strata-core.rs`  — strata-core クレートの lib.rs(cargo test 4本パス・警告ゼロ確認済)
- `tex2math.rs`     — tex2math クレートの lib.rs(cargo test 14本パス・警告ゼロ確認済)
- `CONVERSATION.md` — 設計対話ログ

---

## Claude Code / git への移行手順(目安)

```bash
# 1. cargo ワークスペースを作る
mkdir strata && cd strata
git init

# 2. ワークスペース Cargo.toml
cat > Cargo.toml <<'TOML'
[workspace]
resolver = "2"
members = ["crates/strata-core", "crates/tex2math"]
TOML

# 3. 各クレートを作って lib.rs を配置
cargo new crates/strata-core --lib
cargo new crates/tex2math --lib
# strata-core.rs → crates/strata-core/src/lib.rs
# tex2math.rs    → crates/tex2math/src/lib.rs
# 各 Cargo.toml に serde / serde_json / ulid(core のみ)を追加

# 4. docs/ と README を配置
mkdir docs
# strata-spec.md → docs/strata-spec.md

# 5. 確認してコミット
cargo test
git add -A && git commit -m "Strata v0.1: spec + canonical schema + tex2math parser"
```

### 各クレートの依存(Cargo.toml)

strata-core:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ulid = { version = "1", features = ["serde"] }
```

tex2math:
```toml
[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

> 補足: 現状 `tex2math` は `MathNode` を独自に持っている(strata-core のコピー)。
> git 化の際に `MathNode` を strata-core 側に一本化し、tex2math は strata-core に依存させて
> 重複を解消するのが最初のリファクタ候補。

---

## 次の一手(候補)

- `MathNode` を strata-core に一本化し、`Inline::Math { tree }` への結線
- `MathNode → MathML 文字列` レンダラ(層3。ブラウザ表示まで通すと層1→2→3 が数式で一気通貫)
- 仕様 §10 を「store/index 分離 + プレーンテキスト vault」へ正式改訂
- vault(プレーンテキスト)⇄ インメモリ Graph の load/save
- 表(次元の木)→ HTML レンダラ(結合セルの span を葉数から計算)

## サブセットの既知の穴(出たら足す方針, §6)
行列(`\begin{matrix}`)、書体(`\mathbb`/`\mathcal`)、多文字関数名(`\sin`/`\log`)は未対応。
`UnknownCommand` で炙り出されるので、出てきた順に追加する。
