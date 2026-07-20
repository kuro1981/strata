# Contributing to Strata

## フィードバック

不具合報告・機能要望は [GitHub Issues](https://github.com/kuro1981/strata/issues)
へ。専用テンプレートは無い(自由記述でよい)。良い書き方の実例として
[Issue #1](https://github.com/kuro1981/strata/issues/1)(`rows: contains` への
`exclude-class` 追加要望)を参照してほしい。「要望 → 背景 → ユースケース →
(あれば)提案」の形で、具体的な入出力例が添えられていると設計対話が速い。

## 裁定フロー

Strata の機能追加・仕様変更は、思いつきでコードに落とさない。以下の順で進む:

```
Issue 起票
   │
   ▼
設計対話(Issue 上、または対話セッション)で論点を詰める
   │
   ▼
docs/sml-spec.md §1 に "Dn" として裁定を凍結(論点と結論を1行〜数行で記録)
   │
   ▼
実装(コード + 必要ならテスト + ハンドオフドキュメント)
   │
   ▼
コミット・PR で Issue を Closes #n
```

- **Dn を経ずに仕様上の挙動を変えない**。バグ修正(既存の Dn 通りに動いていない
  箇所を直す)は例外だが、それも「これは仕様通りかバグか」の判断自体が裁定を
  要することがある。迷ったら Issue で確認してから着手する。
- 1つの Issue が複数の Dn に分解されることもある(例: `sml-spec.md` §1.17 の
  D58 は Issue #1 から起票された)。
- マイルストーン単位の設計決定・実装の橋渡しは `docs/*-handoff.md` に記録する
  慣習がある。大きめの変更はハンドオフドキュメントを添えることが望ましい。

## AI エージェントで開発する場合

Strata 自体の開発(このリポジトリのコード・仕様を書き換える作業)を AI
エージェントで行う場合は `AGENTS.md` のルールに従うこと(commit/push/merge は
指示なしに実行しない、テスト時は実装を触らない/実装時はテストを触らない、
設計書は `docs/` に置く、等)。

SML **文書**(`.sml` ファイルの中身)を AI エージェントに書かせる場合は
`docs/sml-agent-guide.md` が正典。これは Strata というプロジェクト自体の開発
作法ではなく、Strata フォーマットの上で文書を執筆する AI エージェント向けの
利用者側ガイド(ULID を自分で発行しない、確信のあるエッジだけ張る、
`strata fmt`/`strata build` で必ず検証する、等)。両者は役割が異なるので
混同しないこと。

## コミット規約

日本語の conventional commits 風スタイルを使う: `<type>: <要約>` に加えて、
関連する設計決定番号(Dn)や補足を em dash(`—`)で続けることが多い。

```
feat: strata search 実装 — 検索ライブラリ+CLI(D56)とパレット裁定(D57)
fix: rows: contains に class フィルタを追加(D58)
docs: Milestone 4 (strata render) の設計決定 D18〜D22 と実装ハンドオフを追加
chore: devshell に claude-code (llm-agents.nix) を追加、.claude/ を gitignore
```

主な `type`: `feat`(新機能・仕様拡張)/ `fix`(バグ修正)/ `docs`(ドキュメントのみ)/
`chore`(ビルド・開発環境等)。要約は日本語、命令形よりは体言止め・「〜実装」
「〜追加」のような名詞的な言い切りが多い。本文(2行目以降)は省略されることも
多いが、裁定の背景が非自明な場合は箇条書きで補足するとよい。

`git commit` はユーザーの明示的な指示があるときのみ実行する(AI エージェントが
自発的にコミットしない。`AGENTS.md` 参照)。
