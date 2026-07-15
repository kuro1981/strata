# M7.5 実装ハンドオフ — ワークスペース v0.5 + concat + class 継承統一(D44〜D46)

sml-spec §1.11(2026-07-15 確定)の実装指示。M7 の積み残し(単一 render 不能)、
D35 見送りの再裁定(実需2件)、class 継承のセマンティクス穴(ユーザー発見の
「note 連打の気持ち悪さ」の根本)をまとめて解消する。

## 必読(この順)

1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリ)
2. `docs/sml-spec.md` §1.11(D44〜D46)・§1.10(M7)・§1.4(D23)
3. `crates/strata-build/src/workspace.rs`(M7 の統合グラフ)、
   `crates/strata-{typst,md,context,view}`、`crates/strata-cli`
4. `docs/view-def-v1.md`(concat 追記対象)、`docs/sml-agent-guide.md`(指針追記対象)

## スコープ境界(やらないこと)

- strata-html(凍結)・fixture 改版・fmt 変更
- `~/dev/strata-my-resume` は `sml/` 配下のみ書き込み可・git 操作禁止

## 作業パッケージ

### WP-Z1: render / context の workspace 対応(D44)

1. `strata render --workspace <strata.toml> [--doc <文書alias>] --format <typst|md> -o <outdir>`
   — `--doc` 指定時は当該文書のみ、省略時は全メンバーを出力。出力ファイル名は
   メンバーのファイル名 stem+拡張子。cross-doc 参照の描画:
   - **MD**: 相対 .md リンク+アンカー(`[text](work_history.md#見出しアンカー)`。
     対象が見出しでない場合の退化は単一文書時の規則に文書名を添える形で裁量)
   - **Typst**: 単文書 PDF に他文書リンクは張れないため退化テキスト
     (表示 text+「(職務経歴書)」のような文書名注記。体裁裁量)
2. `strata context --workspace <strata.toml> [--node ...] [--hops N] [--class tag]`
   — 統合グラフを対象に。--node の近傍は文書境界を跨いで辿れる(chunk の位置文脈に
   文書名を含める)。--doc での絞り込みも可(裁量)
3. `--hide` は両コマンドの workspace モードでも従来どおり
4. M7 の CrossDocRef Error 案内メッセージに「render --workspace」も含める(文言更新)

### WP-Z2: concat コンビネータ(D45)

1. strata-view に `concat: { parts: [<コンビネータ...>], separator: "" }` を追加
   (parts の各要素は pick/date/literal 等の任意コンビネータ。糖衣文字列も可)
2. `docs/view-def-v1.md` §4 に追記(「見送り」注記を D45 採用に更新、実例付き)
3. 適用(~/dev/strata-my-resume/sml/):
   - cv-jis の氏名: `concat` で `resume/basic-info.姓`+`名`(区切りは元表示に合わせ
     全角空白等、既存出力と同値になるよう裁量)。**M7 で足した resume.sml の
     冗長な `氏名` フィールドを撤去**
   - tech-stack の details(level): v0 で落としていた `(level)` 併記を concat で
     復元(experience スロット。テンプレート未使用スロットだが宣言としては完全に)

### WP-Z3: class の実効セマンティクス統一(D46)

1. **実効 class = 自身+祖先(contains 上流)の和集合**という関数を1箇所に定義し
   (置き場所裁量 — strata-core のヘルパ等)、全消費者を統一:
   - `render --hide <class>`(typst/md): 現行のサブツリー非描画と同値になるはず —
     同値性をテストで固定
   - `context --class <tag>`: 実効 class で選択。コンテナ(Section 等)が該当する
     場合は **--node と同様にサブツリーを chunk として出す**(子を重複列挙しない)
   - view の class フィルタ(include-only-class / exclude-class): 実効 class で判定
2. テスト: コンテナ class(Section・リスト・引用)の3形とも、
   render/context/view の3消費者で一貫すること
3. `docs/sml-agent-guide.md` に指針追記: 「複数ブロックにまたがる note は
   コンテナ(見出し・リスト・引用)に class を**1回**書く(D46 継承)。
   1段落ごとに [class=note] を繰り返さない」

### WP-Z4: ドッグフーディング(気持ち悪さの解消)

`~/dev/strata-my-resume/sml/work_history.sml` にて:

1. **note 連打のリライト**: 1段落ごとに `[id=..., class=note]` を繰り返している
   箇所(特に HUMABUILD の「未来の自分へのメモ」6連打)を、コンテナ形式へ:
   `[class=note]` 付き H5 見出し(例: `##### 【補足】未来の自分へのメモ: …`)の
   配下に素の段落・ネストリスト(「・」の羅列は本来リスト)として再構成。
   他プロジェクトの単発 note は現状のままでよい(1ブロック note は正当)。
   **既存の ULID は可能な限り保存**(fmt が再発行しないよう行の対応を保つ。
   ブロック分割・統合で消える ID があれば報告)
2. 検証:
   - fmt 冪等・`build --workspace` 診断ゼロ
   - `context --workspace --class note` が**リライト前と同じ内容をカバー**
     (件数はコンテナ化で減ってよいが、本文の欠落ゼロ — 【補足】テキストの
     全文が出力に含まれることを機械的に確認)
   - 提出版(--hide note)の view 出力・JIS PDF・MD に note 内容が一切
     漏れないこと(コンテナ継承の実地検証)
   - `render --workspace` で resume.md / resume.typ を**再生成**(M7 以来の
     再生成不能の解消を実証)し、cross-doc 参照が MD で相対リンクに
     なっていることを確認
   - content YAML(氏名・licenses 含む)が従来と同値

## 完了の定義

- `cargo test --workspace` 全通過(D44/D45/D46 の新テスト込み)、clippy 新規警告
  ゼロ(strata-html 既存分対象外)、既存フロー非退行(fixture・ゴールデン無変更。
  context の --class 出力形が変わる場合はゴールデン意図的更新として差分報告)
- WP-Z4 の全検証パス
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所(MD アンカーの
  文書跨ぎ規則・typst 退化体裁・context のコンテナ chunk 形・concat の separator
  規定・ULID 保存の結果)をまとめて終了
