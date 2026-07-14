---
id: 01J2T8Z0000000000000000000
---

# 機械学習モデルの評価レポート {#01J2T8Z1000000000000000000}

## 1. 導入 {#01J2T8Z2000000000000000000}

[id=01J2T8Z3000000000000000000]
本レポートでは、新たに開発した [予測モデル](term:予測モデル) の性能評価結果について報告する。
評価にあたっては、以下の2つを主要な評価指標（メトリクス）として採用した。

[id=01J2T8Z4000000000000000000]
- [予測精度](term:予測精度) — F1スコアを基準とする {#01J2T8Z5000000000000000000}
- [推論速度](term:推論速度) — 推論1回あたりのレイテンシで測る {#01J2T8Z6000000000000000000}

[id=01J2T8Z7000000000000000000, supports=term:予測精度]
予測精度はモデルの実用性を担保するために最も重要な指標であり、今回はF1スコアを基準とする。

## 2. 評価結果（多次元表） {#01J2T8Z8000000000000000000}

[id=01J2T8Z9000000000000000000]
実験における各モデルの評価結果は以下の通りである。

::table {#01J2T8ZA000000000000000000 alias=eval-table}
[caption="モデル別・データセット別の性能比較"]

# 行軸: 実験対象のモデル（フラット次元は [...] 糖衣で書ける）
@rows:
  - model: [Baseline-v1, Opt-v2]

# 列軸: データセット × メトリクス（ネスト次元）
@cols:
  - dataset:
    - Dataset-A:
      - metric: [F1-Score, Latency]
    - Dataset-B:
      - metric: [F1-Score, Latency]

# セル値（行パス | 列パス : 値）。"45 ms" は数量（値 45 / 単位 ms）としてパースされる
@cells:
  Baseline-v1 | Dataset-A.F1-Score : 0.82
  Baseline-v1 | Dataset-A.Latency  : 45 ms
  Baseline-v1 | Dataset-B.F1-Score : 0.78
  Baseline-v1 | Dataset-B.Latency  : 50 ms

  Opt-v2      | Dataset-A.F1-Score : 0.91
  Opt-v2      | Dataset-A.Latency  : 12 ms
  Opt-v2      | Dataset-B.F1-Score : 0.88
  Opt-v2      | Dataset-B.Latency  : 15 ms
::

## 3. 分析と考察 {#01J2T8ZB000000000000000000}

[id=01J2T8ZC000000000000000000]
[評価結果の表](table:eval-table) から明らかなように、`Opt-v2` は `Baseline-v1` と比較して大幅な性能向上を達成している。

[id=01J2T8ZD000000000000000000, supports=eval-table]
特に、[Dataset-A における Opt-v2 のレイテンシ](cell:eval-table#Opt-v2|Dataset-A.Latency) は **12 ms** であり、Baseline の 45 ms から約73%の高速化を実現している。行列積の並列パイプラインが効率的に機能しているためである。

[id=01J2T8ZE000000000000000000]
推論の数学的ボトルネックは、以下の損失関数の計算部分であった。

::math {#01J2T8ZF000000000000000000 alias=loss-formula}
L = \frac{1}{N} \sum_{i=1}^{N} (y_i - \hat{y}_i)^2
::

[id=01J2T8ZG000000000000000000, depends-on=loss-formula]
この [損失関数](math:loss-formula) の計算をGPU向けにカーネル最適化したことが、`Opt-v2` の速度向上の最大の要因である。

::figure {#01J2T8ZH000000000000000000 alias=perf-chart}
[kind=chart, data-ref=eval-table, mark=bar]
[encode-x="model", encode-y="Dataset-A.F1-Score"]
[depicts="Baseline-v1 と Opt-v2 の Dataset-A における F1 スコアの比較。Baseline-v1 が 0.82 であるのに対し、Opt-v2 は 0.91 へと向上していることを示す棒グラフ。"]
[caption="モデルごとの予測精度（F1-Score）比較"]
::
