# 画像対応ハンドオフ(Phase A: strata本体)

## 背景

vault統合(kurochanBrainPrivate等)の実機ドッグフーディングで、画像が全く表示されない
ことが判明した。調査の結果、原因は独立した複数件:

1. Obsidianのローカル添付embed構文 `![[file.png]]` は strata-sml のパーサが未対応
   (D59時点で意図的にスコープ外、§10保留)。診断ゼロで無害なリテラルテキストに
   なるだけ(「静かに壊れる」パターン)。移行済み198文書中17件がこの構文を使用。
2. 元vaultには実際に79件の画像ファイルが存在するが、移行時にバイナリをコピーする
   手順が無く、strata-notes側には画像ファイルが1件も無い。
3. **Typstレンダラの画像出力は最初からプレースホルダ実装**
   (`crates/strata-typst/src/lib.rs` の `render_image`)。alt/srcをテキスト表示する
   箱を描くだけで、実際に `#image()` を呼んで画像を埋め込んだことが一度もない。
4. リモートURL画像(`![alt](https://...)`、移行データ中74箇所)はパーサ・core・
   ui/(`BlockTree.tsx` の `<img src={node.src}>`)いずれも対応コードがあるが、
   実機で本当に表示されているかは未検証。

ユーザー裁定: 「実弾投入するためには画像対応必須」。ui/(strata-editorが使う
ビューア)・Typst出力、両方を今回のスコープに含める。

このハンドオフは **Phase A(strata本体のみ)**。strata-editor 側の Tauri 対応と
obsidian-import-skill の添付コピー手順追加は別ハンドオフ(Phase B、Phase A 完了・
検証後に着手)。

## 設計(推奨、裁量の余地は各項目に明記)

### 1. ワークスペースへの `assets` グロブ宣言

`strata.toml` に新セクション/フィールドを追加: `assets = ["assets/**/*"]` のような
明示的globでアセットファイルを列挙する(D41「スキャンしない」原則を踏襲。
member との違いは「.smlではなくバイナリファイルを列挙するglob」という点のみ)。

- 認識拡張子: `png`/`jpg`/`jpeg`/`gif`/`svg`/`webp`(大文字小文字は許容、小文字化
  して比較)。それ以外の拡張子がglobにマッチしても無視してよい(将来の非画像
  添付は今回スコープ外)。
- workspace build時にこれらをスキャンし、basename(拡張子込み)→絶対パスの
  マップを構築する。
- 同名ファイルが複数箇所にマッチしたら曖昧(後述の診断)。

### 2. `![[target]]` / `![[target|alt]]` 構文のパーサ対応

`crates/strata-sml/src/inline.rs` に、既存の wikilink 実装(D59、`try_parse_wikilink`
相当の関数)と並ぶ形で画像embedを追加する。

- 判定: `!` に直後続いて `[[` で始まり、対象名(`|` 前の部分)が上記の認識拡張子で
  終わっている場合のみ画像embedとして扱う。それ以外の `![[...]]`(非画像添付)は
  現状維持でスコープ外(リテラルテキストのまま、変更しない)。
- alt: `![[target|alt]]` の `|` 後ろ。省略時はファイル名(拡張子除く)を alt とする。
- 単一ファイルbuild文脈(ワークスペース無し)では解決不能なので、D59の
  `WikilinkNeedsWorkspace` と同型のエラーを新設(命名は裁量、例:
  `AssetNeedsWorkspace`)。
- ワークスペースbuild文脈で `assets` インデックスと突き合わせ、`ImageFigure` ノードを
  生成。未解決は `UnresolvedAsset`、曖昧(同名複数)は `AmbiguousAsset` 相当の
  エラー(D59のUnresolvedWikilink/AmbiguousWikilinkに倣った文言・扱い)。
- 実装場所は wikilink 解決ロジックと同じ layer(単一ファイルparse時点では
  「未解決の画像embed」を仮ノードとして持ち、workspace build時に解決する2段階
  構成のはず — 既存のwikilink実装のデータフローを確認してから着手すること)。

### 3. `ImageFigure.src` の意味を確定する

`src` は次のいずれかの文字列になる(呼び出し側は `starts_with("http")` で判別):

- リモートURL(既存の `![alt](url)` 経由、変更なし)
- **解決済みのローカル画像の絶対ファイルシステムパス**(今回追加。可搬性より
  実装の単純さを優先するv0の割り切り。将来ポータブルな相対パス化が要るなら
  §10 保留へ登録すること)

`strata-core` の `ImageFigure` 構造体自体は変更不要なはず(`src: String` のまま)。

### 4. Typstレンダラ: プレースホルダを実描画に置き換える

`crates/strata-typst/src/lib.rs` の `render_image` を書き換え、実際に
`#figure(image(local_path, ...), caption: [...])` を出力する。

- ローカル画像(絶対パス): そのまま `#image()` に渡してよい(Typstは絶対パスを
  受け付ける。要 `typst compile` 実地検証)。
- リモートURL画像: Typstは直接HTTPフェッチできないため、**render時に一時
  ディレクトリへダウンロードし、ローカルパスへ差し替えてから `#image()` に渡す**
  (`reqwest` 等の追加依存が要る可能性がある。ネットワーク不通・404等の失敗時は、
  既存のプレースホルダボックスへフォールバックしつつ warning 診断を積むこと —
  render は例外的失敗で全体を止めない、という既存方針を踏襲)。
- どちらの場合も alt はキャプションまたは代替テキストとして活かすこと(現状の
  プレースホルダ実装のalt表示を丸ごと消さない)。

### 5. MDレンダラ: 変更不要と判断してよい(要確認)

`crates/strata-md/src/lib.rs` の `render_image` は既に `![alt](src)` をそのまま
出力している。絶対パスでもURLでも大半のローカルMarkdownビューアは解決できるため、
**変更不要という判断でよい**(裁量、最終報告に明記すること)。

### 6. ui/(`BlockTree.tsx`)にsrc解決フックを追加する

ブラウザ/Tauri webview は絶対ファイルシステムパスを直接 `<img src>` に渡しても
読めない。`ui/src/state/GraphContext.tsx` に `resolveImageSrc: (src: string) => string`
を追加し(デフォルトは恒等関数)、`BlockTree.tsx` の `<img src={node.src}>` を
`<img src={resolveImageSrc(node.src)}>` に変更する。

- 既定実装(static サイト文脈): httpから始まるURLはそのまま。絶対パスは
  静的サイトでは解決できないため、既定では「画像なしプレースホルダ」を出す
  (壊れた `<img>` を出さない)。
- **strata-editor(Tauri)側は独自の `resolveImageSrc` を `GraphProvider` に渡して
  上書きする**(Phase B の作業。今回のPhase Aでは「差し替え可能なフック」を
  用意するところまでで良い。ui/自体にTauri依存を持ち込まないこと — D54の
  「ui/はエディタに依存しない」境界を守る)。

### 7. `strata site` CLI: 参照画像の実コピー

静的サイト出力では絶対パスを直接使えない。`crates/strata-cli/src/main.rs` の
`site` サブコマンド(既存の `copy_dir_recursive` — ui/dist コピーに使っている
ヘルパー、1483行目付近)を参考に:

- グラフ中の全 `ImageFigure.src` を走査し、絶対パスのものを `<output>/assets/`
  へ実コピーする(ファイル名衝突は連番等でリネーム、裁量)。
- 出力するgraph JSON中の該当 `src` を、コピー先からの相対パス
  (例: `assets/foo.png`)に書き換える(絶対パスのまま出力しない — 生成物を
  他マシンに配布したときに壊れるため)。
- リモートURLの `src` はそのまま(コピー不要)。

## 検証

- `crates/strata-sml` に回帰テスト追加: `![[foo.png]]` の単一ファイル文脈エラー・
  ワークスペース文脈での解決成功/未解決/曖昧の3パターン
- 最小再現ワークスペース(ダミーPNG数枚 + それを参照する `.sml`)を用意し:
  - `strata build --workspace` 診断ゼロ(正常系)
  - `strata render --format typst` → `typst compile` が実際に画像入りPDFを
    出力することを確認(プレースホルダでなく本物の画像が埋め込まれているか、
    `typst query` 等で検証)
  - `strata render --format md` の出力を確認
  - `strata site` の出力ディレクトリに画像が実コピーされ、graph JSON の src が
    相対パスに書き換わっていることを確認
- `cargo test --workspace` 全通過・clippy新規警告ゼロ(strata-html除く)
- `pnpm build`(ui/)・型チェック通過
- **fixture(`docs/sml_example_*`)は変更しない**(画像を使っていないはずなので
  影響しないはずだが、念のため確認)

## ルール

git操作禁止。曖昧な点(エラー種別の命名、リトライ/タイムアウト方針、ファイル名
衝突時のリネーム規則等)は裁量として最終報告に明記。既存の `HtmlNotSupported` 等の
「安全に諦める」設計思想(壊すより警告して継続)を踏襲すること。ネットワーク
アクセス(Typstのリモート画像ダウンロード)を追加する場合、失敗が全体のbuildや
render を止めないことを最優先に。

## 完了の定義

上記1〜7実装完了・検証項目全通過・コミットしない。

最終報告: 各項目の実装内容 / 新設した診断種別一覧 / Typstの画像ダウンロード
方式の詳細(依存クレート追加の有無含む) / `strata site` の実コピー方式 /
裁量箇所 / Phase B(strata-editor)へ引き継ぐべき `resolveImageSrc` インターフェース
の正確なシグネチャ
