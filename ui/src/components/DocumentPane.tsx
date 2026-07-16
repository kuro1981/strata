// 右ペイン: 文書ペイン(D49 v0 体験 1./6.)。複数文書ワークスペースはタブ切り替え
// (裁量: 連結表示も選べたが、文書境界を曖昧にしない・スクロール位置が文書ごとに
// 独立する、の2点でタブを選んだ。最終報告参照)。

import { useGraph } from "@/state/GraphContext";
import { BlockTree, OrphanNodeCard } from "@/components/blocks/BlockTree";
import { docRootLabel } from "@/lib/label";
import { cn } from "@/lib/utils";

export function DocumentPane() {
  const { idx, activeDoc, setActiveDoc, selected } = useGraph();
  const roots = idx.roots.filter((r) => r.root);

  if (roots.length === 0) {
    return <div className="p-4 text-sm text-muted-foreground">フロントマターを持つ文書がありません。</div>;
  }

  const active = roots.find((r) => r.path === activeDoc) ?? roots[0];
  // term 宣言のように文書の contains 木から到達できないノードが選択された場合、
  // 通常のタブ内容には現れない(スクロール先が無い)ので、専用カードで内容+関係を出す
  // (lib/layout.ts の「文書外」帯とグラフペイン側は既に対応済み)。
  const selectedIsOrphan = selected != null && !idx.docOf.has(selected);

  return (
    <div className="flex h-full flex-col">
      {selectedIsOrphan && (
        <div className="shrink-0 border-b border-border px-3 pt-3">
          <OrphanNodeCard id={selected!} />
        </div>
      )}
      {roots.length > 1 && (
        <div className="flex shrink-0 flex-wrap gap-1 border-b border-border bg-background px-2 py-1.5">
          {roots.map((r) => (
            <button
              key={r.path}
              onClick={() => setActiveDoc(r.path)}
              className={cn(
                "rounded-md px-2 py-1 text-xs font-medium transition-colors",
                r.path === active.path ? "bg-primary text-primary-foreground" : "text-muted-foreground hover:bg-muted",
              )}
              title={r.path}
            >
              {docRootLabel(idx, r)}
            </button>
          ))}
        </div>
      )}
      {/* D53: `doc:` 参照の「先頭スクロール」着地(GraphContext.scrollToSelected)が
          Document ターゲットのときにこのコンテナ自体を scrollTop: 0 へ戻すための目印。 */}
      <div data-doc-scroll className="min-h-0 flex-1 overflow-y-auto px-3 py-3">
        {active.root && <BlockTree id={active.root} />}
      </div>
    </div>
  );
}
