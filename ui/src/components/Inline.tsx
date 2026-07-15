// インライン AST(Inline[])のレンダラ。相互参照(ref/term/anchor)はクリックで
// グラフ⇄文書の選択同期を発火する(D49 の選択同期はブロック粒度だが、インライン参照は
// その参照先ブロックへジャンプする形で同じ仕組みに乗せる)。

import type { Inline as InlineT, MathNode } from "@/types/graph";
import { useGraph } from "@/state/GraphContext";

export function InlineList({ items }: { items: InlineT[] | undefined }) {
  if (!items || items.length === 0) return null;
  return (
    <>
      {items.map((it, i) => (
        <InlineNode key={i} node={it} />
      ))}
    </>
  );
}

export function InlineNode({ node }: { node: InlineT }) {
  const { select } = useGraph();
  switch (node.t) {
    case "text":
      return <>{node.s}</>;
    case "emph": {
      const children = <InlineList items={node.children} />;
      switch (node.kind) {
        case "strong":
          return <strong>{children}</strong>;
        case "em":
          return <em>{children}</em>;
        case "code":
          return <code className="rounded bg-muted px-1 py-0.5 font-mono text-[0.9em]">{children}</code>;
        case "strike":
          return <s>{children}</s>;
        default:
          return <>{children}</>;
      }
    }
    case "math":
      return (
        <span className="font-mono text-[0.95em]">
          <MathInline tree={node.tree} />
        </span>
      );
    case "ref":
      return (
        <button
          type="button"
          className="cursor-pointer rounded text-amber-700 underline decoration-dotted underline-offset-2 hover:bg-amber-100 dark:text-amber-400 dark:hover:bg-amber-950"
          title={`${node.rel} → ${node.to}`}
          onClick={(e) => {
            e.stopPropagation();
            select(node.to);
          }}
        >
          {node.text || "[ref]"}
        </button>
      );
    case "term":
      return (
        <button
          type="button"
          className="cursor-pointer rounded text-violet-700 underline decoration-dotted underline-offset-2 hover:bg-violet-100 dark:text-violet-400 dark:hover:bg-violet-950"
          title={`term-ref → ${node.to}`}
          onClick={(e) => {
            e.stopPropagation();
            select(node.to);
          }}
        >
          {node.text || "[term]"}
        </button>
      );
    case "anchor":
      return (
        <button
          type="button"
          className="cursor-pointer rounded text-cyan-700 underline decoration-dotted underline-offset-2 hover:bg-cyan-100 dark:text-cyan-400 dark:hover:bg-cyan-950"
          onClick={(e) => {
            e.stopPropagation();
            select(node.to);
          }}
        >
          ⚓
        </button>
      );
    case "link":
      return (
        <a href={node.url} target="_blank" rel="noreferrer" className="text-blue-700 underline dark:text-blue-400">
          {node.text || node.url}
        </a>
      );
    case "image":
      // eslint-disable-next-line jsx-a11y/alt-text
      return <img src={node.url} alt={node.alt} loading="lazy" className="my-1 inline-block max-w-full rounded" />;
    default:
      return null;
  }
}

/** 数式(v0: TeX 記法ではなく MathNode 木を等幅テキストへ素朴に線形化する。
 * KaTeX 等の外部依存は入れない — 裁量、最終報告参照)。 */
export function MathInline({ tree }: { tree: MathNode }) {
  return <>{mathToText(tree)}</>;
}

export function mathToText(n: MathNode): string {
  switch (n.op) {
    case "num":
    case "ident":
    case "op":
      return n.v;
    case "text":
      return n.s;
    case "row":
      return n.items.map(mathToText).join(" ");
    case "frac":
      return `(${mathToText(n.num)})/(${mathToText(n.den)})`;
    case "sup":
      return `${mathToText(n.base)}^(${mathToText(n.sup)})`;
    case "sub":
      return `${mathToText(n.base)}_(${mathToText(n.sub)})`;
    case "sub_sup":
      return `${mathToText(n.base)}_(${mathToText(n.sub)})^(${mathToText(n.sup)})`;
    case "under_over": {
      const under = n.under ? `_(${mathToText(n.under)})` : "";
      const over = n.over ? `^(${mathToText(n.over)})` : "";
      return `${mathToText(n.base)}${under}${over}`;
    }
    case "sqrt":
      return `sqrt(${mathToText(n.body)})`;
    case "root":
      return `root(${mathToText(n.index)})(${mathToText(n.radicand)})`;
    case "fenced":
      return `${n.open}${mathToText(n.body)}${n.close}`;
    default:
      return "";
  }
}
