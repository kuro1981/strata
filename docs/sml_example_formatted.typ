// Strata Document - Generated Typst Source
#set document(title: "機械学習モデルの評価レポート")

#set page(
  paper: "a4",
  margin: (x: 2.5cm, y: 2.5cm),
)
#set text(
  font: ("Libertinus Serif", "Noto Sans CJK JP", "IPAexMincho"),
  size: 10pt,
  lang: "ja",
)
#set par(
  justify: true,
  leading: 0.65em,
)
// D22: table/math/figure のみ自動番号付けの対象。math.equation の numbering を
// 有効にすると、ブロック数式(display 形)にだけ番号が振られる(インライン数式は
// Typst が非 display と判定するため番号は付かない)。
#set math.equation(numbering: "(1)")

// スタイル定義
#show heading: set text(fill: rgb("#2b3a42"))
#show heading.where(level: 1): it => {
  v(1em)
  align(center, text(size: 20pt, weight: "bold")[#it.body])
  v(0.5em)
}
#show heading.where(level: 2): it => {
  v(0.8em)
  block(
    width: 100%,
    stroke: (bottom: 1pt + rgb("#dddddd")),
    inset: (bottom: 0.5em),
    text(size: 14pt, weight: "bold")[#it.body]
  )
  v(0.3em)
}

= 機械学習モデルの評価レポート <01J2T8Z1000000000000000000>

== 1. 導入 <01J2T8Z2000000000000000000>

本レポートでは、新たに開発した 予測モデル の性能評価結果について報告する。
評価にあたっては、以下の2つを主要な評価指標（メトリクス）として採用した。 <01J2T8Z3000000000000000000>

#block[
- 予測精度 — F1スコアを基準とする <01J2T8Z5000000000000000000>
- 推論速度 — 推論1回あたりのレイテンシで測る <01J2T8Z6000000000000000000>
] <01J2T8Z4000000000000000000>

予測精度はモデルの実用性を担保するために最も重要な指標であり、今回はF1スコアを基準とする。 <01J2T8Z7000000000000000000>

== 2. 評価結果（多次元表） <01J2T8Z8000000000000000000>

実験における各モデルの評価結果は以下の通りである。 <01J2T8Z9000000000000000000>

#figure(
  table(
    columns: (auto, 1fr, 1fr, 1fr, 1fr),
    stroke: 0.5pt + luma(150),
    fill: (x, y) => if y < 2 or x < 1 { rgb("#f7f9fa") } else { none },
    table.cell(colspan: 1, rowspan: 2)[],
    table.cell(colspan: 2)[*Dataset-A*],
    table.cell(colspan: 2)[*Dataset-B*],
    table.cell[*F1-Score*],
    table.cell[*Latency*],
    table.cell[*F1-Score*],
    table.cell[*Latency*],
    table.cell[*Baseline-v1*],
    [0.82],
    [45 ms],
    [0.78],
    [50 ms],
    table.cell[*Opt-v2*],
    [0.91],
    [12 ms],
    [0.88],
    [15 ms],
  ),
  caption: [モデル別・データセット別の性能比較]
) <01J2T8ZA000000000000000000>

== 3. 分析と考察 <01J2T8ZB000000000000000000>

#link(<01J2T8ZA000000000000000000>)[評価結果の表] から明らかなように、`Opt-v2` は `Baseline-v1` と比較して大幅な性能向上を達成している。 <01J2T8ZC000000000000000000>

特に、#link(<01J2T8ZA000000000000000000>)[Dataset-A における Opt-v2 のレイテンシ (Opt-v2, Dataset-A.Latency)] は *12 ms* であり、Baseline の 45 ms から約73%の高速化を実現している。行列積の並列パイプラインが効率的に機能しているためである。 <01J2T8ZD000000000000000000>

推論の数学的ボトルネックは、以下の損失関数の計算部分であった。 <01J2T8ZE000000000000000000>

$ L = (1) / (N) (sum)_(i = 1)^(N) ( (y)_(i) - ((y)^(\^))_(i) ())^(2) $ <01J2T8ZF000000000000000000>

この #link(<01J2T8ZF000000000000000000>)[損失関数] の計算をGPU向けにカーネル最適化したことが、`Opt-v2` の速度向上の最大の要因である。 <01J2T8ZG000000000000000000>

#figure(
  box(width: 100%, height: 4cm, stroke: 0.5pt + luma(150))[
    #align(center + horizon)[
      チャート(プレースホルダ) #linebreak()
      Baseline-v1 と Opt-v2 の Dataset-A における F1 スコアの比較。Baseline-v1 が 0.82 であるのに対し、Opt-v2 は 0.91 へと向上していることを示す棒グラフ。 #linebreak()
      データ: @01J2T8ZA000000000000000000 #linebreak()
      bar: model × Dataset-A.F1-Score
    ]
  ],
  caption: [モデルごとの予測精度（F1-Score）比較]
) <01J2T8ZH000000000000000000>

