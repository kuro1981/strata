// グラフ JSON から UI が使う索引を1回だけ構築する(G1 WS-B)。
//
// - contains の親子(構造ペインのレイアウト・実効 class の祖先チェーンに使う)
// - 意味エッジ(contains 以外)の from/to 索引(関係パネル・グラフオーバーレイに使う)
// - alias / ULID → NodeId のジャンプ索引(doc 修飾 "doc/alias" と裸の alias 両対応)
// - ノード → 所属文書(docPath)

import type { Edge, GraphJson, NodeId, SiteRoot, StrataNode } from "@/types/graph";
import { isSemanticRel } from "@/types/graph";

export interface GraphIndex {
  /** 生の graph.json 一式(doc_aliases など索引化しなかったフィールドを直接参照したい
   * 場合に備えて保持する)。 `roots`/`edges` は下記フィールドとして展開済みなので、
   * それらを使う分には `raw` を読む必要はない。 */
  raw: GraphJson;
  roots: SiteRoot[];
  edges: Edge[];
  nodes: Map<NodeId, StrataNode>;
  /** contains の子(ord 昇順)。 */
  childrenOf: Map<NodeId, NodeId[]>;
  parentOf: Map<NodeId, NodeId>;
  /** 意味エッジ(contains 以外)。 */
  semanticOut: Map<NodeId, Edge[]>;
  semanticIn: Map<NodeId, Edge[]>;
  /** ノード → 所属文書の roots[].path(到達不能なノードは undefined)。 */
  docOf: Map<NodeId, string>;
  /** グラフ全体に現れる意味エッジの rel 一覧(凡例用、未知 rel も含む)。 */
  relsPresent: string[];
  /** グラフ全体に現れる class 一覧(class トグル用)。 */
  allClasses: string[];
  aliasIndex: AliasIndex;
}

export interface AliasIndex {
  /** 裸の alias → NodeId(複数文書で衝突する場合は最初の1件。v1 は完全一致のみでよい
   * という指示に合わせた裁量: 衝突時のあいまいさは既知の制限として最終報告に明記)。 */
  byBareAlias: Map<string, NodeId>;
  /** "doc/alias" → NodeId(doc_aliases から)。 */
  byQualifiedAlias: Map<string, NodeId>;
  /** 文書 alias(roots[].alias) → その文書の root NodeId。 */
  byDocAlias: Map<string, NodeId>;
}

const ULID_RE = /^[0-7][0-9A-HJKMNP-TV-Z]{25}$/i;

export function buildIndex(gj: GraphJson): GraphIndex {
  const nodes = new Map<NodeId, StrataNode>(Object.entries(gj.graph.nodes));
  const childrenOf = new Map<NodeId, NodeId[]>();
  const parentOf = new Map<NodeId, NodeId>();
  const semanticOut = new Map<NodeId, Edge[]>();
  const semanticIn = new Map<NodeId, Edge[]>();
  const relsPresent = new Set<string>();

  // contains 子は ord 昇順(未指定は末尾)。同一 from への複数 contains エッジを
  // 一旦集めてからソートする。
  const rawChildren = new Map<NodeId, Edge[]>();

  for (const e of gj.graph.edges) {
    if (e.rel === "contains") {
      if (!rawChildren.has(e.from)) rawChildren.set(e.from, []);
      rawChildren.get(e.from)!.push(e);
      if (!parentOf.has(e.to)) parentOf.set(e.to, e.from);
    } else {
      if (isSemanticRel(e.rel)) relsPresent.add(e.rel);
      if (!semanticOut.has(e.from)) semanticOut.set(e.from, []);
      semanticOut.get(e.from)!.push(e);
      if (!semanticIn.has(e.to)) semanticIn.set(e.to, []);
      semanticIn.get(e.to)!.push(e);
    }
  }
  for (const [from, edges] of rawChildren) {
    edges.sort((a, b) => (a.ord ?? Number.MAX_SAFE_INTEGER) - (b.ord ?? Number.MAX_SAFE_INTEGER));
    childrenOf.set(
      from,
      edges.map((e) => e.to),
    );
  }

  // 所属文書: 各 root から contains を BFS で辿って docPath を割り当てる。
  const docOf = new Map<NodeId, string>();
  for (const root of gj.roots) {
    if (!root.root) continue;
    const stack = [root.root];
    while (stack.length > 0) {
      const id = stack.pop()!;
      if (docOf.has(id)) continue;
      docOf.set(id, root.path);
      for (const child of childrenOf.get(id) ?? []) stack.push(child);
    }
  }

  const allClasses = new Set<string>();
  for (const n of nodes.values()) {
    for (const c of n.classes ?? []) allClasses.add(c);
  }

  const aliasIndex = buildAliasIndex(gj, nodes);

  return {
    raw: gj,
    roots: gj.roots,
    edges: gj.graph.edges,
    nodes,
    childrenOf,
    parentOf,
    semanticOut,
    semanticIn,
    docOf,
    relsPresent: [...relsPresent].sort(),
    allClasses: [...allClasses].sort(),
    aliasIndex,
  };
}

function buildAliasIndex(gj: GraphJson, nodes: Map<NodeId, StrataNode>): AliasIndex {
  const byBareAlias = new Map<string, NodeId>();
  const byQualifiedAlias = new Map<string, NodeId>();
  const byDocAlias = new Map<string, NodeId>();

  for (const n of nodes.values()) {
    if (n.alias && !byBareAlias.has(n.alias)) byBareAlias.set(n.alias, n.id);
  }
  for (const root of gj.roots) {
    if (root.alias && root.root) byDocAlias.set(root.alias, root.root);
  }
  if (gj.doc_aliases) {
    for (const [doc, blocks] of Object.entries(gj.doc_aliases)) {
      for (const [alias, id] of Object.entries(blocks)) {
        byQualifiedAlias.set(`${doc}/${alias}`, id);
      }
    }
  }

  return { byBareAlias, byQualifiedAlias, byDocAlias };
}

/** D46: 実効 class(自身 + contains 祖先の classes の和集合)。 */
export function effectiveClasses(idx: GraphIndex, id: NodeId): Set<string> {
  const set = new Set<string>();
  let cur: NodeId | undefined = id;
  while (cur) {
    const n = idx.nodes.get(cur);
    if (n) for (const c of n.classes ?? []) set.add(c);
    cur = idx.parentOf.get(cur);
  }
  return set;
}

/** alias/ULID の完全一致解決(v1 は検索なしなので完全一致のみでよい、という指示どおり)。
 * 優先順位: ULID 直接一致 → "doc/alias" 修飾 → 裸の alias → 文書 alias。 */
export function resolveJumpTarget(idx: GraphIndex, query: string): NodeId | undefined {
  const q = query.trim();
  if (!q) return undefined;
  if (ULID_RE.test(q) && idx.nodes.has(q.toUpperCase())) return q.toUpperCase();
  if (idx.nodes.has(q)) return q;
  if (q.includes("/")) {
    const hit = idx.aliasIndex.byQualifiedAlias.get(q);
    if (hit) return hit;
  }
  const bare = idx.aliasIndex.byBareAlias.get(q);
  if (bare) return bare;
  return idx.aliasIndex.byDocAlias.get(q);
}
