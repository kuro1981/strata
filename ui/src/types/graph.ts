// Strata canonical graph JSON — 手書き最小限の型定義(G1 WS-B、裁量: 自動生成はしない)。
//
// 対応する Rust 定義: crates/strata-core/src/lib.rs(Node/NodePayload/Edge/Rel/Inline/…)。
// `strata site` が書き出す graph.json の実際の形は
// `cargo run -p strata-cli -- build --workspace docs/spec-sml/strata.toml -o -` で確認した
// (docs/graph-ui-g1-handoff.md 指示どおり)。
//
// 前方互換の方針: `Rel` と `NodePayload.type` はどちらも「知らない値でも落ちない」ことが
// 要件(sml-spec.md §1.13 の revises 追加のように、rel は今後も増えうる)。そのため
// - `Rel` は既知語彙の union ではなく `string`(既知語彙は `KNOWN_RELS` で別途持つ)
// - `Node` は既知 type の判別共用体 `KnownNode` + フォールバックの `UnknownNode` の union

export type NodeId = string; // ULID 文字列

// --- Rel ---------------------------------------------------------------------

/** 既知の意味エッジ語彙(sml-spec.md §4, §1.13 D48)。凡例・配色のためだけに使う —
 * 未知の rel 文字列が来ても `Edge.rel: string` としてそのまま保持・描画できる。 */
export const KNOWN_RELS = [
  "contains",
  "defines",
  "term-ref",
  "supports",
  "depends-on",
  "cites",
  "instance-of",
  "refers-to",
  "revises",
] as const;
export type KnownRel = (typeof KNOWN_RELS)[number];
export type Rel = KnownRel | string;

/** 紙面(render)には出ない「意味エッジ」= contains 以外の全て(D48 の revises も同列)。
 * グラフペインのオーバーレイ対象。 */
export function isSemanticRel(rel: Rel): boolean {
  return rel !== "contains";
}

// --- Edge ----------------------------------------------------------------------

export interface Edge {
  from: NodeId;
  to: NodeId;
  rel: Rel;
  ord?: number;
}

// --- Inline AST ------------------------------------------------------------------

export interface CellCoord {
  row_path: string[];
  col_path: string[];
}

export type EmphKind = "strong" | "em" | "code" | "strike";

export type Inline =
  | { t: "text"; s: string }
  | { t: "emph"; kind: EmphKind; children: Inline[] }
  | { t: "math"; tree: MathNode }
  | { t: "ref"; to: NodeId; rel: Rel; coord?: CellCoord; text?: string }
  | { t: "term"; to: NodeId; text?: string }
  | { t: "anchor"; to: NodeId }
  | { t: "link"; url: string; text: string }
  | { t: "image"; url: string; alt: string };

// --- Math (MathML Presentation サブセット) ---------------------------------------

export type MathNode =
  | { op: "num"; v: string }
  | { op: "ident"; v: string }
  | { op: "op"; v: string }
  | { op: "row"; items: MathNode[] }
  | { op: "frac"; num: MathNode; den: MathNode }
  | { op: "sup"; base: MathNode; sup: MathNode }
  | { op: "sub"; base: MathNode; sub: MathNode }
  | { op: "sub_sup"; base: MathNode; sub: MathNode; sup: MathNode }
  | { op: "under_over"; base: MathNode; under?: MathNode; over?: MathNode }
  | { op: "sqrt"; body: MathNode }
  | { op: "root"; radicand: MathNode; index: MathNode }
  | { op: "fenced"; open: string; close: string; body: MathNode }
  | { op: "text"; s: string };

// --- 表 = 次元の木 -----------------------------------------------------------------

export interface Member {
  key: string;
  label?: Inline[];
  children?: Dim[];
}
export interface Dim {
  name: string;
  members: Member[];
}
export type DimTree = Dim[];

export interface DateValue {
  y: number;
  m: number;
  d?: number;
}

export type CellValue =
  | { k: "number"; v: number }
  | { k: "text"; v: string }
  | { k: "ref"; to: NodeId }
  | { k: "empty" }
  | { k: "quantity"; v: number; unit: string }
  | ({ k: "date" } & DateValue)
  | { k: "period"; from: DateValue; to?: DateValue };

export interface Cell {
  row_path: string[];
  col_path: string[];
  value: CellValue;
}

// --- 図 ------------------------------------------------------------------------

export interface Encoding {
  x: string;
  y: string;
  color?: string;
}

export type Figure =
  | {
      kind: "chart";
      data_ref: NodeId;
      mark: "line" | "bar" | "point" | "area";
      encode: Encoding;
      caption?: Inline[];
      depicts?: Record<string, string>;
    }
  | {
      kind: "image";
      src: string;
      alt: string;
      depicts?: Record<string, string>;
      caption?: Inline[];
    };

// --- ノード ----------------------------------------------------------------------

interface NodeBase {
  id: NodeId;
  /** D23: 出し分け用の意味分類タグ。実効 class(自身+祖先の和集合)は
   * `lib/graphIndex.ts` の `effectiveClasses` で計算する(D46)。 */
  classes?: string[];
  /** D26: 解決済みエイリアス(fmt が注入)。 */
  alias?: string;
}

export interface DocumentNode extends NodeBase {
  type: "document";
  title?: string;
}
export interface SectionNode extends NodeBase {
  type: "section";
  heading: Inline[];
}
export interface ParaNode extends NodeBase {
  type: "para";
  inline: Inline[];
  checked?: boolean;
}
export interface ListNode extends NodeBase {
  type: "list";
  ordered: boolean;
  start?: number;
}
export interface RecordEntry {
  key: string;
  value: CellValue;
}
export interface RecordNode extends NodeBase {
  type: "record";
  entries: RecordEntry[];
}
export interface TableNode extends NodeBase {
  type: "table";
  rows: DimTree;
  cols: DimTree;
  cells: Cell[];
  caption?: Inline[];
}
export interface MathBlockNode extends NodeBase {
  type: "math";
  tree: MathNode;
}
export interface CodeNode extends NodeBase {
  type: "code";
  lang: string;
  src: string;
}
export interface TermNode extends NodeBase {
  type: "term";
  name: string;
}
export interface AnchorNode extends NodeBase {
  type: "anchor";
  inline: Inline[];
}
export interface ValueNode extends NodeBase {
  type: "value";
  scalar: number | string | boolean;
  unit?: string;
}
export interface QuoteNode extends NodeBase {
  type: "quote";
}
export interface ThematicBreakNode extends NodeBase {
  type: "thematic_break";
}
export type FigureNode = NodeBase & { type: "figure" } & Figure;

export type KnownNode =
  | DocumentNode
  | SectionNode
  | ParaNode
  | ListNode
  | RecordNode
  | TableNode
  | MathBlockNode
  | CodeNode
  | TermNode
  | AnchorNode
  | ValueNode
  | QuoteNode
  | ThematicBreakNode
  | FigureNode;

/** 前方互換フォールバック: 将来 payload バリアントが増えても UI は落ちない
 * (未知 type は汎用ブロックとして「type 名 + 生 JSON」を表示する)。 */
export interface UnknownNode extends NodeBase {
  type: string;
  [key: string]: unknown;
}

export type StrataNode = KnownNode | UnknownNode;

const KNOWN_TYPES = new Set<string>([
  "document",
  "section",
  "para",
  "list",
  "record",
  "table",
  "math",
  "code",
  "term",
  "anchor",
  "value",
  "quote",
  "thematic_break",
  "figure",
]);

export function isKnownNode(n: StrataNode): n is KnownNode {
  return KNOWN_TYPES.has(n.type);
}

// --- グラフ本体・ワークスペース ------------------------------------------------------

export interface Graph {
  nodes: Record<NodeId, StrataNode>;
  edges: Edge[];
}

/** ワークスペース build の `DocRoot`(単一文書 build も `strata site` 側で
 * 同じ形に正規化して書き出す。裁量、最終報告参照)。 */
export interface SiteRoot {
  path: string;
  alias?: string;
  root?: NodeId;
}

/** `strata site` が書き出す graph.json のトップレベル形。単一ファイル/ワークスペース
 * どちらの build でも同じ形になるよう strata-cli 側で正規化する(裁量)。 */
export interface GraphJson {
  graph: Graph;
  roots: SiteRoot[];
  doc_aliases?: Record<string, Record<string, NodeId>>;
}
