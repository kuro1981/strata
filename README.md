# Strata — 意味グラフ文書フォーマット

Strata is a semantic-graph document format: a single canonical graph (nodes +
typed edges) from which both human-facing views (Typst/Markdown) and
AI-facing views (an ID-addressable context dump) are derived losslessly. SML,
its authoring surface, is a strict superset of CommonMark/GFM — any plain
Markdown file is already a valid SML draft.

人志向の読みやすさと機械志向の厳密さを、1つの **canonical グラフ**から両方
導出することで両立させる文書フォーマット。オーサリング表面の SML
(Strata Markup Language)は Markdown (CommonMark/GFM) の**上位互換**として
設計されており、素の `.md` ファイルはそのまま有効な SML ドラフトになる。

55件以上の設計対話を経て今のかたちに至っている。判断の経緯を追いたい場合は
`docs/sml-spec.md` §1(D1〜D58 の裁定一覧)と `Plans.md` / git log を参照。

---

## アーキテクチャ

Strata は3層モデルで構成される。

```
層1: SML (オーサリング表面)      … 人/AI が書く。Markdown 互換 + ID・意味エッジ・
                                    多次元表・数式などのアノテーション
       │ strata fmt   (ID 逆注入・冪等)
       │ strata build (パース・参照解決・不変条件検証)
       ▼
層2: canonical グラフ            … Node + Edge のみ。真実の源(ロスレス)
       │ strata render / view / context / search / site
       ▼
層3: ビュー群                    … Typst / Markdown(人向け)、AI コンテキスト、
                                    検索結果、宣言的ビュー定義の出力、
                                    静的グラフ UI サイト
```

- **ワークスペース**(`strata.toml`、`members` のグロブ列挙)で複数の `.sml`
  ファイルを束ね、ファイル横断の参照(`ref:<文書alias>/<ブロックalias>`、
  `doc:<文書alias>`)を解決できる。
- **ビュー定義**(YAML、`docs/view-def-v1.md`)は canonical グラフから
  テンプレート消費用のデータファイルを宣言的に取り出す仕組み。セレクタ
  (alias / class / セル座標 / 型+contains パス)とコンビネータ(rename・
  rows・join・date・age・concat 等)の組み合わせのみで、スクリプトや正規表現は
  持たない。
- **実効 class**(「これは何であるか」を書くブロック属性)は自身+祖先の和集合
  として全消費者(`render --hide` / `context --class` / view のフィルタ)に
  一貫して適用される。「誰に見せるか」は文書ではなくビュー側の仕事。

## クイックスタート

```bash
nix develop                      # Rust(cargo/clippy)+ Typst + CJK フォント + Node/pnpm 一式
cargo run -p strata-cli -- --help
```

## CLI コマンド一覧と実例

以下は `docs/spec-sml/`(このリポジトリ自身の設計決定 D1〜D58 を SML 化した
ワークスペース、`docs/spec-sml/strata.toml` + `decisions.sml`)を対象にした
実際に動く例。

### fmt — ID 逆注入(フォーマッタ)

```bash
cargo run -p strata-cli -- fmt docs/spec-sml/decisions.sml
cargo run -p strata-cli -- fmt --check docs/spec-sml/decisions.sml   # 差分の有無だけ確認(exit 0/1)
```

### build — SML → canonical グラフ(JSON)

```bash
cargo run -p strata-cli -- build --workspace docs/spec-sml/strata.toml
```

### render — Typst / Markdown への描画

```bash
# 既定は Typst(一次レンダラ)
cargo run -p strata-cli -- render --workspace docs/spec-sml/strata.toml --format typst -o out/

# 人間向け最小依存ビュー(GitHub・チャット・エディタでそのまま読める GFM)
cargo run -p strata-cli -- render --workspace docs/spec-sml/strata.toml --format md -o out/
```

### view — 宣言的ビュー定義の適用

```bash
cargo run -p strata-cli -- view docs/spec-sml/decisions.sml --view <view-def.yaml> --check
```

### context — AI 向けコンテキストビュー

```bash
cargo run -p strata-cli -- context --workspace docs/spec-sml/strata.toml
# 特定ノード周辺だけ(意味エッジを N ホップ辿った近傍つき)
cargo run -p strata-cli -- context --workspace docs/spec-sml/strata.toml --node d19 --hops 1
```

### search — 全文検索 + 構造述語

```bash
cargo run -p strata-cli -- search "スパンパッチ" --workspace docs/spec-sml/strata.toml
cargo run -p strata-cli -- search "alias:d1" --workspace docs/spec-sml/strata.toml
cargo run -p strata-cli -- search "class:note" --workspace docs/spec-sml/strata.toml --json
```

### site — 自己完結の静的グラフ UI

```bash
cargo run -p strata-cli -- site --workspace docs/spec-sml/strata.toml -o out/site
# out/site/index.html を直接開く(サーバ不要)。graph.json + 事前ビルド済み UI(ui/dist)を合成
```

## ドキュメント案内

| ドキュメント | 位置づけ |
|---|---|
| `docs/sml-spec.md` | **正典 (normative)**。§1 に D1〜D58 の裁定一覧(設計決定の記録)、以降にブロック分類・ID規則・文法・処理パイプラインなど |
| `docs/sml-agent-guide.md` | AI エージェント向けの SML 執筆ガイド(これだけ読めば書けることを品質基準とする実用要約) |
| `docs/view-def-v1.md` | ビュー定義 YAML の文法(D30〜D35) |
| `docs/*-handoff.md` | 各マイルストーンの実装ハンドオフ(設計決定→実装への橋渡し) |
| `Plans.md` / git log | 初期構想〜マイルストーン計画の履歴(現状と乖離している箇所もある一次資料) |

`docs/` にはこの他、互換性監査(`md-compat-audit.md`)やパーサ設計メモなど
実装過程の資料が置かれている。読む場所に迷ったら `sml-spec.md` §1 から入り、
関心のある Dn の周辺ドキュメントへ辿るのが早い。

## strata-editor(別リポジトリ)

Strata 用の Tauri v2 エディタ、[strata-editor](https://github.com/kuro1981/strata-editor)
(未公開ならローカル `~/dev/strata-editor`)。Obsidian の代替を狙う骨格で、
2ペイン(グラフ⇄文書)の往復・CodeMirror 6 編集ループ・保存時 fmt/build を持つ。
フォーマット本体(このリポジトリ)への依存は公開境界(クレートの path/git 依存、
graph JSON スキーマ、`ui/` ビューアの外部参照)のみで、エディタ無しでも
Strata の全機能(CLI・ビュー・検索・静的サイト出力)は完結する。

## フィードバック

不具合報告・提案は [GitHub Issues](https://github.com/kuro1981/strata/issues)
へ。Strata の機能追加は「Issue → 設計対話で裁定(`sml-spec.md` §1 に Dn として
凍結)→ 実装 → Issue を Closes」という流れで進む。運用の詳細は
[CONTRIBUTING.md](CONTRIBUTING.md) を参照。

## ライセンス

MIT ([LICENSE](LICENSE))
