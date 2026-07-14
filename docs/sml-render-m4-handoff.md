# Milestone 4 実装ハンドオフ — `strata render`(canonical グラフ → Typst)

本書は Milestone 4 の設計確定事項と、別セッション/サブエージェント向けの自己完結な
作業指示。設計決定は `sml-spec.md` §1.3 の D18〜D22(2026-07-14 対話にて確定)で
凍結済みであり、本書はその実装への落とし込みを定義する。

## 前提(Milestone 3 完了時点の状態)

- `strata-sml`: パーサ + fmt(フロントマター/コードフェンスID/リストid 対応済み、
  Diag severity あり)
- `strata-build`: `build(src) -> Result<BuildOutput, Vec<BuildError>>`。
  `BuildOutput { graph, root, warnings }`。全282テスト green
- `strata-cli`: `fmt` / `build` サブコマンド + 旧 YAML フロー(`run_legacy`。M4 で削除)
- `strata-typst`: `render_to_typst(&Graph, NodeId) -> Result<String, String>`。
  vault 時代の実装で、M3 語彙(Document/RefersTo/Ref{coord,text}/Term{text}/
  Quantity/Chart.depicts/Table.caption)は未対応または最小フォールバック
- `strata-html`: **凍結**(D19)。触らない
- ゴールデンペア `docs/sml_example_draft.sml` / `sml_example_formatted.sml`(改版しない)

## 必読ドキュメント(この順で読むこと)

1. `AGENTS.md` — ルール: **git commit/push はユーザー指示なしに絶対しない**
2. `docs/sml-spec.md` — 正典。特に §1.3(D18〜D22)・§5.2/§5.3(参照)・§6(フェンス)
3. `crates/strata-typst/src/lib.rs` 全体(改修対象)
4. `crates/strata-build/src/lib.rs`(公開 API)と `crates/strata-core/src/lib.rs`(スキーマ)
5. `crates/strata-cli/src/main.rs`(fmt/build の流儀。render はこれに合わせる)
6. 本書の残り全部

## スコープ境界(やらないこと)

- strata-html に触れない(凍結。コンパイルが通る状態は維持)
- Chart の SVG 実描画はやらない(M5 以降。M4 はプレースホルダ枠まで — D-R4)
- 用語集・`defines` エッジはやらない(保留継続)
- Typst の HTML export・PDF 直出し(typst ライブラリ組み込み)はやらない(将来)
- `render` への JSON 入力・`--format` オプションはやらない(D18/D19。Typst 出力のみ)
- fixture(docs/sml_example_*.sml)は変更しない

## 設計確定事項(本書で凍結)

### D-R1: vault 削除(D20)

- `crates/strata-vault/` を削除、workspace members から除去
- `vault/` ディレクトリ(resume.yaml / work_history.yaml)を削除
- `strata-cli` から `run_legacy` と YAML フロー関連(clap の `--input` 等の旧引数、
  `strata-vault` 依存)を削除。`fmt` / `build` の挙動は変えない
- `crates/strata-cli/examples/compile_takeokunn.rs` が vault 依存なら一緒に削除
- README 等に vault/YAML フローへの言及があれば削除・修正

### D-R2: strata-typst の M3 語彙対応(D21・D22)

`render_to_typst` の中身を canonical グラフ全域に対応させる:

1. **Document(D21)**: `NodePayload::Document` をルートとして受け、
   `#set document(title: "...")` を出力してから contains 子を ord 順に描画。
   title 決定: `Document.title` → 最初の H1 Section の heading プレーンテキスト →
   呼び出し側から渡すフォールバック名(CLI が入力ファイル名を渡せる API にする。
   例: `render_to_typst(&graph, root, fallback_title: &str)` — シグネチャは裁量、
   変更内容を報告)
2. **ラベル(D22)**: 全ブロックノード(Section/Para/List/Table/MathBlock/Figure/Code)
   の直後に `<ULID>` ラベルを付与。Typst のラベルは付与対象の要素が必要な点に注意
   (ブロックに続けて `<...>` を置く)
3. **figure 化(D22)**: Table → `#figure(table(...), caption: ...)`(D16 の
   `Table.caption` を使用)。MathBlock → `#figure` か `$ ... $ <label>`(Typst は
   数式に直接ラベル+番号を振れる — `#set math.equation(numbering: ...)`。方式は
   裁量、番号参照が効くこと)。Figure::Chart / Image → `#figure(caption: ...)`。
   `#set text(lang: "ja")` を文書头に置き、supplement(「表」「図」「式」)は
   Typst の既定ローカライズに任せる(不足なら明示 supplement 指定)
4. **Chart プレースホルダ(D22)**: `#figure` の中身は枠(box/rect)+
   `depicts["description"]` のテキスト + data_ref への参照(`@data_ref_ULID`)。
   mark/encode の情報は小さく併記(例: `bar: dataset × f1`)。見栄えの詳細は裁量
5. **インライン**:
   - `Inline::Ref { to, text, coord, .. }`: text 非空 → `#link(<to>)[text]`。
     text 空 → `@to`(番号付き対象)。番号を持たない対象(Para/List/Code)への
     text 空参照 → `#link(<to>)[§]` 等の短い代替表記(裁量。黙って落とさない)。
     `coord` があれば表示テキストに `(行パス, 列パス)` を添える(体裁は裁量)
   - `Inline::Term { to, text }`: text 非空 → text、空 → Term ノードの `name`。
     体裁はプレーン(強調しない)。Term ノード自体は描画しない(グラフにのみ存在)
   - `CellValue::Quantity { v, unit }`: `12 ms` 形式(数値と単位の間に半角スペース)。
     表セル内の揃えは既存の表整形に従う(最小限)
6. **エッジ**: 描画が辿るのは contains のみ。supports/depends-on/cites/RefersTo/
   TermRef エッジは描画しない(グラフの意味情報であり、紙面には出さない)

### D-R3: CLI `render` サブコマンド(D18)

```
strata-cli render <file.sml>              # Typst マークアップを stdout へ
strata-cli render <file.sml> -o out.typ   # ファイルへ(原子的書き込み)
```

- 内部: 読み込み → `strata_build::build` → `render_to_typst`。中間 JSON なし
- `root: None`(フロントマター無しで build 成功)→ exit 2、
  「フロントマターがありません。`strata fmt` を先に実行してください」
- exit code: 成功 0 / 読み書き失敗 1 / BuildError・render エラー 2。
  BuildError の表示は `build` と同一形式を再利用。Warning は stderr 表示 + exit 0
- 既存 `fmt` / `build` の挙動を変えない

## 作業パッケージ分割

依存: `WP-R1(vault 削除)` は独立、`WP-R2(typst 改修) → WP-R3(CLI + 統合)`。
WP-R1 ∥ WP-R2 は並列可(WP-R1 が run_legacy を消すため strata-cli で衝突し得る —
WP-R3 と同一エージェントに寄せるか、順次実行を推奨)。

### WP-R1: vault 削除

- D-R1 の全部
- 完了条件: `cargo test --workspace` 全通過(vault 分のテストが消えるのは想定内)、
  `cargo build --workspace` クリーン、`fmt`/`build` の統合テスト無傷

### WP-R2: strata-typst 改修

- D-R2 の全部
- テスト: formatted fixture のグラフ(build 経由)→ Typst 文字列のゴールデン契約
  (完全一致で固定。決定的なので可能)。加えて要素別の単体テスト
  (Document title フォールバック3段・text有無×番号有無の Ref 4形・Term
  フォールバック・Quantity・Chart 枠・coord 付き cell 参照)
- 手元に typst バイナリがあれば `typst compile` が通ることも確認(無ければ
  スキップし、その旨報告。devshell への typst 追加は勝手にしない — 報告事項)

### WP-R3: CLI render + 統合テスト

- D-R3 の全部
- 統合テスト: formatted fixture の render 成功(ゴールデン .typ と一致)、
  draft → exit 2(MissingId 案内)、フロントマター無し・全 ULID 手書きの合成入力 →
  exit 2(D21 の案内)、`-o` 原子的書き込み、fmt/build 退行なし

## 完了の定義

- 全 WP のテストを含め `cargo test --workspace` 通過
- `nix develop -c cargo clippy --workspace --all-targets` で新規・変更コードの
  警告ゼロ(凍結中の strata-html の既存警告は対象外)
- 既存 CLI(fmt/build)が退行していない
- **コミットはしない**。変更ファイル一覧・テスト消化状況・仕様の曖昧点
  (勝手に解釈せず報告)をまとめて終了

## 既知の注意点

- strata-typst の現実装は vault 時代の想定(Section 直下中心)。M3 グラフは
  Document ルート + 全ブロック種が来るため、`render_node` の網羅を先に固めること
- Typst のラベルは「直前の要素」に付く。段落直後の `<label>` は段落に付くが、
  空行を挟むと浮く — ゴールデンで固定して回帰を防ぐ
- `@ref` は参照先に番号付け(figure/equation/heading の numbering)が無いと
  コンパイルエラーになる。番号を振らない要素への参照は必ず `#link` 側に倒すこと
- Typst 文字列のエスケープ(`#`, `@`, `<`, `>`, `$`, `*`, `_` 等)は既存実装の
  エスケープ関数を確認し、新経路(caption・depicts・Term name 等)にも必ず通すこと
- vault 削除で `strata-core` の一部 API が未使用になる可能性があるが、
  strata-core は触らない(dead code 警告が出た場合のみ最小対応し報告)
