// Table(次元の木)を HTML の行列ヘッダへ変換する純粋関数群。
// 「結合されたヘッダセル」= children を持つ Member。span は元データに無く、
// render 時に葉の数から計算する派生物(strata-core の設計どおり、UI 側も踏襲)。
//
// 裁量(最終報告参照): 列軸(cols)は colspan/rowspan つきの入れ子ヘッダとして
// 完全に描画するが、行軸(rows)は左側ヘッダの見やすさを優先して祖先ラベルを
// " / " で連結した単一セルに簡略化している(rowspan の入れ子結合はしない)。

import type { Cell, CellValue, Dim, Member } from "@/types/graph";
import { inlineToText } from "./label";

export interface HeaderNode {
  label: string;
  path: string[];
  children: HeaderNode[];
  leaf: boolean;
}

export function buildHeaderTree(dims: Dim[] | undefined, pathPrefix: string[] = []): HeaderNode[] {
  if (!dims) return [];
  const nodes: HeaderNode[] = [];
  for (const dim of dims) {
    for (const m of dim.members) {
      nodes.push(memberToNode(m, pathPrefix));
    }
  }
  return nodes;
}

function memberToNode(m: Member, pathPrefix: string[]): HeaderNode {
  const path = [...pathPrefix, m.key];
  const label = m.label ? inlineToText(m.label) : m.key;
  const children = m.children && m.children.length > 0 ? buildHeaderTree(m.children, path) : [];
  return { label, path, children, leaf: children.length === 0 };
}

export function headerDepth(nodes: HeaderNode[]): number {
  if (nodes.length === 0) return 0;
  return 1 + Math.max(0, ...nodes.map((n) => headerDepth(n.children)));
}

export function leafCount(n: HeaderNode): number {
  return n.leaf ? 1 : n.children.reduce((s, c) => s + leafCount(c), 0);
}

export function collectLeaves(nodes: HeaderNode[]): HeaderNode[] {
  const out: HeaderNode[] = [];
  for (const n of nodes) {
    if (n.leaf) out.push(n);
    else out.push(...collectLeaves(n.children));
  }
  return out;
}

export interface HeaderCell {
  label: string;
  colSpan: number;
  rowSpan: number;
  path: string[];
}

/** 列ヘッダを depth 段の行に展開する(colspan/rowspan つき)。 */
export function columnHeaderRows(nodes: HeaderNode[], depth: number): HeaderCell[][] {
  const rows: HeaderCell[][] = Array.from({ length: Math.max(depth, 1) }, () => []);
  const walk = (ns: HeaderNode[], level: number) => {
    for (const n of ns) {
      if (n.leaf) {
        rows[level].push({ label: n.label, colSpan: 1, rowSpan: depth - level, path: n.path });
      } else {
        rows[level].push({ label: n.label, colSpan: leafCount(n), rowSpan: 1, path: n.path });
        walk(n.children, level + 1);
      }
    }
  };
  walk(nodes, 0);
  return rows;
}

/** 行ヘッダ用の葉一覧。祖先ラベルを " / " で連結した簡略表示(裁量)。 */
export function rowLeafLabels(nodes: HeaderNode[]): { label: string; path: string[] }[] {
  const leaves = collectLeaves(nodes);
  return leaves.map((l) => ({ label: ancestorLabel(nodes, l.path), path: l.path }));
}

function ancestorLabel(nodes: HeaderNode[], path: string[]): string {
  const labels: string[] = [];
  let cur = nodes;
  for (const key of path) {
    const found = cur.find((n) => n.path[n.path.length - 1] === key);
    if (!found) break;
    labels.push(found.label);
    cur = found.children;
  }
  return labels.join(" / ");
}

export function cellLookup(cells: Cell[]): Map<string, CellValue> {
  const map = new Map<string, CellValue>();
  for (const c of cells) {
    map.set(cellKey(c.row_path, c.col_path), c.value);
  }
  return map;
}

export function cellKey(rowPath: string[], colPath: string[]): string {
  return `${rowPath.join("␟")}|${colPath.join("␟")}`;
}

export function cellValueToText(v: CellValue | undefined): string {
  if (!v) return "";
  switch (v.k) {
    case "number":
      return String(v.v);
    case "text":
      return v.v;
    case "ref":
      return `→${v.to}`;
    case "empty":
      return "";
    case "quantity":
      return `${v.v} ${v.unit}`;
    case "date":
      return v.d ? `${v.y}-${pad(v.m)}-${pad(v.d)}` : `${v.y}-${pad(v.m)}`;
    case "period": {
      const from = v.from.d ? `${v.from.y}-${pad(v.from.m)}-${pad(v.from.d)}` : `${v.from.y}-${pad(v.from.m)}`;
      const to = v.to ? (v.to.d ? `${v.to.y}-${pad(v.to.m)}-${pad(v.to.d)}` : `${v.to.y}-${pad(v.to.m)}`) : "現在";
      return `${from} 〜 ${to}`;
    }
    default:
      return "";
  }
}

function pad(n: number): string {
  return String(n).padStart(2, "0");
}
