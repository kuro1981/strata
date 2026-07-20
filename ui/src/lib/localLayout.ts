// ローカルグラフ(D51 G1.5、既定モード)。選択ノードを中心に意味エッジの近傍 1〜2 ホップを
// 力学シミュレーションなしで同心円状に配置する(Obsidian のローカルグラフ相当。
// v0 の「力学ではなく構造ベース」という方針(lib/layout.ts)をローカルグラフにも踏襲)。
//
// 配置方針: 中心 = 半径0。次に contains の隣接(親・子・兄弟)を常時表示のリングとして
// 内側に置き(hop トグルの対象外、G1.6 #1)、その外側に意味エッジの hop1・hop2 を同心円状に
// 配置する。hop2 は発見元の hop1 ノードの角度付近に扇状配置する(hop1 が複数の hop2 を
// 持つ場合に近傍関係が視覚的にわかるよう)。
//
// G1.6(ユーザー目視評価の是正): 各リングの半径・角度幅はノードのラベル幅(概算)に応じて
// 動的に配分する(等分割ではない)。これにより hop2 やノード数が多いリングでラベルが
// 重なりにくくなる(#2)。加えて contains の隣接を常時混ぜることで、意味エッジが少ない
// ノード(特に未選択時のルート)でも1ノードだけにならないようにする(#1)。
//
// G2.1(実機目視評価の是正 #1): G1.6 の「リング半径をラベル幅から逆算」だけでは
// 実データで不十分だった(中心ラベルがリングに被る、ring 間でラベルが重なる)。
// 追加で以下を行う:
// 1. アンカー切替: 各ノードのラベルは角度(x の符号)に応じて左右どちらに伸ばすかを
//    決める(左半分のノードは左側にラベルを伸ばす)。中心ノードは常に「上」に伸ばす
//    (どの角度のリングノードとも構造的に対称な位置)。
// 2. 衝突解決パス: 全ノード(中心含む)のラベル矩形を集め、AABB 重なりを検出したら
//    垂直方向に押し出す(反復数回で収束)。中心は固定点として動かさず、他のラベルを
//    押しのける側にする。結果は `labelSide`/`labelDy` として各ノードに持たせ、
//    描画側(GraphPane.tsx)はそれをそのまま使う(位置計算のロジックをここに一本化)。

import type { GraphIndex } from "./graphIndex";
import { deriveLabel, estimateLabelWidth, truncate } from "./label";
import { NODE_R } from "./layout";
import type { Edge, NodeId } from "@/types/graph";

export type LocalNodeKind = "center" | "semantic" | "contains";

/** G2.1: ラベルの水平アンカー側。中心ノードは常に "top"(上に伸ばす、リングのどの
 * 角度とも対称)。 */
export type LabelSide = "left" | "right" | "top";

export interface LocalNodePos {
  id: NodeId;
  x: number;
  y: number;
  /** 意味エッジの hop 数(0 = 中心, 1, 2)。`kind === "contains"` のノードは
   * hop トグルの対象外(常時表示)なので便宜上 1 を入れるが、描画側は `kind` で分岐する。 */
  hop: number;
  /** ノードの由来。"contains" = 親・子・兄弟(G1.6 #1、常時表示・視覚的に控えめ)。 */
  kind: LocalNodeKind;
  /** G2.1: ラベルをどちら向きに伸ばすか(衝突解決込みで localLayout 側が確定させる)。 */
  labelSide: LabelSide;
  /** G2.1: ラベル矩形の衝突解決で加えた追加の垂直オフセット(px)。既定 0。 */
  labelDy: number;
}

export interface LocalLayoutResult {
  center: NodeId;
  nodes: LocalNodePos[];
  edges: Edge[];
  /** 中心の contains 祖先(root → 中心の直前の親の順、中心自身は含まない)。
   * D51「contains の親パス(パンくず的な文脈)」の描画元データ。 */
  breadcrumb: NodeId[];
  /** SVG に必要な半径(パディング抜き)。ノードが無ければ 0。 */
  radius: number;
}

const HOP1_RADIUS_MIN = 120;
const HOP2_RADIUS_MIN = 220;
const CONTAINS_RADIUS_MIN = 60;

/** ノード円の後にラベルとの間・次ノードとの間に確保する隙間(px 相当)。 */
const LABEL_GAP = 16;

/** 兄弟・子が大量にいる場合の裁量上限(最終報告に既知の制限として明記)。
 * 子は文書順の先頭から、兄弟は中心に近い順(前後交互)に採用する。 */
const MAX_CONTAINS_CHILDREN = 12;
const MAX_SIBLINGS = 8;

/** 各リングでの表示上のラベル最大文字数(G1.6 #2: 強めの切り詰め。全文は hover の
 * <title> tooltip で見せる、GraphPane.tsx 側)。 */
export const MAX_LABEL_CONTAINS = 10;
export const MAX_LABEL_HOP1 = 18;
export const MAX_LABEL_HOP2 = 12;
/** 中心ノードのラベル最大文字数(GraphPane.tsx の描画側と共有。G2.1 でここに集約し、
 * 衝突解決の矩形計算にも同じ値を使う)。 */
export const MAX_LABEL_CENTER = 24;

/** ノード種別+hop からノード円の半径を決める(GraphPane.tsx の描画と同じ規則。
 * G2.1: 衝突解決の矩形計算にも使うのでここに一本化する)。 */
export function nodeRadiusFor(kind: LocalNodeKind, hop: number): number {
  return kind === "center" ? NODE_R + 4 : kind === "contains" ? NODE_R - 2 : hop === 1 ? NODE_R + 1 : NODE_R - 1;
}

/** ノード種別+hop からラベルの最大表示文字数を決める(GraphPane.tsx の描画と同じ規則)。 */
function maxLabelFor(kind: LocalNodeKind, hop: number): number {
  return kind === "center" ? MAX_LABEL_CENTER : kind === "contains" ? MAX_LABEL_CONTAINS : hop === 2 ? MAX_LABEL_HOP2 : MAX_LABEL_HOP1;
}

export function computeLocalLayout(
  idx: GraphIndex,
  center: NodeId,
  hops: 1 | 2,
  isVisible: (id: NodeId) => boolean,
): LocalLayoutResult {
  const breadcrumb: NodeId[] = [];
  {
    let cur = idx.parentOf.get(center);
    while (cur) {
      breadcrumb.unshift(cur);
      cur = idx.parentOf.get(cur);
    }
  }

  if (!isVisible(center) || !idx.nodes.has(center)) {
    return { center, nodes: [], edges: [], breadcrumb, radius: 0 };
  }

  // 無向 BFS(意味エッジの in/out どちらも辿る)で hop 数までの近傍集合を求める。
  // `discoveredVia` は角度配置のグルーピング(扇状配置)に使う「発見元の hop1 ノード」。
  const hopOf = new Map<NodeId, number>([[center, 0]]);
  const discoveredVia = new Map<NodeId, NodeId>();
  let frontier = [center];
  for (let h = 1; h <= hops; h++) {
    const next: NodeId[] = [];
    for (const id of frontier) {
      for (const n of neighborsOf(idx, id)) {
        if (!isVisible(n) || hopOf.has(n)) continue;
        hopOf.set(n, h);
        discoveredVia.set(n, h === 1 ? n : (discoveredVia.get(id) ?? id));
        next.push(n);
      }
    }
    frontier = next;
  }

  const cmp = cmpLabel(idx);
  const hop1 = [...hopOf.entries()].filter(([, h]) => h === 1).map(([id]) => id).sort(cmp);
  const hop2 = [...hopOf.entries()].filter(([, h]) => h === 2).map(([id]) => id);

  // --- G1.6 #1: contains の隣接(親・子・兄弟)を hop トグルとは無関係に常時含める。
  // 意味エッジが無い(または少ない)ノード(特に未選択時の文書ルート)でも1ノードだけに
  // ならないための対策。意味エッジ側で既に発見済みのノードは意味ノードとしての表現
  // (色・実線)を優先し、contains 側では重複させない。
  const parent = idx.parentOf.get(center);
  const childrenAll = idx.childrenOf.get(center) ?? [];
  const children = childrenAll.slice(0, MAX_CONTAINS_CHILDREN);
  const siblings = parent ? nearestSiblings(idx, parent, center, MAX_SIBLINGS) : [];

  const containsOrder = [...(parent ? [parent] : []), ...children, ...siblings];
  const seenContains = new Set<NodeId>();
  const containsNodes = containsOrder.filter((id) => {
    if (id === center || hopOf.has(id) || seenContains.has(id)) return false;
    if (!isVisible(id) || !idx.nodes.has(id)) return false;
    seenContains.add(id);
    return true;
  });

  const positions = new Map<NodeId, { x: number; y: number }>();
  positions.set(center, { x: 0, y: 0 });

  // --- G1.6 #2: 各リングの半径・角度幅はラベル幅(概算)に応じて動的に配分する
  // (固定の等分割ではない)。内側から順に: contains → hop1 → hop2(扇状)。
  const rc = placeFullRing(containsNodes, positions, idx, CONTAINS_RADIUS_MIN, NODE_R - 2, MAX_LABEL_CONTAINS);
  const r1 = placeFullRing(hop1, positions, idx, Math.max(HOP1_RADIUS_MIN, rc + 50), NODE_R + 1, MAX_LABEL_HOP1);

  const byHop1: Map<NodeId, NodeId[]> = new Map();
  for (const id of hop2) {
    const p = discoveredVia.get(id) ?? id;
    if (!byHop1.has(p)) byHop1.set(p, []);
    byHop1.get(p)!.push(id);
  }
  const r2 = placeWedges(hop1, byHop1, positions, idx, Math.max(HOP2_RADIUS_MIN, r1 + 70), NODE_R - 1, MAX_LABEL_HOP2, cmp);

  const rawNodes: Omit<LocalNodePos, "labelSide" | "labelDy">[] = [
    { id: center, x: 0, y: 0, hop: 0, kind: "center" },
    ...containsNodes.map((id) => ({ id, x: positions.get(id)!.x, y: positions.get(id)!.y, hop: 1, kind: "contains" as const })),
    ...hop1.map((id) => ({ id, x: positions.get(id)!.x, y: positions.get(id)!.y, hop: 1, kind: "semantic" as const })),
    ...hop2.map((id) => ({ id, x: positions.get(id)!.x, y: positions.get(id)!.y, hop: 2, kind: "semantic" as const })),
  ];
  const nodes = resolveLabelCollisions(rawNodes, idx);

  // 描画するエッジ: 含めたノード同士を結ぶものは意味エッジ・contains 問わず全て描く
  // (例: hop1 の意味ノードがたまたま中心の実子である場合、contains の破線も重ねて出る
  // ことがあるが、それは構造的な関係を補足する情報として許容する)。
  const included = new Set<NodeId>([center, ...containsNodes, ...hop1, ...hop2]);
  const edges = idx.edges.filter((e) => included.has(e.from) && included.has(e.to));

  const radius = r2 > 0 ? r2 : r1 > 0 ? r1 : rc;
  return { center, nodes, edges, breadcrumb, radius };
}

function neighborsOf(idx: GraphIndex, id: NodeId): NodeId[] {
  const out = (idx.semanticOut.get(id) ?? []).map((e) => e.to);
  const inn = (idx.semanticIn.get(id) ?? []).map((e) => e.from);
  return [...new Set([...out, ...inn])];
}

function cmpLabel(idx: GraphIndex) {
  return (a: NodeId, b: NodeId) => {
    const na = idx.nodes.get(a);
    const nb = idx.nodes.get(b);
    const la = na ? deriveLabel(na) : a;
    const lb = nb ? deriveLabel(nb) : b;
    return la.localeCompare(lb);
  };
}

/** 中心に最も近い兄弟から交互(後→前)に最大 `max` 件を採る(裁量: 隣接の兄弟の方が
 * 文脈として関連が強いと考えられるため、文書順の先頭で打ち切るより中心付近を優先する)。 */
function nearestSiblings(idx: GraphIndex, parent: NodeId, center: NodeId, max: number): NodeId[] {
  const all = idx.childrenOf.get(parent) ?? [];
  const pos = all.indexOf(center);
  if (pos < 0) return all.filter((id) => id !== center).slice(0, max);
  const before = all.slice(0, pos);
  const after = all.slice(pos + 1);
  if (before.length + after.length <= max) return [...before, ...after];
  const result: NodeId[] = [];
  let bi = before.length - 1;
  let ai = 0;
  while (result.length < max && (bi >= 0 || ai < after.length)) {
    if (ai < after.length) result.push(after[ai++]);
    if (result.length < max && bi >= 0) result.push(before[bi--]);
  }
  return result;
}

interface ArcItem {
  id: NodeId;
  need: number;
}

/** ノードがリング上で占めるべき弧の長さ(px 相当)の概算。ノード円の直径+隙間+
 * (実際に表示される切り詰めラベルの)概算幅。 */
function arcNeed(idx: GraphIndex, id: NodeId, nodeR: number, maxLabel: number): number {
  const node = idx.nodes.get(id);
  const label = node ? truncate(deriveLabel(node), maxLabel) : id;
  return nodeR * 2 + LABEL_GAP + estimateLabelWidth(label);
}

/** ids を全周(2π)にラベル幅に応じて敷き詰め、positions に書き込む。必要な半径を返す
 * (0件なら 0)。半径はラベル幅の合計から「隙間なく1周ぴったり収まる」値として算出する
 * ため、`minRadius` より狭くなる場合のみ `minRadius` に切り上げる。 */
function placeFullRing(
  ids: NodeId[],
  positions: Map<NodeId, { x: number; y: number }>,
  idx: GraphIndex,
  minRadius: number,
  nodeR: number,
  maxLabel: number,
): number {
  if (ids.length === 0) return 0;
  const items: ArcItem[] = ids.map((id) => ({ id, need: arcNeed(idx, id, nodeR, maxLabel) }));
  const sum = items.reduce((s, it) => s + it.need, 0);
  const radius = Math.max(minRadius, sum / (2 * Math.PI));
  let acc = -Math.PI / 2;
  for (const it of items) {
    const width = (2 * Math.PI * it.need) / sum;
    const angle = acc + width / 2;
    positions.set(it.id, { x: radius * Math.cos(angle), y: radius * Math.sin(angle) });
    acc += width;
  }
  return radius;
}

/** hop2 用: hop1 の各ノードに割り当てた扇(wedge)内に、そのノード経由で発見された hop2
 * ノードをラベル幅に応じて敷き詰める。扇の角度幅は hop1 の周方向の持ち分から決め、
 * 半径は「最も混雑した扇」に必要な値を全体で共有する(扇ごとに半径を変えると同心円の
 * 見た目が崩れるため)。 */
function placeWedges(
  hop1: NodeId[],
  byHop1: Map<NodeId, NodeId[]>,
  positions: Map<NodeId, { x: number; y: number }>,
  idx: GraphIndex,
  minRadius: number,
  nodeR: number,
  maxLabel: number,
  cmp: (a: NodeId, b: NodeId) => number,
): number {
  const hop2Count = [...byHop1.values()].reduce((s, v) => s + v.length, 0);
  if (hop2Count === 0 || hop1.length === 0) return 0;

  const wedgeAngle = Math.min(Math.PI / 2.2, ((2 * Math.PI) / hop1.length) * 0.85);
  let radius = minRadius;
  for (const pid of hop1) {
    const kids = byHop1.get(pid) ?? [];
    if (kids.length === 0) continue;
    const sum = kids.reduce((s, id) => s + arcNeed(idx, id, nodeR, maxLabel), 0);
    radius = Math.max(radius, sum / wedgeAngle);
  }

  hop1.forEach((pid, i) => {
    const baseAngle = (2 * Math.PI * i) / hop1.length - Math.PI / 2;
    const kids = (byHop1.get(pid) ?? []).sort(cmp);
    if (kids.length === 0) return;
    const items: ArcItem[] = kids.map((id) => ({ id, need: arcNeed(idx, id, nodeR, maxLabel) }));
    const sum = items.reduce((s, it) => s + it.need, 0);
    let acc = baseAngle - wedgeAngle / 2;
    for (const it of items) {
      const width = (wedgeAngle * it.need) / sum;
      const angle = acc + width / 2;
      positions.set(it.id, { x: radius * Math.cos(angle), y: radius * Math.sin(angle) });
      acc += width;
    }
  });

  return radius;
}

/** ラベル背景プレートと同じ縦の高さ(GraphPane.tsx の `<rect>` と揃える)。 */
const LABEL_RECT_H = 14;
/** ラベル矩形の衝突解決の反復回数(裁量の値。ノード数が多くても軽い計算量なので
 * 収束を優先してやや多めに回す)。 */
const COLLISION_ITERATIONS = 8;
/** 1ノードが衝突解決で動ける最大の累積オフセット(px)。無制限に押し出すと
 * ノードから離れすぎて「どのノードのラベルか」がわからなくなるので上限を設ける。 */
const MAX_LABEL_DY = 46;

interface LabelRect {
  id: NodeId;
  /** 中心ノードは固定点として扱う(押しのけられる側専用、動かさない)。 */
  fixed: boolean;
  x0: number;
  x1: number;
  y0: number;
  y1: number;
  dy: number;
}

/** ノードの位置・種別からラベル矩形の初期形状とアンカー側を決める(G2.1 #1: 角度に
 * 応じたアンカー切替)。中心は常に真上、リングのノードは x の符号(左右どちらの
 * 半分にいるか)でラベルを伸ばす向きを決める。 */
function initialLabelRect(idx: GraphIndex, n: Omit<LocalNodePos, "labelSide" | "labelDy">): { rect: LabelRect; side: LabelSide } {
  const node = idx.nodes.get(n.id);
  const maxLabel = maxLabelFor(n.kind, n.hop);
  const label = node ? truncate(deriveLabel(node), maxLabel) : n.id;
  const labelW = estimateLabelWidth(label);
  const r = nodeRadiusFor(n.kind, n.hop);

  if (n.kind === "center") {
    const halfW = labelW / 2 + 3;
    const baseline = -(r + 12);
    return {
      side: "top",
      rect: { id: n.id, fixed: true, x0: n.x - halfW, x1: n.x + halfW, y0: n.y + baseline - 11, y1: n.y + baseline + 3, dy: 0 },
    };
  }

  const side: LabelSide = n.x >= 0 ? "right" : "left";
  const width = labelW + 6;
  const x0 = side === "right" ? n.x + r + 3 : n.x - r - 3 - width;
  const x1 = x0 + width;
  return { side, rect: { id: n.id, fixed: false, x0, x1, y0: n.y - 8, y1: n.y + LABEL_RECT_H - 8, dy: 0 } };
}

/** G2.1 #1: ラベル矩形の衝突解決パス。初期配置(アンカー切替込み)の矩形同士で
 * AABB の重なりを検出したら垂直方向に押し出し、数回反復して収束させる(裁量:
 * 半径方向ではなく垂直ナッジを選んだ理由 — 半径方向に押すとリングの同心円の見た目が
 * 崩れるが、垂直ナッジならノードの位置自体は変えずラベルだけをずらせるため)。
 * 中心ノードのラベル矩形は固定点として扱う(他のノードのラベルが中心のラベルと
 * 重なったら、中心側ではなくもう一方を押しのける)。 */
function resolveLabelCollisions(raw: Omit<LocalNodePos, "labelSide" | "labelDy">[], idx: GraphIndex): LocalNodePos[] {
  const sides = new Map<NodeId, LabelSide>();
  const rects: LabelRect[] = raw.map((n) => {
    const { rect, side } = initialLabelRect(idx, n);
    sides.set(n.id, side);
    return rect;
  });

  for (let iter = 0; iter < COLLISION_ITERATIONS; iter++) {
    let moved = false;
    for (let i = 0; i < rects.length; i++) {
      for (let j = i + 1; j < rects.length; j++) {
        const a = rects[i];
        const b = rects[j];
        const ay0 = a.y0 + a.dy;
        const ay1 = a.y1 + a.dy;
        const by0 = b.y0 + b.dy;
        const by1 = b.y1 + b.dy;
        const xOverlap = Math.min(a.x1, b.x1) - Math.max(a.x0, b.x0);
        const yOverlap = Math.min(ay1, by1) - Math.max(ay0, by0);
        if (xOverlap <= 0 || yOverlap <= 0) continue;

        const push = yOverlap / 2 + 1;
        const aOnTop = ay0 + ay1 <= by0 + by1;
        if (!a.fixed && !b.fixed) {
          a.dy += aOnTop ? -push : push;
          b.dy += aOnTop ? push : -push;
        } else if (!a.fixed) {
          a.dy += aOnTop ? -push * 2 : push * 2;
        } else if (!b.fixed) {
          b.dy += aOnTop ? push * 2 : -push * 2;
        } else {
          continue; // 両方固定(中心同士、実際には起こらない)なら何もしない。
        }
        moved = true;
      }
    }
    // 発散防止: 累積オフセットを上限でクランプする。
    for (const r of rects) r.dy = Math.max(-MAX_LABEL_DY, Math.min(MAX_LABEL_DY, r.dy));
    if (!moved) break;
  }

  const dyOf = new Map(rects.map((r) => [r.id, r.dy]));
  return raw.map((n) => ({ ...n, labelSide: sides.get(n.id) ?? "right", labelDy: dyOf.get(n.id) ?? 0 }));
}
