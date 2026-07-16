// アプリ全体の選択状態(2ペイン同期の要)。React Context で GraphPane/DocumentPane/
// RelationPanel 間のプロップドリリングを避ける(裁量)。
//
// 同期方式の要約:
// - `selected` が唯一の真実の源。グラフのノードクリック・文書ペインのブロッククリック・
//   関係パネルのジャンプ・alias/ULID ジャンプ、全て `select()` を呼ぶだけ。
// - `select()` は選択ノードの所属文書へ `activeDoc` を切り替え、次の effect で
//   文書ペイン側の対応要素までスクロール+ハイライトする(`registerBlockRef` で
//   登録された DOM 要素を辿る)。
// - class トグルは `hiddenClasses`(OFF にした class の集合)を持ち、
//   `isHidden(id)` は D46 の実効 class(自身+祖先)がそれと交差するかで判定する
//   (祖先ごと消える = サブツリーごと消える)。
// - D51(G1.5): グラフペインの詳細度は `graphMode`(local/overview/outline)。
//   `local` は選択中心の近傍グラフで `localHops`(1〜2)まで辿る。モード・ホップ数の
//   状態もここに集約し、GraphPane.tsx から参照する(class トグルと同じ理由: 将来
//   他コンポーネントから参照する可能性に備える)。

import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react";
import type { NodeId } from "@/types/graph";
import { buildIndex, effectiveClasses, resolveJumpTarget, type GraphIndex } from "@/lib/graphIndex";
import type { GraphJson } from "@/types/graph";

/** グラフペインの詳細度(D51 G1.5)。既定は `local`(ローカルグラフ)。
 * `overview` はセクション粒度、`outline` は旧来の全展開(オプションに降格)。 */
export type GraphMode = "local" | "overview" | "outline";

interface GraphContextValue {
  idx: GraphIndex;
  selected: NodeId | null;
  select: (id: NodeId | null) => void;
  activeDoc: string | null;
  setActiveDoc: (path: string) => void;
  hiddenClasses: Set<string>;
  toggleClass: (cls: string) => void;
  isHidden: (id: NodeId) => boolean;
  registerBlockRef: (id: NodeId, el: HTMLElement | null) => void;
  jumpError: string | null;
  jump: (query: string) => void;
  /** D51: グラフペインの詳細度モード。 */
  graphMode: GraphMode;
  setGraphMode: (mode: GraphMode) => void;
  /** D51: ローカルグラフのホップ数(1〜2、トグル)。 */
  localHops: 1 | 2;
  setLocalHops: (hops: 1 | 2) => void;
}

const GraphContext = createContext<GraphContextValue | null>(null);

export function useGraph(): GraphContextValue {
  const ctx = useContext(GraphContext);
  if (!ctx) throw new Error("useGraph must be used within GraphProvider");
  return ctx;
}

export function GraphProvider({ graph, children }: { graph: GraphJson; children: ReactNode }) {
  const idx = useMemo(() => buildIndex(graph), [graph]);
  const [selected, setSelected] = useState<NodeId | null>(null);
  const [activeDoc, setActiveDocState] = useState<string | null>(idx.roots[0]?.path ?? null);
  const [hiddenClasses, setHiddenClasses] = useState<Set<string>>(new Set());
  const [jumpError, setJumpError] = useState<string | null>(null);
  const [graphMode, setGraphMode] = useState<GraphMode>("local");
  const [localHops, setLocalHops] = useState<1 | 2>(1);
  const blockRefs = useRef(new Map<NodeId, HTMLElement>());

  const hiddenNodeIds = useMemo(() => {
    if (hiddenClasses.size === 0) return new Set<NodeId>();
    const bad = new Set<NodeId>();
    for (const id of idx.nodes.keys()) {
      const eff = effectiveClasses(idx, id);
      for (const c of eff) {
        if (hiddenClasses.has(c)) {
          bad.add(id);
          break;
        }
      }
    }
    return bad;
  }, [idx, hiddenClasses]);

  const isHidden = useCallback((id: NodeId) => hiddenNodeIds.has(id), [hiddenNodeIds]);

  const setActiveDoc = useCallback((path: string) => setActiveDocState(path), []);

  const registerBlockRef = useCallback((id: NodeId, el: HTMLElement | null) => {
    if (el) blockRefs.current.set(id, el);
    else blockRefs.current.delete(id);
  }, []);

  const scrollToSelected = useCallback(
    (id: NodeId) => {
      // タブ切り替え直後は対象要素がまだ DOM に無いことがあるので次フレームまで待つ。
      requestAnimationFrame(() => {
        const el = blockRefs.current.get(id);
        if (!el) return;
        // D53: `doc:` 参照(Document ノード直指し)のクリックは「文書先頭へスクロール」
        // であるべきだが、Document ノード自身は文書ペインの最外周コンテナ(文書全体を
        // 包む巨大な div)として登録されている。通常の scrollIntoView(block: "center")
        // だと長い文書ではその巨大な div の「中央」に合わせようとして途中までスクロール
        // してしまう。Document ターゲットだけは文書ペインのスクロールコンテナ自体を
        // 先頭(scrollTop: 0)へ戻す特別扱いにする(裁量、最終報告参照)。
        if (idx.nodes.get(id)?.type === "document") {
          const scrollParent = el.closest<HTMLElement>("[data-doc-scroll]");
          (scrollParent ?? el).scrollTo({ top: 0, behavior: "smooth" });
          return;
        }
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        el.classList.add("strata-block-flash");
        window.setTimeout(() => el.classList.remove("strata-block-flash"), 1200);
      });
    },
    [idx],
  );

  const select = useCallback(
    (id: NodeId | null) => {
      setSelected(id);
      if (id) {
        const doc = idx.docOf.get(id);
        if (doc && doc !== activeDoc) {
          setActiveDocState(doc);
          // タブ切替のレンダーを待ってからスクロール。
          setTimeout(() => scrollToSelected(id), 30);
        } else {
          scrollToSelected(id);
        }
      }
    },
    [idx, activeDoc, scrollToSelected],
  );

  const toggleClass = useCallback((cls: string) => {
    setHiddenClasses((prev) => {
      const next = new Set(prev);
      if (next.has(cls)) next.delete(cls);
      else next.add(cls);
      return next;
    });
  }, []);

  const jump = useCallback(
    (query: string) => {
      const target = resolveJumpTarget(idx, query);
      if (!target) {
        setJumpError(`'${query}' は見つかりませんでした`);
        return;
      }
      setJumpError(null);
      select(target);
    },
    [idx, select],
  );

  const value: GraphContextValue = {
    idx,
    selected,
    select,
    activeDoc,
    setActiveDoc,
    hiddenClasses,
    toggleClass,
    isHidden,
    registerBlockRef,
    jumpError,
    jump,
    graphMode,
    setGraphMode,
    localHops,
    setLocalHops,
  };

  return <GraphContext.Provider value={value}>{children}</GraphContext.Provider>;
}
