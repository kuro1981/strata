// 左ペイン: グラフペイン。D51(G1.5, sml-spec.md §1.13)の LOD 実装:
//
// - `local`(既定): 選択ノード中心に意味エッジの近傍 1〜2 ホップ(トグル)+ contains の
//   隣接(親・子・兄弟、hop トグルの対象外・常時表示)+ contains の親パス(パンくず)を
//   常に読めるサイズで描く(lib/localLayout.ts、同心円配置。G1.6 で contains 隣接を追加)。
// - `overview`: document/section 粒度に集約し、ブロック間の意味エッジは所属セクション間へ
//   束ねて本数を太さ+バッジで表す(lib/overview.ts + lib/layout.ts の `include` 一般化)。
// - `outline`: 旧来の全展開アウトライン(G1 v0 の実装。デフォルトから第3の選択肢に降格)。
//
// ズーム/パンは手書きの最小実装(wheel = zoom、drag = pan)。モード切替時は座標系が
// 変わるため tf(パン/ズーム)をリセットする。
//
// G1.6(ユーザー目視評価の是正 #3): SVG ルート要素は CSS 既定で `overflow: visible`
// (入れ子 svg と違い `:root` には UA スタイルシートの overflow:hidden が効かない)。
// pan/zoom で `<g>` の transform が viewBox の外まで動くと、そのまま文書ペイン側に
// 描画・ポインタイベントが漏れてリンクを覆ってしまっていたのが実体。svg 要素自身に
// 明示的に `overflow-hidden` を付けて根治する(ラッパー div の overflow-hidden だけでは
// 効かない)。

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useGraph, type GraphMode } from "@/state/GraphContext";
import { computeLayout, INDENT, NODE_R, ROW_H, type DocBand, type LayoutResult } from "@/lib/layout";
import {
  computeLocalLayout,
  nodeRadiusFor,
  MAX_LABEL_CENTER,
  MAX_LABEL_CONTAINS,
  MAX_LABEL_HOP1,
  MAX_LABEL_HOP2,
  type LocalLayoutResult,
  type LocalNodePos,
} from "@/lib/localLayout";
import { isGranular, bundleSemanticEdges, sectionOf, type OverviewEdge } from "@/lib/overview";
import { deriveLabel, docRootLabel, estimateLabelWidth, hoverTitle, truncate } from "@/lib/label";
import { relStyle } from "@/lib/relStyle";
import type { Edge, NodeId, SiteRoot, StrataNode } from "@/types/graph";
import { isSemanticRel } from "@/types/graph";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const PAD = 16;
const LABEL_W = 220;
const LOCAL_PAD = 170; // ローカルグラフはラベルが外周からはみ出すので余白を広めに取る
const LOCAL_MIN_EXTENT = 90;

export function GraphPane() {
  const { idx, selected, select, isHidden, activeDoc, graphMode, setGraphMode, localHops, setLocalHops } = useGraph();
  const [tf, setTf] = useState({ scale: 1, x: 0, y: 0 });
  const draggingRef = useRef<{ startX: number; startY: number; ox: number; oy: number } | null>(null);
  const [didDrag, setDidDrag] = useState(false);

  const isVisible = useCallback((id: string) => !isHidden(id), [isHidden]);
  // G1.7: 文書ラベルは document ノードの title を最優先(alias/path は無題時のみ)。
  // DocumentPane のタブラベルと同じ関数を使い、同じ文書が場所によって別表記に
  // ならないようにする。
  const docLabel = useCallback((r: SiteRoot) => docRootLabel(idx, r), [idx]);

  // モードが変わると座標系(構造ベースの縦アウトライン ⇔ 同心円)も変わるので、
  // パン/ズームを引き継ぐと画面外に飛んでしまう。モード切替のたびにリセットする。
  useEffect(() => {
    setTf({ scale: 1, x: 0, y: 0 });
  }, [graphMode]);

  // --- local: 選択ノード中心(未選択時は文書ルート、D51「文書ルートまたは最初の見出し」の
  // 裁量として文書ルートを採用。常に存在し決定的なため)。 -------------------------------
  const defaultCenter = useMemo<NodeId | null>(() => {
    const root = idx.roots.find((r) => r.path === activeDoc) ?? idx.roots[0];
    return root?.root && isVisible(root.root) ? root.root : null;
  }, [idx, activeDoc, isVisible]);
  const localCenter = selected && isVisible(selected) ? selected : defaultCenter;
  const local = useMemo(
    () => (graphMode === "local" && localCenter ? computeLocalLayout(idx, localCenter, localHops, isVisible) : null),
    [graphMode, idx, localCenter, localHops, isVisible],
  );

  // --- outline: 旧来の全展開(第3モード)。 -------------------------------------------
  const outline = useMemo(
    () => (graphMode === "outline" ? computeLayout(idx, isVisible, docLabel) : null),
    [graphMode, idx, isVisible, docLabel],
  );
  const outlineEdges = useMemo(
    () =>
      outline
        ? idx.edges.filter((e) => isSemanticRel(e.rel) && outline.positions.has(e.from) && outline.positions.has(e.to))
        : [],
    [idx, outline],
  );
  const outlineHighlighted = useMemo(() => {
    if (!selected || !outline) return null;
    const set = new Set<string>([selected]);
    for (const e of outlineEdges) {
      if (e.from === selected) set.add(e.to);
      if (e.to === selected) set.add(e.from);
    }
    return set;
  }, [selected, outline, outlineEdges]);

  // --- overview: document/section 粒度。 --------------------------------------------
  const overview = useMemo(
    () => (graphMode === "overview" ? computeLayout(idx, isVisible, docLabel, (_id, node) => isGranular(node)) : null),
    [graphMode, idx, isVisible, docLabel],
  );
  const overviewEdges = useMemo(
    () => (graphMode === "overview" ? bundleSemanticEdges(idx, isVisible) : []),
    [graphMode, idx, isVisible],
  );
  const overviewHighlight = useMemo(
    () => (selected && overview ? (sectionOf(idx, selected) ?? null) : null),
    [selected, overview, idx],
  );

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
  const onNodeClick = (id: NodeId, andLocalize = false) => {
    if (didDrag) return;
    select(id);
    if (andLocalize) setGraphMode("local");
  };

  const outlineW = outline ? PAD * 2 + (outline.maxDepth + 1) * INDENT + LABEL_W : 0;
  const outlineH = outline ? PAD * 2 + outline.height : 0;
  const overviewW = overview ? PAD * 2 + (overview.maxDepth + 1) * INDENT + LABEL_W : 0;
  const overviewH = overview ? PAD * 2 + overview.height : 0;
  const localExtent = Math.max(LOCAL_MIN_EXTENT, local?.radius ?? 0) + LOCAL_PAD;

  const viewBox =
    graphMode === "local"
      ? `${-localExtent} ${-localExtent} ${localExtent * 2} ${localExtent * 2}`
      : graphMode === "overview"
        ? `0 0 ${overviewW} ${overviewH}`
        : `0 0 ${outlineW} ${outlineH}`;

  return (
    <div className="relative flex h-full flex-col overflow-hidden">
      <div className="flex shrink-0 flex-wrap items-center gap-2 border-b border-border bg-background px-2 py-1.5">
        <ModeSwitch mode={graphMode} onChange={setGraphMode} />
        {graphMode === "local" && (
          <Button
            size="sm"
            variant="outline"
            className="h-7 px-2 text-[11px]"
            onClick={() => setLocalHops(localHops === 1 ? 2 : 1)}
            title="意味エッジを辿るホップ数を切り替える(contains の親・子・兄弟は対象外・常に表示)"
          >
            {localHops} hop
          </Button>
        )}
        {graphMode === "local" && local && local.breadcrumb.length > 0 && (
          <Breadcrumb ids={local.breadcrumb} onJump={(id) => select(id)} />
        )}
      </div>

      <div className="relative min-h-0 flex-1 overflow-hidden">
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
          className="h-full w-full cursor-grab overflow-hidden bg-background active:cursor-grabbing"
          onWheel={onWheel}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerLeave={onPointerUp}
        >
          <svg
            width="100%"
            height="100%"
            viewBox={viewBox}
            preserveAspectRatio="xMidYMid meet"
            className="overflow-hidden"
            style={{ overflow: "hidden" }}
          >
            <g transform={`translate(${tf.x} ${tf.y}) scale(${tf.scale})`}>
              {graphMode === "local" &&
                (local ? (
                  <LocalGraph local={local} selected={selected} onNodeClick={(id) => onNodeClick(id)} />
                ) : (
                  <EmptyMessage text="対象のノードがありません" />
                ))}
              {graphMode === "overview" && overview && (
                <OverviewGraph
                  layout={overview}
                  edges={overviewEdges}
                  width={overviewW}
                  highlight={overviewHighlight}
                  onNodeClick={(id) => onNodeClick(id, true)}
                />
              )}
              {graphMode === "outline" && outline && (
                <OutlineGraph
                  layout={outline}
                  edges={outlineEdges}
                  width={outlineW}
                  selected={selected}
                  highlighted={outlineHighlighted}
                  onNodeClick={(id) => onNodeClick(id)}
                />
              )}
            </g>
          </svg>
        </div>
      </div>

      <GraphLegend mode={graphMode} rels={idx.relsPresent} />
    </div>
  );
}

function EmptyMessage({ text }: { text: string }) {
  return (
    <text x={0} y={0} textAnchor="middle" className="fill-muted-foreground text-sm">
      {text}
    </text>
  );
}

// --- モード切替 UI ------------------------------------------------------------------

const MODE_LABEL: Record<GraphMode, string> = { local: "ローカル", overview: "俯瞰", outline: "全展開" };

function ModeSwitch({ mode, onChange }: { mode: GraphMode; onChange: (m: GraphMode) => void }) {
  return (
    <div className="flex overflow-hidden rounded-md border border-border">
      {(Object.keys(MODE_LABEL) as GraphMode[]).map((m) => (
        <button
          key={m}
          type="button"
          onClick={() => onChange(m)}
          className={cn(
            "px-2.5 py-1 text-[11px] font-medium transition-colors",
            m === mode ? "bg-primary text-primary-foreground" : "bg-background text-muted-foreground hover:bg-muted",
          )}
          aria-pressed={m === mode}
        >
          {MODE_LABEL[m]}
        </button>
      ))}
    </div>
  );
}

function Breadcrumb({ ids, onJump }: { ids: NodeId[]; onJump: (id: NodeId) => void }) {
  const { idx } = useGraph();
  return (
    <div className="flex min-w-0 flex-wrap items-center gap-1 text-[11px] text-muted-foreground">
      {ids.map((id, i) => {
        const node = idx.nodes.get(id);
        return (
          <span key={id} className="flex items-center gap-1">
            {i > 0 && <span className="text-muted-foreground/50">/</span>}
            <button
              type="button"
              onClick={() => onJump(id)}
              title={node ? hoverTitle(node) : id}
              className="max-w-32 truncate hover:text-foreground hover:underline"
            >
              {node ? deriveLabel(node) : id}
            </button>
          </span>
        );
      })}
    </div>
  );
}

// --- local ---------------------------------------------------------------------------

function LocalGraph({
  local,
  selected,
  onNodeClick,
}: {
  local: LocalLayoutResult;
  selected: NodeId | null;
  onNodeClick: (id: NodeId) => void;
}) {
  const { idx } = useGraph();
  const posOf = useMemo(() => new Map(local.nodes.map((n) => [n.id, n])), [local.nodes]);

  return (
    <>
      {local.edges.map((e, i) => {
        const from = posOf.get(e.from);
        const to = posOf.get(e.to);
        if (!from || !to) return null;
        const isContains = e.rel === "contains";
        const style = relStyle(e.rel);
        return (
          <line
            key={i}
            x1={from.x}
            y1={from.y}
            x2={to.x}
            y2={to.y}
            stroke={style.color}
            strokeDasharray={style.dash}
            strokeWidth={isContains ? 1 : 1.5}
            opacity={isContains ? 0.55 : 0.75}
          />
        );
      })}
      {local.nodes.map((n) => (
        <LocalNode key={n.id} pos={n} node={idx.nodes.get(n.id)} isSelected={n.id === selected} onClick={() => onNodeClick(n.id)} />
      ))}
    </>
  );
}

function LocalNode({
  pos,
  node,
  isSelected,
  onClick,
}: {
  pos: LocalNodePos;
  node: StrataNode | undefined;
  isSelected: boolean;
  onClick: () => void;
}) {
  if (!node) return null;
  const isContains = pos.kind === "contains";
  const isCenter = pos.kind === "center";
  const r = nodeRadiusFor(pos.kind, pos.hop);
  // G2.1: 円の不透明度はラベルの不透明度と切り離す(以前は `<g>` にまとめてかけていた
  // ため、contains ラベルが 0.7 の円透明度と掛け合わさって薄すぎて読めなくなっていた —
  // 実機目視評価の是正 #1 の一部)。
  const circleOpacity = isContains ? 0.7 : pos.hop === 2 ? 0.85 : 1;
  const maxLabel =
    pos.kind === "center"
      ? MAX_LABEL_CENTER
      : isContains
        ? MAX_LABEL_CONTAINS
        : pos.hop === 2
          ? MAX_LABEL_HOP2
          : MAX_LABEL_HOP1;
  const fullLabel = deriveLabel(node);
  const shownLabel = truncate(fullLabel, maxLabel);
  const labelW = estimateLabelWidth(shownLabel);

  // G2.1 #1: ラベル位置は localLayout.ts の衝突解決パスが決めた `labelSide`/`labelDy`
  // をそのまま使う(アンカー切替+垂直ナッジ済み)。中心は上、リングは左右。
  const dy = pos.labelDy;
  let textX: number;
  let textAnchor: "start" | "end" | "middle";
  let rectX: number;
  let rectY: number;
  let textY: number;
  if (pos.labelSide === "top") {
    textAnchor = "middle";
    textX = 0;
    const baseline = -(r + 12);
    rectY = baseline - 11 + dy;
    textY = baseline + dy;
    rectX = -labelW / 2 - 3;
  } else if (pos.labelSide === "left") {
    textAnchor = "end";
    textX = -(r + 6);
    rectX = -(r + 3) - (labelW + 6);
    rectY = -8 + dy;
    textY = 4 + dy;
  } else {
    textAnchor = "start";
    textX = r + 6;
    rectX = r + 3;
    rectY = -8 + dy;
    textY = 4 + dy;
  }

  return (
    <g
      style={{ transform: `translate(${pos.x}px, ${pos.y}px)`, transition: "transform 300ms ease" }}
      className="cursor-pointer"
      onClick={(e) => {
        e.stopPropagation();
        onClick();
      }}
    >
      <title>{hoverTitle(node)}</title>
      <circle
        r={isSelected ? r + 2 : r}
        opacity={circleOpacity}
        className={cn(
          pos.kind === "center"
            ? "fill-primary"
            : isContains
              ? "fill-slate-200 dark:fill-slate-700"
              : node.type === "document" || node.type === "section"
                ? "fill-slate-400"
                : "fill-background",
          isContains ? "stroke-slate-400" : "stroke-slate-500",
        )}
        strokeWidth={isSelected ? 2.5 : 1}
        strokeDasharray={isContains ? "2 2" : undefined}
      />
      {/* ラベル背景プレート(白抜き/背景色): リングが混み合ってラベル同士が重なっても
          読めるようにする(G1.6 #2)。 */}
      <rect x={rectX} y={rectY} width={labelW + 6} height={14} rx={3} className="fill-background" opacity={0.88} />
      <text
        x={textX}
        y={textY}
        textAnchor={textAnchor}
        className={cn(
          "select-none text-[11px]",
          // G2.1: contains ラベルは「薄すぎて読めない」フィードバックを受けて
          // fill-muted-foreground(円の 0.7 透明度と掛け合わさっていた)から
          // fill-foreground の 75% 不透明度へ変更(コントラストを一段上げる。
          // 円は circleOpacity で引き続き控えめに描く)。
          isCenter ? "fill-foreground font-semibold" : isContains ? "fill-foreground/75 font-medium" : "fill-foreground",
        )}
      >
        {shownLabel}
      </text>
    </g>
  );
}

// --- overview --------------------------------------------------------------------------

function OverviewGraph({
  layout,
  edges,
  width,
  highlight,
  onNodeClick,
}: {
  layout: LayoutResult;
  edges: OverviewEdge[];
  width: number;
  highlight: NodeId | null;
  onNodeClick: (id: NodeId) => void;
}) {
  const { idx } = useGraph();
  return (
    <>
      <DocBands docBands={layout.docBands} width={width} />
      {edges.map((e, i) => {
        const from = layout.positions.get(e.a);
        const to = layout.positions.get(e.b);
        if (!from || !to) return null;
        const x1 = PAD + from.x + NODE_R;
        const y1 = PAD + from.y + ROW_H / 2;
        const x2 = PAD + to.x + NODE_R;
        const y2 = PAD + to.y + ROW_H / 2;
        const midX = Math.max(x1, x2) + 26 + Math.min(30, Math.abs(y2 - y1) * 0.15);
        const strokeWidth = Math.min(9, 1.5 + e.count * 0.9);
        const dim = highlight ? !(e.a === highlight || e.b === highlight) : false;
        return (
          <g key={i} opacity={dim ? 0.2 : 0.85}>
            <path
              d={`M ${x1} ${y1} C ${midX} ${y1}, ${midX} ${y2}, ${x2} ${y2}`}
              fill="none"
              stroke="#64748b"
              strokeWidth={strokeWidth}
            >
              <title>
                {e.count}件 ({e.rels.join(", ")})
              </title>
            </path>
            <text x={(x1 + x2) / 2 + 20} y={(y1 + y2) / 2} className="fill-muted-foreground text-[10px] font-medium">
              {e.count}
            </text>
          </g>
        );
      })}
      {[...layout.positions.entries()].map(([id, pos]) => {
        const node = idx.nodes.get(id);
        if (!node) return null;
        const cx = PAD + pos.x + NODE_R;
        const cy = PAD + pos.y + ROW_H / 2;
        const isHighlighted = highlight === id;
        const dim = highlight ? !isHighlighted : false;
        return (
          <g
            key={id}
            transform={`translate(${cx} ${cy})`}
            className="cursor-pointer"
            opacity={dim ? 0.4 : 1}
            onClick={(e) => {
              e.stopPropagation();
              onNodeClick(id);
            }}
          >
            <title>{hoverTitle(node)}</title>
            <circle
              r={isHighlighted ? NODE_R + 2 : NODE_R}
              className={cn(node.type === "document" || node.type === "section" ? "fill-slate-400" : "fill-background", "stroke-slate-500")}
              strokeWidth={isHighlighted ? 2.5 : 1}
            />
            <text x={NODE_R + 5} y={4} className="select-none fill-foreground text-[11px]">
              {deriveLabel(node)}
            </text>
          </g>
        );
      })}
    </>
  );
}

// --- outline (旧 v0 実装。第3モード) ------------------------------------------------------

function OutlineGraph({
  layout,
  edges,
  width,
  selected,
  highlighted,
  onNodeClick,
}: {
  layout: LayoutResult;
  edges: Edge[];
  width: number;
  selected: NodeId | null;
  highlighted: Set<string> | null;
  onNodeClick: (id: NodeId) => void;
}) {
  const { idx } = useGraph();
  return (
    <>
      <DocBands docBands={layout.docBands} width={width} />
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
              onNodeClick(id);
            }}
          >
            <title>{hoverTitle(node)}</title>
            <circle
              r={isSelected ? NODE_R + 2 : NODE_R}
              className={cn(node.type === "document" || node.type === "section" ? "fill-slate-400" : "fill-background", "stroke-slate-500")}
              strokeWidth={isSelected ? 2.5 : 1}
            />
            <text x={NODE_R + 5} y={4} className="select-none fill-foreground text-[11px]">
              {deriveLabel(node)}
            </text>
          </g>
        );
      })}
    </>
  );
}

function DocBands({ docBands, width }: { docBands: DocBand[]; width: number }) {
  return (
    <>
      {docBands.map((b) => (
        <g key={b.path}>
          <rect x={0} y={b.y0} width={width} height={b.y1 - b.y0} className="fill-muted/30" />
          <text x={PAD / 2} y={b.y0 + 14} className="fill-muted-foreground text-[11px] font-semibold">
            {b.label}
          </text>
        </g>
      ))}
    </>
  );
}

// --- 凡例 -------------------------------------------------------------------------------

function GraphLegend({ mode, rels }: { mode: GraphMode; rels: string[] }) {
  if (mode === "overview") {
    return (
      <div className="shrink-0 border-t border-border bg-background px-2 py-1.5 text-[11px] text-muted-foreground">
        線の太さ・数字 = 束ねたセクション間の意味エッジ本数(内訳はホバーで表示)
      </div>
    );
  }
  if (rels.length === 0 && mode !== "local") return null;
  return (
    <div className="flex shrink-0 flex-wrap gap-x-3 gap-y-1 border-t border-border bg-background px-2 py-1.5 text-[11px]">
      {mode === "local" &&
        (() => {
          const containsStyle = relStyle("contains");
          return (
            <span className="flex items-center gap-1 text-muted-foreground">
              <svg width="18" height="8">
                <line x1={0} y1={4} x2={18} y2={4} stroke={containsStyle.color} strokeWidth={2} strokeDasharray={containsStyle.dash} />
              </svg>
              contains(構造・常時表示)
            </span>
          );
        })()}
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
