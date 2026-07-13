# Milestone 2 実装ハンドオフ — `strata fmt`(ID逆注入フォーマッタ)

本書は Milestone 2 の設計確定事項と、別セッション/サブエージェント向けの自己完結な
作業指示。fmt のコア設計(スパンパッチ方式・3種の編集・契約)は `sml-spec.md` §8 で
凍結済みであり、本書はその実装への落とし込みを定義する。

## 前提(Milestone 1 完了時点の状態)

- `crates/strata-sml` にスパン付き SML パーサが実装済み(156テスト、clippy クリーン)
- 全ブロックがバイトオフセットの `Span` を持つ。`IdTag.inner_span` は `{...}` の内側
  (fmt の置換対象)を指す
- ゴールデンペア `docs/sml_example_draft.sml` / `sml_example_formatted.sml` は
  **fmt の入出力仕様そのもの**(formatted は draft に fmt を掛けた期待結果)
- devshell に clippy あり(`nix develop -c cargo clippy` または direnv シェル内で実行)

## 必読ドキュメント(この順で読むこと)

1. `AGENTS.md`(リポジトリ直下に無ければ読み飛ばす)— ルール: **git commit/push は
   ユーザー指示なしに絶対しない**
2. `docs/sml-spec.md` — 正典。特に §3(IDとエイリアス)と §8(fmt の契約・エラー方針)
3. `docs/sml-parser-design.md` — パーサの設計(層Aだけで fmt が成立する、の意味)
4. 本書の残り全部
5. `crates/strata-sml/src/`(特に ast.rs の `IdTag`/`AttrLine` と block.rs)
6. ゴールデンペア2ファイル

## スコープ境界(やらないこと)

- build(M3)・エイリアス→ULID の参照解決・canonical グラフ構築はやらない
- **参照(`(cell:eval-table#...)` 等)の書き換えは絶対にしない**(D3: エイリアス温存)
- パーサの構文解釈は変更しない(fmt はパーサの出力を消費するだけ。パーサのバグを
  見つけたら報告し、fmt 側で回避しない)
- コードフェンス(```)には ID を注入しない(仕様保留事項。sml-spec §10)
- strata-core / strata-vault / strata-html / strata-typst / tex2math / docs/ に触れない
  (strata-cli は WP-F3 の指定範囲のみ変更可)

## 設計確定事項(本書で凍結)

### D-F1: 配置

fmt のロジックは `crates/strata-sml/src/fmt.rs`(ライブラリ関数)。CLI は strata-cli に
サブコマンドとして載せる(WP-F3)。

```rust
pub struct Patch {
    pub at: usize,       // 挿入/置換の開始バイトオフセット(元テキスト基準)
    pub delete: usize,   // 削除バイト数(挿入のみなら 0)
    pub insert: String,
}

pub struct FmtOutput {
    pub text: String,        // パッチ適用後の全文
    pub patches: Vec<Patch>, // 監査・テスト用(オフセット降順に適用)
}

/// diags が1件でもあれば Err(全か無か、sml-spec §8.2)
pub fn format_with(src: &str, idgen: &mut dyn FnMut() -> Ulid) -> Result<FmtOutput, Vec<Diag>>;
pub fn format(src: &str) -> Result<FmtOutput, Vec<Diag>>;  // ULID Generator で document 順に単調
```

### D-F2: ID 生成の決定性

`format_with` が ID 生成器を注入可能にする(テストの決定性のため)。本番 `format` は
`ulid::Generator`(単調増加)を使い、**文書順**(ブロック出現順、リスト内は項目順)に
発行する — 生成された ULID のソート順が文書順と一致する。

### D-F3: パッチの3種(sml-spec §8.1 の実装対応)

| ケース | 編集 |
|---|---|
| 行型ブロック(見出し・リスト項目・フェンスマーカー)に IDタグ無し | 行末(改行の直前、行末空白の後ろは付けない)に ` {#ULID}` を**挿入** |
| 段落に前置属性行が無い | 段落の先頭行の直前に `[id=ULID]\n` 行を**挿入** |
| 段落の前置属性行に `id` キーが無い | `[` の直後に `id=ULID, ` を**挿入**(既存エントリの前) |
| IDタグ/属性行の id が**非ULIDラベル** | `{#label}` の内側を `ULID alias=label` に**置換** / `[id=label, ...]` の `label` を `ULID, alias=label` に置換 |
| 既に ULID を持つ(alias 有無問わず) | **何もしない** |

パッチはオフセット**降順**に元バイト列へ適用する(先行オフセットが不変のまま)。

### D-F4: CLI(strata-cli)

clap の `args_conflicts_with_subcommands = true` を使い、**既存の引数形式
(`-i/-o/-f`、YAML→HTML/Typst)を無変更で温存**したままサブコマンド `fmt` を追加する:

```
strata-cli fmt <file.sml>          # インプレース整形
strata-cli fmt --check <file.sml>  # 変更が必要なら exit 1(パッチ内容を表示)、不要なら 0
strata-cli -i resume.yaml -o out.html   # 従来動作(退行させない)
```

- 書き込みは**原子的**: 同一ディレクトリの一時ファイルに書いて rename
- パースエラー時: ファイルに触れず、Diag を「行:列: 種別: メッセージ」形式で stderr に
  全件出力して exit 2(`Span` → 行/列変換は span.rs の既存機能を使う)

### D-F5: 契約テスト(sml-spec §8.1 の4契約を機械化)

1. **ゴールデン完全一致**: fixture の16個の ULID(`01J2T8Z0000000000000000000` 〜
   `01J2T8ZF000000000000000000`)を文書順に返す決定的生成器を注入し、
   `format_with(draft) == formatted` を**バイト完全一致**で検証
2. **冪等性**: `format(formatted fixture)` のパッチが**0件**であること。および任意入力で
   `format(format(x).text).patches.is_empty()`
3. **挿入のみ**: 全パッチが「`delete == 0`」または「削除範囲が `{...}`/`[...]` の内側に
   収まる置換」であることをパッチ列から機械検証
4. **意味保存**: `parse(format(x).text)` と `parse(x)` が ID 無視で同型。同型比較は
   `tests/golden_isomorphism.rs` の正規化関数を `tests/common/mod.rs` に共通化して再利用

## 作業パッケージ分割

依存: `WP-F1 → {WP-F2 ∥ WP-F3}`

### WP-F1: fmt コア(`crates/strata-sml/src/fmt.rs` + lib.rs への `pub mod fmt;` 追記)

- D-F1〜D-F3 の実装。パッチ計画(パース結果の走査)と適用を分離すること
- 単体テスト(fmt.rs 内 or tests/fmt_core.rs): 上表の全ケース × ブロック種別、
  ULID済みブロックが無変更であること、パッチ降順適用の正しさ、エラー時 Err
- 変更可: `src/fmt.rs`(新規)、`src/lib.rs`(mod 宣言と re-export のみ)、
  自分のテストファイル

### WP-F2: 契約テスト(`crates/strata-sml/tests/fmt_contract.rs` + `tests/common/mod.rs`)

- D-F5 の4契約を実装。共通化に伴い `golden_isomorphism.rs` から正規化関数を
  `tests/common/mod.rs` へ移す(この移動に限り既存テストファイルの変更可。
  テストの検証内容自体は弱めないこと)
- 追加のプロパティ: fmt 後のファイルを再パースして diags ゼロ / 全ブロックが
  ULID の ID を持つ(コードフェンスを除く)

### WP-F3: CLI(`crates/strata-cli/src/main.rs`)

- D-F4 の実装。既存の YAML フローの挙動を**一切変えない**(既存呼び出し形式の
  スモークテストを先に書いてから着手すること)
- 変更可: `crates/strata-cli/src/main.rs`、`crates/strata-cli/Cargo.toml`
  (strata-sml への依存追加)
- 動作確認: fixture の draft を一時ディレクトリにコピーして `fmt` を実行し、
  再実行で無変更(冪等)・`--check` の exit code・パースエラー時にファイル無傷、を確認

## 完了の定義

- D-F5 の契約テスト4本を含む全テストが `cargo test --workspace` で通過
- `nix develop -c cargo clippy -p strata-sml -p strata-cli --all-targets` 警告ゼロ
  (既存3クレートの警告は対象外)
- 既存の CLI 呼び出し形式が退行していない
- **コミットはしない**。変更ファイル一覧・契約テストの消化状況・発見した仕様の
  曖昧点(勝手に解釈せず報告)をまとめて終了する

## 既知の注意点

- 行末 ` {#ULID}` 挿入時、リスト項目に全角ダッシュ等が含まれる(fixture 参照)。
  バイトオフセットは必ず char boundary を尊重すること(スパンはパーサ由来なので
  基本安全だが、行末検出を自前でやる場合に注意)
- fixture の formatted は挿入行の後に元の空行構造を完全温存している。パッチが
  改行を余分に足す/削ると契約1(バイト完全一致)で即検出される — これは意図した罠
- 段落は複数行にまたがる(fixture の導入段落)。`[id=...]` 行の挿入位置は
  「ブロックの先頭行の行頭」であり、属性行が既にある場合はそれが先頭行になる点に注意
