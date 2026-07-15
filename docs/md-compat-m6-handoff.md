# M6 実装ハンドオフ — CommonMark/GFM 互換(D40)

sml-spec §1.9(2026-07-15 確定)の実装指示。監査 `docs/md-compat-audit.md` で
判明した「静かに壊れる」9件の根絶+GFM 実用拡張。**素の .md がそのまま安全な
SML ドラフトになる**ことが M6 のゴール(D39)。

## 必読(この順)

1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**
2. `docs/md-compat-audit.md` — 全対象の最小再現と実挙動(本作業の要件一覧を兼ねる)
3. `docs/sml-spec.md` §1.9(D40)・§2〜§6・§8(fmt 契約)
4. `crates/strata-sml/src/{scan,block,inline}.rs`(改修の本丸)、
   `crates/strata-core/src/lib.rs`、`crates/strata-build`、
   `crates/strata-typst`、`crates/strata-context`

## スコープ境界(やらないこと)

- 脚注(保留 §10)・HTML の構造化(非対応明記+Warning のみ)
- MD レンダラ本体(D38 — 次タスク。ただし本 M6 の語彙追加が前提になる)
- strata-html は凍結(コンパイル維持の最小保守のみ可)
- fixture `docs/sml_example_*.sml` 改版禁止。ゴールデン(.typ/.context.md/JSON)の
  変更は意図的更新のみ・差分報告
- `~/dev/strata-my-resume` は今回触らない(回帰確認の read+CLI 実行のみ可)

## 作業パッケージ(順次)

### WP-C1: インライン層(監査②1,2,3,7 と ④の関連)

1. **エスケープ**: CommonMark の backslash escape(ASCII 記号)。`\*` は
   リテラル `*`。fmt のバイト保存契約と両立させる(ソースは保存、グラフ上の
   Text は unescape 済み)
2. **強調ネスト修正**: `***bold italic***`、`_em_`/`__strong__` 対応
3. **外部リンク**: `[text](https://…)`・`http`・`mailto` を
   `Inline::Link { url, text }`(core 新設)に。autolink `<https://…>` も。
   `UnknownScheme` Error は真に未知のスキームに限定して維持
4. **インライン画像**: `![alt](url)` を `Inline::Image { url, alt }`(core 新設)に。
   `![alt](ref:target)` の `!` 孤立バグ(監査②3)も解消(内部参照画像の扱いは
   裁量: 当面 Error 診断で明示拒否も可・報告)
5. **参照スタイルリンク**: `[text][label]` + 定義行 `[label]: url "title"` を
   解決し、定義行は**非可視メタ**(グラフに段落ノードを作らない)。
   未解決ラベルはリテラル維持(CommonMark 準拠)
6. **`~~取消線~~`**: `EmphKind` に追加
7. リンクテキスト内の書式(`[**B**](…)`)のインライン構造化(裁量の範囲で。
   見送るなら報告)

### WP-C2: ブロック層(監査②5,6,8,9 と ④の関連)

1. **ゆるいリスト統合**: 空行で区切られた同種マーカーの連続リストを
   1つの List に(CommonMark の loose list)。既存文書のグラフ形が変わる場合は
   影響を報告
2. **順序リスト start 保存**: `core List` に `start: Option<u64>`(serde 後方互換)。
   `5.` 始まりを保持
3. **代替マーカー**: `*`・`+` 箇条書き、`1)` 順序マーカー
4. **Setext 見出し**: H1(`===`)/H2(`---`)を Section に。段落直後の `---` は
   CommonMark 準拠で Setext H2 が優先。fmt の ID 注入方法は裁量(テキスト行末
   `{#id}` 等)・報告
5. **`~~~` コードフェンス**: ``` と同等に(監査②8 の誤爆根絶)
6. **blockquote**: `>` 行群を `NodePayload::Quote`(core 新設、contains で子ブロック)
   に。ネスト引用は裁量(v0 は1段でも可・報告)
7. **見出し閉じ `#` 装飾**: `# H #####` の末尾装飾を除去して heading に
8. **水平線**: 単独行 `---`(段落に隣接しない場合)を… 裁量: ThematicBreak
   ノード新設 or 保留のまま素通し。判断と理由を報告
9. **HTML(D40 Tier 3)**: HTML ブロック/インラインらしき行に **Warning**
   (新診断種別)を出しつつリテラル扱い維持

### WP-C3: GFM パイプ表・タスクリスト

1. **パイプ表ブリッジ**: `| A | B |` + 区切り行 + データ行を**フラット2次元の
   Table ノード**へ(ヘッダセル=列 member label、行 key は自動採番。member key
   の自動生成規則は裁量・報告)。セル値は既存の型付きパースを通す。
   `::table` との使い分けは「多次元・ID 参照が要るなら ::table、単純表は GFM」
   として仕様の書き味を保つ
2. **タスクリスト**: `- [ ]`/`- [x]` のチェック状態を構造化(置き場所は裁量:
   候補は項目ノードのフィールド。core 変更を報告)。監査指摘の
   「単独行 `[ ]` が属性行と誤認され BadKeyCharset」も検証し解消
3. fmt の ID 注入対象(GFM 表に `{#id}` を置く場所)は裁量・報告

### WP-C4: 波及と受け入れ

1. **strata-typst / strata-context** に新語彙の描画を追加(Link/Image/取消線/
   Quote/start/タスク状態/GFM 表)。typst が一次(D19)
2. 受け入れテスト: 監査②の**9件全ての最小再現が解消**していること(監査文書の
   入力をそのままテスト化)+ 素の CommonMark サンプル文書(見出し・リスト・
   リンク・引用・表・コードを含む1本)が fmt→build を無診断で通ること
3. fmt 契約(純挿入・冪等・全か無か)の非退行。既存 SML 文書
   (fixture・~/dev/strata-my-resume/sml/*.sml)の build 結果が変わらないこと
   (ゆるいリスト統合で変わる場合はその差分だけを報告)

## 完了の定義

- `cargo test --workspace` 全通過(受け入れテスト込み)、clippy 新規警告ゼロ
  (strata-html 既存分対象外)
- 監査②9件の解消、fixture 無変更、既存文書の非退行(報告済み差分を除く)
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所(エスケープと fmt
  契約の整合、blockquote/HR/タスク状態のモデル、GFM 表の key 自動生成規則、
  参照リンク定義行の扱い等)・既存文書への影響をまとめて終了
