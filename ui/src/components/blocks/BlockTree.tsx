// 文書ペイン本体: グラフから文書を描画する(Document → contains 順、D49 v0 体験 1.)。
// 各ブロックは種別ごとの内容を描画したうえで、自身の contains 子を再帰的に描画する
// (コンテナ/非コンテナを type ごとに分岐させず、常に「内容 + 子」の形に統一する裁量)。

import { useMemo } from "react";
import type { KnownNode, StrataNode, NodeId, UnknownNode } from "@/types/graph";
import { isKnownNode } from "@/types/graph";
import { useGraph } from "@/state/GraphContext";
import { InlineList, MathInline } from "@/components/Inline";
import { Badge } from "@/components/ui/badge";
import { RelationDegreeBadge, RelationPanel } from "@/components/RelationPanel";
import { ALIAS_BADGE_CLASS, deriveLabel, documentTitle, typeLabel } from "@/lib/label";
import {
  buildHeaderTree,
  cellKey,
  cellLookup,
  cellValueToText,
  collectLeaves,
  columnHeaderRows,
  headerDepth,
  rowLeafLabels,
} from "@/lib/table";
import { cn } from "@/lib/utils";

const HEADING_SIZE = ["text-xl font-semibold", "text-lg font-semibold", "text-base font-semibold", "text-sm font-semibold"];

export function BlockTree({ id, depth = 0 }: { id: NodeId; depth?: number }) {
  const { idx, isHidden } = useGraph();
  if (isHidden(id)) return null;
  const node = idx.nodes.get(id);
  if (!node) return null;
  return <Block node={node} depth={depth} />;
}

/** 文書の contains 木から到達できないノード(term 宣言など)向けの単発カード。
 * 通常の `Block` と違い、子の再帰描画はしない(そもそも所属先が無い)。
 * `DocumentPane` が「選択中ノードが文書外」を検知したときに使う。 */
export function OrphanNodeCard({ id }: { id: NodeId }) {
  const { idx, registerBlockRef } = useGraph();
  const node = idx.nodes.get(id);
  if (!node) return null;
  return (
    <div
      ref={(el) => registerBlockRef(id, el)}
      className="mb-3 rounded-md border border-dashed border-amber-400/60 bg-amber-50/60 px-3 py-2 dark:bg-amber-950/30"
    >
      <div className="mb-1 flex items-center gap-1.5 text-[11px] font-medium text-muted-foreground">
        <span>文書外のノード({typeLabel(node.type)}) — term 宣言のように文書構造の外から参照専用で存在する</span>
        {/* G1.7 方針2: これ自体が「選択中ノードの詳細」表示なので alias バッジを出す。 */}
        {node.alias && <span className={ALIAS_BADGE_CLASS}>#{node.alias}</span>}
      </div>
      <BlockContent node={node} depth={0} />
      <RelationPanel id={id} />
    </div>
  );
}

function Block({ node, depth }: { node: StrataNode; depth: number }) {
  const { idx, selected, select, registerBlockRef, isHidden } = useGraph();
  const isSelected = selected === node.id;
  const children = (idx.childrenOf.get(node.id) ?? []).filter((c) => !isHidden(c));

  return (
    <div
      ref={(el) => registerBlockRef(node.id, el)}
      onClick={(e) => {
        e.stopPropagation();
        select(node.id);
      }}
      className={cn(
        "group my-0.5 cursor-pointer rounded-md px-2 py-1 transition-colors",
        isSelected ? "bg-primary/10 ring-1 ring-primary/50" : "hover:bg-muted/60",
      )}
      data-node-id={node.id}
    >
      <div className="flex flex-wrap items-start gap-1.5">
        <div className="min-w-0 flex-1">
          <BlockContent node={node} depth={depth} />
        </div>
        <div className="flex shrink-0 items-center gap-1 pt-0.5">
          {/* G1.7 方針2: alias バッジは「ノード詳細(選択時)」に限定する
              (常時表示すると和文本文の脇に英字 alias が常に並び、ユーザー目視
              フィードバックの「同じ場所を示す2つの表記が奇妙」の一因になっていた)。 */}
          {isSelected && node.alias && (
            <span className={ALIAS_BADGE_CLASS} title={node.id}>
              #{node.alias}
            </span>
          )}
          {(node.classes ?? []).map((c) => (
            <Badge key={c} variant="secondary" className="px-1.5 py-0 text-[10px]">
              {c}
            </Badge>
          ))}
          <RelationDegreeBadge id={node.id} />
        </div>
      </div>

      {isSelected && <RelationPanel id={node.id} />}

      {children.length > 0 && <ChildBlocks parent={node} childIds={children} depth={depth + 1} />}
    </div>
  );
}

function ChildBlocks({ parent, childIds, depth }: { parent: StrataNode; childIds: NodeId[]; depth: number }) {
  const list = isKnownNode(parent) && parent.type === "list" ? parent : null;
  if (list) {
    const ordered = list.ordered;
    const start = list.start ?? 1;
    return (
      <ol className="ml-1 list-none border-l border-border/50 pl-3">
        {childIds.map((cid, i) => (
          <li key={cid} className="flex gap-1.5">
            <span className="mt-1 min-w-4 select-none text-xs text-muted-foreground">{ordered ? `${start + i}.` : "•"}</span>
            <div className="min-w-0 flex-1">
              <BlockTree id={cid} depth={depth} />
            </div>
          </li>
        ))}
      </ol>
    );
  }
  if (parent.type === "quote") {
    return (
      <div className="ml-1 border-l-2 border-border pl-3 italic text-muted-foreground">
        {childIds.map((cid) => (
          <BlockTree key={cid} id={cid} depth={depth} />
        ))}
      </div>
    );
  }
  return (
    <div className={cn(depth > 0 ? "ml-2 border-l border-border/40 pl-3" : "")}>
      {childIds.map((cid) => (
        <BlockTree key={cid} id={cid} depth={depth} />
      ))}
    </div>
  );
}

function BlockContent({ node, depth }: { node: StrataNode; depth: number }) {
  if (!isKnownNode(node)) return <UnknownBlockContent node={node} />;
  return <KnownBlockContent node={node} depth={depth} />;
}

function UnknownBlockContent({ node }: { node: UnknownNode }) {
  return (
    <div className="rounded border border-dashed border-amber-400 bg-amber-50 px-2 py-1 text-xs dark:bg-amber-950">
      <span className="font-medium">未知のブロック種別: {typeLabel(node.type)}</span>
      <pre className="mt-1 overflow-x-auto font-mono text-[10px]">{JSON.stringify(node, null, 1)}</pre>
    </div>
  );
}

function KnownBlockContent({ node, depth }: { node: KnownNode; depth: number }) {
  const { idx } = useGraph();
  const resolveRef = (id: NodeId) => {
    const n = idx.nodes.get(id);
    return n ? deriveLabel(n) : id;
  };
  switch (node.type) {
    case "document":
      // G1.7: フォールバック順(title → alias → ULID 短縮)を label.ts の1箇所に
      // 集約する。ただし deriveLabel と違って切り詰めない(28文字は文書見出しには
      // 短すぎる、documentTitle 側のコメント参照)。
      return <h1 className="text-2xl font-bold">{documentTitle(node)}</h1>;
    case "section": {
      const sizeClass = HEADING_SIZE[Math.min(depth, HEADING_SIZE.length - 1)];
      return (
        <div className={sizeClass}>
          <InlineList items={node.heading} />
        </div>
      );
    }
    case "para":
      return (
        <p className="leading-relaxed">
          {node.checked != null && (
            <input type="checkbox" checked={node.checked} readOnly className="mr-2 align-middle" />
          )}
          <InlineList items={node.inline} />
        </p>
      );
    case "list":
      return null; // ラベル無し。子は ChildBlocks が箇条書きとして描画する。
    case "record":
      return (
        <table className="w-full max-w-xl border-collapse text-sm">
          <tbody>
            {node.entries.map((e, i) => (
              <tr key={i} className="border-b border-border/50 last:border-0">
                <th className="w-1/3 py-1 pr-3 text-left align-top font-medium text-muted-foreground">{e.key}</th>
                <td className="py-1">{cellValueToText(e.value, resolveRef)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      );
    case "table":
      return <TableContent node={node} />;
    case "math":
      return (
        <div className="rounded bg-muted/50 px-2 py-1 font-mono text-sm">
          <MathInline tree={node.tree} />
        </div>
      );
    case "code":
      return (
        <pre className="overflow-x-auto rounded bg-muted px-3 py-2 font-mono text-xs">
          <code>{node.src}</code>
        </pre>
      );
    case "term":
      return <span className="font-medium italic">{node.name}</span>;
    case "anchor":
      return (
        <span className="rounded border border-cyan-400/50 bg-cyan-50 px-1 dark:bg-cyan-950">
          <InlineList items={node.inline} />
        </span>
      );
    case "value":
      return (
        <span className="font-mono">
          {String(node.scalar)}
          {node.unit ? ` ${node.unit}` : ""}
        </span>
      );
    case "quote":
      return null; // 内容は無く、子(ChildBlocks)が blockquote として描画される。
    case "thematic_break":
      return <hr className="my-2 border-border" />;
    case "figure":
      return <FigureContent node={node} />;
    default: {
      // 網羅性チェック: KnownNode に新しいバリアントが増えたらここで型エラーになる。
      const _exhaustive: never = node;
      return _exhaustive;
    }
  }
}

function TableContent({ node }: { node: Extract<StrataNode, { type: "table" }> }) {
  const { idx } = useGraph();
  const resolveRef = (id: NodeId) => {
    const n = idx.nodes.get(id);
    return n ? deriveLabel(n) : id;
  };
  const colTree = useMemo(() => buildHeaderTree(node.cols), [node.cols]);
  const rowTree = useMemo(() => buildHeaderTree(node.rows), [node.rows]);
  const colDepth = useMemo(() => Math.max(headerDepth(colTree), 1), [colTree]);
  const colRows = useMemo(() => columnHeaderRows(colTree, colDepth), [colTree, colDepth]);
  const colLeaves = useMemo(
    () => (colTree.length > 0 ? collectLeaves(colTree) : [{ label: "", path: [], leaf: true, children: [] }]),
    [colTree],
  );
  const rowLeaves = useMemo(
    () => (rowTree.length > 0 ? rowLeafLabels(rowTree) : [{ label: "", path: [] }]),
    [rowTree],
  );
  const lookup = useMemo(() => cellLookup(node.cells), [node.cells]);

  return (
    <div className="max-w-full overflow-x-auto">
      {node.caption && (
        <div className="mb-1 text-xs font-medium text-muted-foreground">
          <InlineList items={node.caption} />
        </div>
      )}
      <table className="border-collapse text-sm">
        <thead>
          {colRows.map((row, ri) => (
            <tr key={ri}>
              {ri === 0 && rowTree.length > 0 && <th rowSpan={colDepth} className="border border-border bg-muted/40" />}
              {row.map((c, ci) => (
                <th
                  key={ci}
                  colSpan={c.colSpan}
                  rowSpan={c.rowSpan}
                  className="border border-border bg-muted/40 px-2 py-1 text-center font-medium"
                >
                  {c.label}
                </th>
              ))}
            </tr>
          ))}
        </thead>
        <tbody>
          {rowLeaves.map((rl, ri) => (
            <tr key={ri}>
              {rowTree.length > 0 && (
                <th className="border border-border bg-muted/20 px-2 py-1 text-left font-medium">{rl.label}</th>
              )}
              {colLeaves.map((cl, ci) => {
                const value = lookup.get(cellKey(rl.path, cl.path));
                return (
                  <td key={ci} className="border border-border px-2 py-1 text-right">
                    {cellValueToText(value, resolveRef)}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function FigureContent({ node }: { node: Extract<StrataNode, { type: "figure" }> }) {
  const { idx } = useGraph();
  if (node.kind === "chart") {
    const dataNode = idx.nodes.get(node.data_ref);
    return (
      <div className="rounded border border-border bg-muted/30 px-3 py-2 text-sm">
        <div className="font-medium">
          [図: {node.mark} chart — x={node.encode.x}, y={node.encode.y}
          {node.encode.color ? `, color=${node.encode.color}` : ""}]
        </div>
        {dataNode && (
          <div className="mt-1 flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
            <span>
              データ元: {deriveLabel(dataNode)}({typeLabel(dataNode.type)})
            </span>
            {dataNode.alias && <span className={ALIAS_BADGE_CLASS}>#{dataNode.alias}</span>}
          </div>
        )}
        {node.caption && (
          <div className="mt-1 text-xs italic">
            <InlineList items={node.caption} />
          </div>
        )}
        {node.depicts?.description && <div className="mt-1 text-xs text-muted-foreground">{node.depicts.description}</div>}
      </div>
    );
  }
  return (
    <figure className="rounded border border-border bg-muted/30 px-3 py-2 text-sm">
      {/* eslint-disable-next-line jsx-a11y/alt-text */}
      <img src={node.src} alt={node.alt} loading="lazy" className="max-w-full rounded" />
      {node.caption && (
        <figcaption className="mt-1 text-xs italic">
          <InlineList items={node.caption} />
        </figcaption>
      )}
    </figure>
  );
}

