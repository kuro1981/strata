// class トグル(D49 v0 体験 4.)。存在する class をチップで一覧し、OFF にすると
// 実効 class(D46: 自身+祖先)が該当するサブツリーが両ペインから消える
// (`render --hide` のインタラクティブ版)。

import { useGraph } from "@/state/GraphContext";
import { Toggle } from "@/components/ui/toggle";

export function ClassToggle() {
  const { idx, hiddenClasses, toggleClass } = useGraph();
  if (idx.allClasses.length === 0) return null;

  return (
    <div className="flex flex-wrap items-center gap-1">
      <span className="mr-1 text-xs text-muted-foreground">class:</span>
      {idx.allClasses.map((c) => {
        const on = !hiddenClasses.has(c);
        return (
          <Toggle
            key={c}
            pressed={on}
            onPressedChange={() => toggleClass(c)}
            size="sm"
            variant="outline"
            className="h-6 px-2 text-[11px] data-[state=on]:border-primary data-[state=on]:bg-primary/10"
            title={on ? `'${c}' を非表示にする` : `'${c}' を再表示する`}
          >
            {c}
          </Toggle>
        );
      })}
    </div>
  );
}
