// ブロック脇の関係パネル(D49 v0 体験 3.)。フォーカス中のブロックの in/out 意味エッジを
// 一覧し、クリックで相手へジャンプする(選択同期の再利用: `select()` を呼ぶだけで
// 文書ペインのスクロール+グラフペインのハイライトが両方付いてくる)。

import type { Edge, NodeId } from "@/types/graph";
import { useGraph } from "@/state/GraphContext";
import { deriveLabel, typeLabel } from "@/lib/label";
import { relStyle } from "@/lib/relStyle";
import { Badge } from "@/components/ui/badge";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";

export function RelationDegreeBadge({ id }: { id: NodeId }) {
  const { idx } = useGraph();
  const out = idx.semanticOut.get(id) ?? [];
  const inn = idx.semanticIn.get(id) ?? [];
  if (out.length + inn.length === 0) return null;
  return (
    <Tooltip>
      <TooltipTrigger className="ml-1 inline-flex select-none items-center rounded-full bg-muted px-1.5 text-[10px] font-medium text-muted-foreground">
        ⇄{out.length + inn.length}
      </TooltipTrigger>
      <TooltipContent>
        out {out.length}({[...new Set(out.map((e) => e.rel))].join(", ") || "-"}) / in {inn.length}(
        {[...new Set(inn.map((e) => e.rel))].join(", ") || "-"})
      </TooltipContent>
    </Tooltip>
  );
}

export function RelationPanel({ id }: { id: NodeId }) {
  const { idx, select } = useGraph();
  const out = idx.semanticOut.get(id) ?? [];
  const inn = idx.semanticIn.get(id) ?? [];

  if (out.length === 0 && inn.length === 0) return null;

  return (
    <div className="mt-1 mb-2 ml-2 rounded-md border border-dashed border-border bg-muted/40 p-2 text-xs">
      <div className="mb-1 font-medium text-muted-foreground">関係</div>
      <div className="flex flex-col gap-1">
        {out.map((e, i) => (
          <RelationRow key={`o${i}`} edge={e} other={e.to} direction="out" onJump={select} />
        ))}
        {inn.map((e, i) => (
          <RelationRow key={`i${i}`} edge={e} other={e.from} direction="in" onJump={select} />
        ))}
      </div>
    </div>
  );
}

function RelationRow({
  edge,
  other,
  direction,
  onJump,
}: {
  edge: Edge;
  other: NodeId;
  direction: "out" | "in";
  onJump: (id: NodeId) => void;
}) {
  const { idx } = useGraph();
  const node = idx.nodes.get(other);
  const style = relStyle(edge.rel);
  return (
    <button
      type="button"
      onClick={() => onJump(other)}
      className="flex w-full items-center gap-1.5 rounded px-1 py-0.5 text-left hover:bg-muted"
    >
      <span className="inline-block h-2 w-2 shrink-0 rounded-full" style={{ backgroundColor: style.color }} />
      <span className="shrink-0 font-mono text-[10px] text-muted-foreground">{direction === "out" ? "→" : "←"}</span>
      <Badge variant="outline" className="shrink-0 px-1 py-0 text-[10px]">
        {edge.rel}
      </Badge>
      <span className="truncate">{node ? deriveLabel(node) : other}</span>
      {node && <span className="shrink-0 text-muted-foreground">({typeLabel(node.type)})</span>}
    </button>
  );
}
