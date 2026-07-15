// 左ペイン: グラフペイン(D49 v0 体験 2.)。力学レイアウトではなく構造ベース
// (contains 階層をインデントで縦に並べ、意味エッジを曲線オーバーレイ)。
// ズーム/パンは手書きの最小実装(wheel = zoom、drag = pan)。

import { useCallback, useMemo, useRef, useState } from "react";
import { useGraph } from "@/state/GraphContext";
import { computeLayout, INDENT, NODE_R, ROW_H } from "@/lib/layout";
import { deriveLabel } from "@/lib/label";
import { relStyle } from "@/lib/relStyle";
import { isSemanticRel } from "@/types/graph";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const PAD = 16;
const LABEL_W = 220;

export function GraphPane() {
  const { idx, selected, select, isHidden } = useGraph();
  const [tf, setTf] = useState({ scale: 1, x: 0, y: 0 });
  const draggingRef = useRef<{ startX: number; startY: number; ox: number; oy: number } | null>(null);
  const [didDrag, setDidDrag] = useState(false);

  const isVisible = useCallback((id: string) => !isHidden(id), [isHidden]);
  const docLabel = useCallback((r: { alias?: string; path: string }) => r.alias ?? r.path, []);
  const layout = useMemo(() => computeLayout(idx, isVisible, docLabel), [idx, isVisible, docLabel]);

  const edges = useMemo(
    () =>
      idx.edges.filter(
        (e) => isSemanticRel(e.rel) && layout.positions.has(e.from) && layout.positions.has(e.to),
      ),
    [idx, layout],
  );

  const width = PAD * 2 + (layout.maxDepth + 1) * INDENT + LABEL_W;
  const height = PAD * 2 + layout.height;

  const highlighted = useMemo(() => {
    if (!selected) return null;
    const set = new Set<string>([selected]);
    for (const e of edges) {
      if (e.from === selected) set.add(e.to);
      if (e.to === selected) set.add(e.from);
    }
    return set;
  }, [selected, edges]);

  const onWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = -e.deltaY * 0.0012;
    setTf((t) => ({ ...t, scale: Math.min(4, Math.max(0.25, t.scale * (1 + delta))) }));
  };
  const onPointerDown = (e: React.PointerEvent) => {
    draggingRef.current = { startX: e.clientX, startY: e.clientY, ox: tf.x, oy: tf.y };
    setDidDrag(false);
  };
  const onPointerMove = (e: React.PointerEvent) => {
    if (!draggingRef.current) return;
    const dx = e.clientX - draggingRef.current.startX;
    const dy = e.clientY - draggingRef.current.startY;
    if (Math.abs(dx) + Math.abs(dy) > 3) setDidDrag(true);
    setTf((t) => ({ ...t, x: draggingRef.current!.ox + dx, y: draggingRef.current!.oy + dy }));
  };
  const onPointerUp = () => {
    draggingRef.current = null;
  };

  return (
    <div className="relative flex h-full flex-col overflow-hidden">
      <div className="absolute right-2 top-2 z-10 flex gap-1">
        <Button size="sm" variant="outline" onClick={() => setTf((t) => ({ ...t, scale: Math.min(4, t.scale * 1.25) }))}>
          +
        </Button>
        <Button size="sm" variant="outline" onClick={() => setTf((t) => ({ ...t, scale: Math.max(0.25, t.scale / 1.25) }))}>
          −
        </Button>
        <Button size="sm" variant="outline" onClick={() => setTf({ scale: 1, x: 0, y: 0 })}>
          リセット
        </Button>
      </div>

      <div
        className="min-h-0 flex-1 cursor-grab overflow-hidden bg-background active:cursor-grabbing"
        onWheel={onWheel}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerUp}
      >
        <svg width="100%" height="100%" viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="xMinYMin meet">
          <g transform={`translate(${tf.x} ${tf.y}) scale(${tf.scale})`}>
            {/* 文書境界の帯 */}
            {layout.docBands.map((b) => (
              <g key={b.path}>
                <rect
                  x={0}
                  y={b.y0}
                  width={width}
                  height={b.y1 - b.y0}
                  className="fill-muted/30"
                />
                <text x={PAD / 2} y={b.y0 + 14} className="fill-muted-foreground text-[11px] font-semibold">
                  {b.label}
                </text>
              </g>
            ))}

            {/* 意味エッジ(曲線オーバーレイ) */}
            {edges.map((e, i) => {
              const from = layout.positions.get(e.from)!;
              const to = layout.positions.get(e.to)!;
              const x1 = PAD + from.x + NODE_R;
              const y1 = PAD + from.y + ROW_H / 2;
              const x2 = PAD + to.x + NODE_R;
              const y2 = PAD + to.y + ROW_H / 2;
              const midX = Math.max(x1, x2) + 26 + Math.min(30, Math.abs(y2 - y1) * 0.15);
              const style = relStyle(e.rel);
              const dim = highlighted && !(highlighted.has(e.from) && highlighted.has(e.to));
              return (
                <path
                  key={i}
                  d={`M ${x1} ${y1} C ${midX} ${y1}, ${midX} ${y2}, ${x2} ${y2}`}
                  fill="none"
                  stroke={style.color}
                  strokeDasharray={style.dash}
                  strokeWidth={highlighted && highlighted.has(e.from) && highlighted.has(e.to) ? 2.25 : 1.25}
                  opacity={dim ? 0.15 : 0.8}
                />
              );
            })}

            {/* ノード */}
            {[...layout.positions.entries()].map(([id, pos]) => {
              const node = idx.nodes.get(id);
              if (!node) return null;
              const cx = PAD + pos.x + NODE_R;
              const cy = PAD + pos.y + ROW_H / 2;
              const isSelected = selected === id;
              const dim = highlighted ? !highlighted.has(id) : false;
              return (
                <g
                  key={id}
                  transform={`translate(${cx} ${cy})`}
                  className="cursor-pointer"
                  opacity={dim ? 0.35 : 1}
                  onClick={(e) => {
                    e.stopPropagation();
                    if (!didDrag) select(id);
                  }}
                >
                  <circle
                    r={isSelected ? NODE_R + 2 : NODE_R}
                    className={cn(
                      node.type === "document" || node.type === "section" ? "fill-slate-400" : "fill-background",
                      "stroke-slate-500",
                    )}
                    strokeWidth={isSelected ? 2.5 : 1}
                  />
                  <text x={NODE_R + 5} y={4} className="select-none fill-foreground text-[11px]">
                    {deriveLabel(node)}
                  </text>
                </g>
              );
            })}
          </g>
        </svg>
      </div>

      <GraphLegend rels={idx.relsPresent} />
    </div>
  );
}

function GraphLegend({ rels }: { rels: string[] }) {
  if (rels.length === 0) return null;
  return (
    <div className="flex shrink-0 flex-wrap gap-x-3 gap-y-1 border-t border-border bg-background px-2 py-1.5 text-[11px]">
      {rels.map((rel) => {
        const style = relStyle(rel);
        return (
          <span key={rel} className="flex items-center gap-1">
            <svg width="18" height="8">
              <line x1={0} y1={4} x2={18} y2={4} stroke={style.color} strokeWidth={2} strokeDasharray={style.dash} />
            </svg>
            {rel}
          </span>
        );
      })}
    </div>
  );
}
