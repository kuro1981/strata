# Strata 設計対話ログ

新しい文書フォーマット「Strata」を設計した対話の時系列記録。

---

## 1. 問題提起

人志向ドキュメントと機械志向ドキュメントが両方必要な時代。Markdown は人志向(可読性重視・難解さを制限)。機械志向はどうあるべきか。要件は「人の世界のドキュメントの複雑さ(数式、多段結合表など)を機械で読め、適切な parser を通せば人が読める」。当初の仮説は「mdではなく html か、隠蔽された html か?」。

応答の骨子: 「人 vs 機械」の対立ではなく「単一ソースから両方を生成できる構造」が要る。Markdown も HTML も *表現* のレイヤーで *構造* ではない。意味モデル → parser → 人/機械 という分離。「隠蔽された html」は逆向きで、HTML はプレゼンテーション寄り。機械が読みたいのは意味が明示された構造化層(MathML が良い先例)。既存近傍: AsciiDoc / reStructuredText / Org-mode / Typst / Pandoc AST。

## 2. 複雑さの厳密化(選択: 複雑さの定義を厳密化する)

複雑さを次元に分解:
- 軸1 参照構造(線形 / 木 / DAG / 一般グラフ)
- 軸2 局所表記(プレーン / 装飾 / 再帰木 / 入れ子コンテナ)
- 軸3 レイアウト束ね(結合セルは意味か装飾か = 情報欠損)
- 軸4 文脈依存(指示語・スコープ)

各次元に「機械可読」と「人へ戻す」を当てて要件を導出。可換性(往復で意味が保存されるか)。レイアウト束ねと文脈依存は不可逆になりやすい。

## 3. 組版・画像の追加(問い: 見栄えや image の扱いは?)

第5・第6の軸を追加:
- 軸5 連続実体(画像): 意味が画素にある写真 / 記号にある図。記号を画素に焼くのは意味欠損。
- 軸6 物理レイアウト: 意味モデルの *関数*。`表示 = render(意味モデル, 面の制約, スタイル)`。意味層に入れてはいけない。

3グループ化: 意味層に入れる(1,2)/入れたいが欠損しがち(3,5一部)/入れてはいけない(6, 5写真)。

## 4. 階級マップ + 物理分離(両方やる)

各軸を順序数的に階級化し、Markdown / HTML / AsciiDoc / Org / Typst / MathML / 理想 をマッピング。結論: 軸3を素で登れる形式がほぼ無い(HTML+RDFa のみ)。軸6は理想列だけ「不在」が正解。Markdown は6軸中5軸で L0〜L1。

物理分離の設計: 物理は意味の関数。侵入A(物理が意味に上がる)は語彙から構文的排除で塞ぐ。侵入B(指示語)は ID 参照のみ許し表示時に再縮約。三層に割る(content / style / binding)。音声出力で軸6が消滅 = 物理は意味の外、の証明。

## 5. 未踏地: canonical 記法の設計(選択: 未踏地へ)

軸3の核心: 表をグリッド+colspan で持つのをやめ、**次元の木**(MultiIndex)で持つ。結合プリミティブが消え「意味か装飾か」が問えなくなる。

最小コアモデル: Node(id, type, payload) + 型付き Edge(contains, defines, supports, depends-on, cites...)。インラインは payload、ブロックは Node。

ドキュメント = グラフへのビュー。トランスクルージョン自明(複数親 contains)。

払い出された報酬: 実体は2つでなく3つ(オーサリング / canonical / render)。「隠蔽された HTML」は canonical と render の分離を指していた。Markdown の敗因は3役兼任。

## 6. 新しい表現として作る(意思: 既存の枠に入れない)

Obsidian は「ファイルのグラフ」、作るべきは「ノードのグラフ」。フォーマット = canonical データモデル = Node/Edge の2レコード。ドキュメントはビュー。トランスクルージョンは単一ノード+複数参照。ストアはグラフ(ファイルは射影)。Obsidian クローンだが中身のデータの持ち方が根本的に違う。

## 7. モチベーションの記述

仕様冒頭にモチベーション節を追加(問題 / Markdown 診断 / 二項でなく三者 / なぜ新規に作るか / 成功の姿 / 非目標)。

## 8. 粒度: レベル1 + 昇格(MathML はサブセット採用)

数式は MathML サブセットを canonical、TeX をオーサリング表面に。

粒度の3レベル: L0 ノート(Obsidian)/ L1 段落=ノード(Roam, §2.3 の既定)/ L2 全スパン=ノード(超細粒度)。L2 はノード数10〜30倍 + 編集地獄(テキストノード分割)。

解決: レベル1既定 + 需要駆動の昇格。identity が要るスパンだけ `anchor` ノードへ昇格。term/table/math は元から昇格済み。granularity は動的な量。

## 9. 先行研究調査(2ラウンド)

系統: A コンテンツ/プレゼン分離・シングルソース出版(DITA/DocBook)/ B Xanadu・トランスクルージョン(ID安定性の解 = 局所名+不変leaf)/ C OHCO論争・Text-as-Graph(木の反証, ハイパーグラフ)/ D Core Calculus for Documents(render の落とし穴)/ E AST としてのマークアップ(matklad)/ F 現代実装(MyST, PreTeXt, Roam/Logseq/Tana)/ G GraphRAG(事後抽出 vs ネイティブ)。

PreTeXt は単一ソース → PDF/EPUB/アクセシブルHTML/点字/触図 を現実に出している最重要先例。MyST は AST を .json 公開、横断参照。アクセシビリティ研究が次元木テーブルの必要性を裏づけ。

## 10. 木モデルとの精密な差分 + 新規性

MyST/PreTeXt は OHCO(単一階層の木 + IDREF 上乗せ)。Strata は Text-as-Graph。足す4点: (1) 背骨がグラフ(複数親 contains)、(2) エッジが意味(supports/depends-on)、(3) 表が次元の木、(4) 数式が MathML 木(LaTeX 文字列でない)。逆に彼らにあるもの: 成熟生態系・実働点字出力・計算実行・大規模実証。

モチベーションに新規性5点を明記。

## 11. 型起こし(Rust)

strata-core を Rust の型に。NodeId(ULID)、Node{id, payload(enum)}、Edge{from,to,rel,ord}、Inline、Table(DimTree/Dim/Member/Cell)、MathNode、Figure。物理排除を型で強制(該当バリアントが無い)。不変条件チェック(ダングリング/contains閉路)。cargo test 4本パス・警告ゼロ。
炙り出された未決点: contains の ord 必須化、Cell path の葉到達検証、Chart.encode の型結合。

## 12. ストア再設計(問い: SQL前提? 持ち運び微妙)

SQL/DuckDB を source of truth にするのが問題(DB を変えても同じ)。store(プレーンテキスト vault)と index(派生・使い捨て)を分離。Obsidian/git/イベントソーシングと同じパターン。トランスクルージョン: ノードは1ファイルに住み他は ID 参照。粒度はドキュメント単位ファイル + ID 埋め込みに確定。

index は当面 DB なし(インメモリ Graph 再構築)→ 必要なら redb。データはグラフだがクエリは1〜2ホップ → グラフDB不要(メモリ上 graph 指向、ディスクはフラット)。Qdrant は後回し。serde は全フェーズの土台(redb とは層が違う)。

## 13. パーサ実装(TeX → MathNode)

Pratt パーサ。Lexer + Parser + normalize。対応: \frac, _ / ^ (SubSup 畳み), \sqrt{}/\sqrt[n]{}, \sum/\prod/\int, \text{}, \left()\right(), ギリシャ文字, 名前付き演算子。大型演算子の Sub/Sup を UnderOver に正規化(TeX の見た目 vs MathML の意味のズレを吸収)。未知綴りは UnknownCommand で明示(§6 の「出たら足す」合図)。cargo test 14本パス・警告ゼロ。実数式デモ確認。

失敗した1テストは期待エラー型の誤り(\frac{a} は ExpectedGroup が正)→ エラーを enum で型付けした配当。

## 14. アーカイブ + git 化(現在地)

各コードと対話を Google Drive に保存。Claude Code + git 管理への中間地点。
