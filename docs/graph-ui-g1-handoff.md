# G1 実装ハンドオフ — グラフ UI コア(D48〜D50)

sml-spec §1.13(2026-07-15 確定)の実装指示。2ワークストリーム:
**WS-A(revises rel、Rust 小)** と **WS-B(UI コア、フロントエンド大)**。
並列実行可(触るファイルが分離: A=strata-{core,sml,build,context}+decisions.sml、
B=ui/ 新設+strata-cli の site サブコマンド+flake.nix devshell)。

## 共通ルール

- **git commit/push はしない**。~/dev/strata-my-resume は読み取り+CLI 実行のみ
- fixture(docs/sml_example_*.sml)改版禁止。ゴールデン変更は意図的更新のみ・報告
- 曖昧な点は裁量として最終報告に明記

---

## WS-A: rel `revises`(D48)

必読: sml-spec §1.13(D48)・§4.1、docs/sml-agent-guide.md、M8 報告の境界事例
(docs/spec-sml-m8-handoff.md と decisions.sml)。

1. `strata-core`: `Rel::Revises` 追加(serde 後方互換に注意)
2. `strata-sml`: 属性行キー `revises=` を既知キーに追加(supports 等と同列)
3. `strata-build`: エッジ materialise(既存 rel と同経路)
4. `strata-context`: エッジ一覧・近傍表示に revises を含める(表示名 `revises`)
5. `strata-typst` / `strata-md`: **描画しない**(supports 等と同じ「紙面に出さない
   意味エッジ」— 変更不要のはずだが、未知 rel で落ちないことを確認)
6. **decisions.sml への適用**: M8 の境界事例のうち「時間発展」6件に `revises` を
   張る — d14→d8, d14→d13(追認)/ d46→d23(明文化)/ d40→d39(実装裁定)/
   d35→d32 は既に depends-on 済みなので**張り替えない**(D35 の「再裁定」は
   d32 の運用の適用であり依拠が主 — 裁量で判断・報告)/ d47→d37(確信原則への
   依拠 — depends-on か revises か裁量・報告)/ D41→d29・d43→d9(一貫・機構
   再利用)は**引き続き張らない**(時間発展でも依拠でもない)。
   1件ごとに判断理由を報告
7. テスト: パース・エッジ生成・context 表示。fmt 冪等維持
8. 検証: `build --workspace docs/spec-sml/strata.toml` 診断ゼロ、
   `context --node d23 --hops 1` に revises が現れること

## WS-B: UI コア(D49/D50)

### 環境

- `flake.nix` の devshell に Node.js(LTS)+ pnpm を追加してよい(変更内容を報告)
- `ui/` を新設(pnpm + Vite + React + TypeScript + Tailwind + shadcn/ui)。
  **CDN・外部フォント・外部 API への実行時依存ゼロ**(ビルド成果物は自己完結の
  静的資産。フォントはシステムフォントスタックで可)

### 入力データ

- `strata build --workspace` の graph JSON(nodes / edges / roots / doc_aliases)。
  UI は `graph.json` を fetch する SPA(D50: 静的=配布形態、実行は動的)
- スキーマは実物から読む(`build --workspace docs/spec-sml/strata.toml -o ...` で
  生成して確認)。TypeScript 型は手書きで最小限に(自動生成はしない)

### v0 の体験(D49)— 2ペインの往復

1. **文書ペイン(右)**: グラフから文書を描画(Document → contains 順)。
   見出し・段落・リスト(ネスト・タスク)・record(2列表)・多次元表・コード・
   引用・数式(v0 は TeX 文字列の等幅表示でよい — KaTeX 等の外部依存は入れない。
   裁量で妥協点を報告)・class バッジ表示。各ブロックに anchor
2. **グラフペイン(左)**: ノード=ブロック(ラベルは見出し/先頭テキストの短縮)。
   v0 レイアウトは**力学ではなく構造ベース**(contains 階層で縦に並べ、
   意味エッジ(supports/depends-on/refers-to/revises)を曲線オーバーレイ。
   rel ごとに色/線種を分け、凡例を置く)。ズーム/パン
3. **同期**: グラフのノードクリック → 文書ペインが該当ブロックへスクロール+
   ハイライト。文書ペインのブロッククリック(またはフォーカス)→ グラフ側で
   当該ノード+接続エッジがハイライト。**ブロック脇に関係パネル**
   (in/out のエッジ一覧。クリックで相手へジャンプ)
4. **class トグル**: 存在する class の一覧をチップ表示、OFF にすると該当
   サブツリー(実効 class、D46)が両ペインから消える(--hide のインタラクティブ版)
5. **ジャンプ**: alias / ULID の入力でノードへ(検索は v1 なので完全一致でよい)
6. 複数文書(ワークスペース): 文書ペインは文書タブ or 連結表示(裁量)、
   グラフは統合グラフ(文書境界を視覚的に示す)

### CLI 統合

- `strata site --workspace <strata.toml> -o <outdir>`(単一ファイル版
  `strata site <file.sml>` も可・裁量): 内部 build → `graph.json` 書き出し →
  ビルド済み UI 資産(`ui/dist`)を合成。`ui/dist` が無ければ
  「`pnpm build` を先に」の明確なエラー
- `ui/dist` は git 管理しない(.gitignore 追記)

### 検証

- decisions ワークスペース(51 裁定+エッジ)と履歴書ワークスペース
  (~/dev/strata-my-resume/sml/strata.toml)の両方で `strata site` を実行し、
  ブラウザなしでの確認として: 生成物の構成、graph.json の整合、
  `pnpm test`(コンポーネント単体があれば)+ `pnpm build` 成功
- 可能なら headless での smoke(裁量。無理なら人間の目視評価に委ねる旨を報告)
- **ユーザーの目視評価が最終ゲート**: 起動手順(例: `python3 -m http.server` や
  `npx serve` 等、自己完結ディレクトリの開き方)を報告に明記

### 最終報告(WS-B)

- ui/ の構成と依存一覧(shadcn の採用コンポーネント)
- 2ペイン同期・関係パネル・class トグルの実装方式の要約
- 裁量箇所(レイアウトアルゴリズム・数式の妥協・タブ vs 連結・graph.json スキーマの
  TS 型)
- 起動手順、既知の制限、G2(Tauri)への持ち越し事項
