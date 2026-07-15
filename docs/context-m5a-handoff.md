# M5-A 実装ハンドオフ — `strata context`(AI 向けコンテキストビュー、D36)

sml-spec §1.7(2026-07-15 確定)の実装指示。グラフを LLM が読める形に直列化する
専用サブコマンドと、実データでのドッグフーディング(面接想定問答の生成)まで。

## 必読(この順)

1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリ)
2. `docs/sml-spec.md` §1.7(D36)・§1.4(D23 class)・§10
3. `crates/strata-build/src/lib.rs`(公開 API: BuildOutput/Graph/Node/Edge)、
   `crates/strata-cli/src/main.rs`(サブコマンドの流儀)
4. `docs/sml_example_formatted.sml`(ゴールデン対象)と
   `~/dev/strata-my-resume/sml/work_history.sml`(ドッグフーディング対象)

## スコープ境界(やらないこと)

- エッジ種の選別パラメータ(保留)、ファイル横断、埋め込み/ベクトル化
- B(AI が書く規約)・C(ビュー定義提案)— 次の対話
- fixture 改版・既存コマンド非退行・strata-html 凍結、
  `~/dev/strata-my-resume` は `sml/` 配下のみ書き込み可・git 操作禁止

## 作業パッケージ

### WP-A1: 直列化器

- 置き場所は裁量(strata-view 内モジュール or 新クレート strata-context。報告)
- 入力: `strata_build::BuildOutput`。出力: Markdown 文字列
- **形式要件**:
  - 全ブロックノードが ULID でアドレス可能(alias があれば併記 — LLM の引用は
    alias 優先が読みやすい)。見出しは `##`、段落は本文、record は `キー: 値` 行、
    表はコンパクトな行表現(行パス | 列パス: 値)、class 付きは明示
  - 意味エッジ(supports / depends-on / cites / refers-to / term)は
    「エッジ」節に一覧(contains は構造自体で表現されるので列挙しない)
  - 決定的(同一入力→バイト同一)。トークン効率に配慮しつつ可読性優先
- **スコープ3形態(D36)**:
  1. 無指定 = 全文書
  2. `--node <alias|ULID>`(複数可)+ `--hops N`(既定1): 指定ノードの contains
     サブツリー = chunk 本体。意味エッジを N ホップ辿った先のノードを「近傍」節に
     要約付加(近傍はサブツリー全展開せず、そのノード自体+親文脈の1行程度)
  3. `--class <tag>`: 該当 class を持つブロックを横断列挙(各項目に親セクション名
     等の位置文脈を1行付ける)
  - 2 と 3 の併用可否は裁量(素直なら併用=AND。報告)

### WP-A2: CLI とテスト

- `strata context <file.sml> [-o out.md] [--node ...] [--hops N] [--class tag]`
- exit code 0/1/2・Warning stderr の既存流儀。存在しない alias/ULID 指定は
  明確なエラー(exit 2)
- テスト: fixture(docs/sml_example_formatted.sml)のゴールデン(全文書)+
  スコープ別単体(node+hops のホップ境界、class 抽出、決定性、不在ノード)

### WP-A3: ドッグフーディング — 面接想定問答

1. `strata context work_history.sml --class note` の出力を確認(35 note が
   位置文脈付きで出ること)
2. その出力**だけ**を根拠に、面接想定問答集を
   `~/dev/strata-my-resume/sml/interview_qa.md` として生成する(あなた自身が
   LLM として読んで書く — これが「AI が読む」のドッグフーディング本体)。
   各問答に**根拠ノードの alias/ULID を引用**として付けること(引用可能性の実証)
3. `--node <どれかのプロジェクト alias> --hops 1` の出力例を1つ取り、
   chunk として意味的に自己完結しているか(そのプロジェクトについて答えるのに
   過不足ないか)を自己評価
4. 報告: 全文書/class/node 各スコープの出力サイズ(行数・概算トークン)、
   コンテキストとして不足だった情報・ノイズだった情報(形式改善の材料)、
   引用の書きやすさ

## 完了の定義

- `cargo test --workspace` 全通過(新テスト込み)、clippy 新規警告ゼロ
  (strata-html 既存分対象外)、既存コマンド非退行、fixture 無変更
- interview_qa.md が生成され、全問答に根拠引用が付いていること
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所(形式の詳細・置き場所・
  併用の扱い)・WP-A3 の評価をまとめて終了
