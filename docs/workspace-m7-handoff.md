# M7 実装ハンドオフ — ワークスペース層 v0(D41〜D43)

sml-spec §1.10(2026-07-15 確定)の実装指示。ファイル横断の実害3件の解消が
そのまま受け入れ基準。

## 必読(この順)

1. `AGENTS.md` — **git commit/push はユーザー指示なしに絶対しない**(両リポジトリ)
2. `docs/sml-spec.md` §1.10(D41〜D43)・§2.1(フロントマター alias)・§10
3. `crates/strata-build/src/{lib,resolve,convert}.rs`(拡張の本丸)、
   `crates/strata-sml`(フロントマター・参照ターゲットのパース)、
   `crates/strata-view`(複数文書入力)、`crates/strata-cli`
4. `docs/view-def-v1.md`(セレクタの doc 修飾を追記する対象)

## スコープ境界(やらないこと)

- context / render の横断(v0.5、§10)— ただし「cross-doc 参照を含む文書の
  単一ファイル render が Error になる」ことの**明確な案内メッセージ**は本 v0 の責務
- index の永続化・インクリメンタル(§10 保留)
- strata-html(凍結)・fixture 改版・fmt の変更(D43: fmt は不変。ただし
  フロントマター `alias` キーの受理だけはパーサ側の追加として必要)

## 作業パッケージ

### WP-W1: フロントマター alias と横断参照の記法(D41/D42)

1. フロントマター許可キーに `alias`(key 字句)を追加(§2.1 改定済み)。
   Document ノードに alias を格納(D26 の Node.alias に乗る)
2. 参照ターゲットの `<文書alias>/<ブロックalias>` 形式を全スキーム
   (`ref:`/`table:`/`fig:`/`math:`/`cell:`、属性行の supports= 等)で受理。
   `/` は alias 字句(`[A-Za-z0-9_-]+`)に含まれないため曖昧さなし。
   無修飾 alias = 同一文書(従来どおり)。ULID は無修飾のままワークスペース全域
3. **単一ファイル build** で doc 修飾参照に遭遇した場合: 専用の Error
   (例: `CrossDocRef: 参照 'work-history/...' はワークスペース build
   (--workspace)が必要です`)。黙って落とさない(D40 の教訓)

### WP-W2: `strata build --workspace`(D43)

1. `strata.toml` のパース: `[workspace] members = ["a.sml", "sml/*.sml"]`
   (strata.toml からの相対パス、グロブ可)。TOML クレートの選定は裁量
2. 全メンバーをパース → **インメモリ index(ID/alias → ノード表)** →
   横断参照を解決した**単一の統合グラフ**を出力
   (`strata build --workspace <strata.toml> [-o out.json]`)
3. 診断(全か無か・全ファイル集約。ファイル名を診断表示に含める —
   従来の「行:列:」に `ファイル:` を前置する等、体裁裁量):
   - 文書 alias の重複(2文書が同じ alias)
   - **ファイル間 ULID 衝突**(コピペ事故)
   - doc 修飾の未解決(文書 alias 不明 / ブロック alias 不明を区別)
   - メンバーに文書 alias が無いのはエラーではない(その文書へは ULID 参照のみ可)
4. Term ノードは D9 の安定 ID により自然合流(重複挿入にならないこと)。
   出力 JSON の形(roots の表現等)は裁量・報告
5. 統合グラフの `invariants::validate` 通過

### WP-W3: view の複数文書入力(D43)

1. `strata view --workspace <strata.toml> --view <def.yaml> [-o] [--profile] [--check]`
   (従来の単一ファイル引数も不変で残す)
2. セレクタに文書スコープを追加: 例 `{ alias: licenses, doc: resume }`
   (糖衣 `resume/basic-info.氏名` の拡張も可 — `/` は alias に出ないので安全。
   形は裁量・報告)。無指定 doc の解決規則(単一文書モード=当該文書、
   ワークスペースモードでの無指定の扱い — 一意なら解決/曖昧ならエラー等)は
   裁量・報告
3. `docs/view-def-v1.md` に doc スコープの節を追記(批准済み文法への追加なので
   変更点を明示)

### WP-W4: ドッグフーディング(受け入れ基準=実害3件の解消)

`~/dev/strata-my-resume/sml/` にて(書き込みは sml/ 配下のみ・git 禁止):

1. `sml/strata.toml` を作成(members: resume.sml, work_history.sml)。
   両文書のフロントマターに alias(`resume` / `work-history`)を付与
2. **実害2の解消**: work_history.sml の複製 record `cv-basic-info` を削除し、
   cv-jis.view.yaml を workspace モード+doc スコープで resume の basic-info
   から直接引く形に書き換え
3. **実害3の解消**: resume-jis.view.yaml が越境宣言していた
   `build_cv/content/licenses.yaml` を cv-jis.view.yaml 側へ移す
   (doc: resume で licenses 表を引く)。マニフェストの注記も更新
4. **実害1の解消**: resume.sml の「職務経歴の詳細は work_history.sml(…)を参照」
   というテキスト妥協段落を、本物の横断参照
   (例: `[職務経歴書](ref:work-history/<適切なブロックalias>)`)に置換
5. 検証: fmt 冪等(両文書)→ `build --workspace` 診断ゼロ →
   `view --workspace --check` ゼロ → content YAML 再生成で従来と同値
   (basic-info 由来・licenses 由来のフィールドが正しいこと)→ JIS PDF 再生成
6. **予見される副作用の確認と報告**: 実害1の参照を入れた resume.sml は
   単一ファイル build が CrossDocRef Error になるはず。その結果
   `render`(typst/md)単体が通らなくなる — これが仕様どおりの挙動であることを
   確認し、v0.5(render --workspace)の必要性として報告(resume.md /
   resume_jis の再生成可否も整理して報告)

## 完了の定義

- `cargo test --workspace` 全通過(workspace build・doc 修飾参照・ULID 衝突・
  view doc スコープ・CrossDocRef の各テスト込み)、clippy 新規警告ゼロ
  (strata-html 既存分対象外)
- 既存の単一ファイルフロー(fmt/build/render/view/context)の非退行
  (fixture・ゴールデン無変更)
- WP-W4 の実害3件解消と副作用報告
- **コミットはしない**。変更ファイル・テスト消化・裁量箇所(strata.toml の形・
  診断のファイル名表示・view の doc 無指定規則・糖衣拡張・roots 表現)を
  まとめて終了
