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

/** image-support-handoff.md item 6: `ImageFigure.src` / `Inline::Image.url`(いずれも
 * strata-core 側で同じ文字列の意味を共有する、item 3)をブラウザ/webview が読める
 * `<img src>` へ解決するフック。差し替え可能にすることで ui/ 自体は表示環境
 * (静的サイト vs strata-editor の Tauri webview)に依存しない(D54)。
 *
 * 契約: 入力 `src` は次のいずれか — (1) リモート URL(`http`/`https` で始まる)、
 * (2) `strata site` が書き出した相対パス(例: `"assets/foo.png"`)、(3) 未加工の
 * ローカル絶対ファイルシステムパス(`strata build`/`strata view` の生JSONをそのまま
 * 読ませた場合 — `strata site` を経由していないので item 7 のコピー/相対パス化を
 * 受けていない)。戻り値は `<img src>` にそのまま渡せる文字列、または空文字列
 * (呼び出し側はこれを「解決不能・画像を出さない」の合図として扱う — 壊れた
 * `<img>` タグを出さないため)。
 *
 * 呼び出し側は関数のシグネチャ `(src: string) => string` を正確に維持すること
 * (Phase B で strata-editor が Tauri の `convertFileSrc` 等に差し替える契約)。 */
export type ResolveImageSrc = (src: string) => string;

/** `resolveImageSrc` の既定実装(static サイト文脈)。
 * - `http`/`https` の URL はそのまま(リモート画像、常にブラウザから直接読める)。
 * - 先頭が `/` でない(≒ 相対パス、`strata site` が item 7 で書き出す
 *   `"assets/foo.png"` 形式)もそのまま — `<img>` の相対 src は現在の HTML の
 *   場所を基準にブラウザが解決してくれる。
 * - 先頭が `/` の未加工の絶対ファイルシステムパス(`strata site` を経由していない
 *   生の graph JSON を直接読ませた場合)はブラウザから読めないため空文字列
 *   (「画像なしプレースホルダ」を表示させる合図、壊れた `<img>` を出さない)。 */
export function defaultResolveImageSrc(src: string): string {
  if (src.startsWith("http")) return src;
  if (!src.startsWith("/")) return src;
  return "";
}

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
  /** image-support-handoff.md item 6。 */
  resolveImageSrc: ResolveImageSrc;
}

const GraphContext = createContext<GraphContextValue | null>(null);

export function useGraph(): GraphContextValue {
  const ctx = useContext(GraphContext);
  if (!ctx) throw new Error("useGraph must be used within GraphProvider");
  return ctx;
}

export function GraphProvider({
  graph,
  children,
  resolveImageSrc = defaultResolveImageSrc,
}: {
  graph: GraphJson;
  children: ReactNode;
  /** image-support-handoff.md item 6: 省略時は `defaultResolveImageSrc`(static サイト
   * 向け)。strata-editor(Phase B)はここへ Tauri 向けの実装を渡して上書きする。 */
  resolveImageSrc?: ResolveImageSrc;
}) {
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
        // 実機フィードバック(2026-07-29): 読みたい対象をクリックしているのに
        // 画面が動くと辛い、との報告。従来は無条件に block: "center" へ
        // scrollIntoView していたため、既に画面内に見えている要素をクリック
        // しても中央へ寄せ直されて動いてしまっていた。既に(先頭が)見えている
        // ときは動かさず、動かす必要があるときも中央でなく先頭合わせにする
        // (読みたいのは要素の先頭であって中央ではないため)。
        const scrollParent = el.closest<HTMLElement>("[data-doc-scroll]");
        if (scrollParent) {
          const elRect = el.getBoundingClientRect();
          const parentRect = scrollParent.getBoundingClientRect();
          const topAlreadyVisible = elRect.top >= parentRect.top && elRect.top <= parentRect.bottom;
          if (!topAlreadyVisible) {
            el.scrollIntoView({ behavior: "smooth", block: "start" });
          }
        } else {
          el.scrollIntoView({ behavior: "smooth", block: "nearest" });
        }
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
    resolveImageSrc,
  };

  return <GraphContext.Provider value={value}>{children}</GraphContext.Provider>;
}
