// rel → 色・線種のマッピング(グラフペインのオーバーレイ・凡例)。
// 既知 rel は固定の色/線種を割り当てる。未知 rel(将来追加されるもの)は文字列から
// 決定的にハッシュした色を割り当てる — 「未知 rel も落ちず描画できる」汎用実装(D49)。

import type { KnownRel } from "@/types/graph";

export interface RelStyle {
  color: string;
  dash?: string;
}

const KNOWN_STYLE: Record<KnownRel, RelStyle> = {
  // outline/overview では構造は位置で表現するため使わないが、local グラフは G1.6 で
  // contains の隣接(親・子・兄弟)も描くようになったため、薄いグレー破線として使う
  // (意味エッジと視覚的に区別するための裁量配色)。
  contains: { color: "#cbd5e1", dash: "3 3" },
  supports: { color: "#16a34a" }, // 緑: 論拠
  "depends-on": { color: "#2563eb" }, // 青: 依存
  "refers-to": { color: "#f59e0b", dash: "4 3" }, // 橙・破線: 弱いナビゲーション参照
  revises: { color: "#dc2626", dash: "1 3" }, // 赤・点線: 改定・追認(D48)
  "term-ref": { color: "#7c3aed" }, // 紫: 用語使用
  defines: { color: "#0891b2" }, // シアン: 定義
  cites: { color: "#a16207" }, // 褐色: 引用
  "instance-of": { color: "#64748b" }, // グレー: 型付け
};

// 未知 rel 用のフォールバック配色プール(識別しやすい彩度高めの色を数色用意)。
const FALLBACK_POOL = ["#db2777", "#059669", "#9333ea", "#ea580c", "#0d9488", "#4338ca"];

function hashString(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) {
    h = (h * 31 + s.charCodeAt(i)) >>> 0;
  }
  return h;
}

export function relStyle(rel: string): RelStyle {
  const known = KNOWN_STYLE[rel as KnownRel];
  if (known) return known;
  const idx = hashString(rel) % FALLBACK_POOL.length;
  return { color: FALLBACK_POOL[idx], dash: "2 2" };
}
