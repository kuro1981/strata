// トップレベル: graph.json を fetch し、2ペイン(グラフ⇄文書)の往復 UI を組み立てる
// (D49/D50)。`strata site` が graph.json と同じディレクトリにこの SPA を書き出すので、
// 常に相対パス `./graph.json` で fetch する(サーバ不要の自己完結配布、vite.config.ts の
// `base: "./"` と対になる)。

import { useEffect, useState, type ReactNode } from "react";
import type { GraphJson } from "@/types/graph";
import { GraphProvider } from "@/state/GraphContext";
import { GraphPane } from "@/components/GraphPane";
import { DocumentPane } from "@/components/DocumentPane";
import { ClassToggle } from "@/components/ClassToggle";
import { JumpBox } from "@/components/JumpBox";
import { TooltipProvider } from "@/components/ui/tooltip";

type LoadState = { status: "loading" } | { status: "error"; message: string } | { status: "ok"; graph: GraphJson };

function App() {
  const [state, setState] = useState<LoadState>({ status: "loading" });

  useEffect(() => {
    fetch("./graph.json")
      .then((res) => {
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        return res.json();
      })
      .then((graph: GraphJson) => setState({ status: "ok", graph }))
      .catch((err: unknown) => setState({ status: "error", message: String(err) }));
  }, []);

  if (state.status === "loading") {
    return <CenterMessage>読み込み中…</CenterMessage>;
  }
  if (state.status === "error") {
    return (
      <CenterMessage>
        graph.json の読み込みに失敗しました: {state.message}
        <div className="mt-2 text-xs text-muted-foreground">
          `strata site` の出力ディレクトリを開いていますか(graph.json が同階層に必要です)。
        </div>
      </CenterMessage>
    );
  }

  return (
    <TooltipProvider>
      <GraphProvider graph={state.graph}>
        <div className="flex h-svh flex-col bg-background text-foreground">
          <header className="flex shrink-0 flex-wrap items-center gap-3 border-b border-border px-3 py-2">
            <span className="text-sm font-semibold">Strata</span>
            <JumpBox />
            <div className="flex-1" />
            <ClassToggle />
          </header>
          <main className="grid min-h-0 flex-1 grid-cols-2">
            <div className="min-h-0 border-r border-border">
              <GraphPane />
            </div>
            <div className="min-h-0">
              <DocumentPane />
            </div>
          </main>
        </div>
      </GraphProvider>
    </TooltipProvider>
  );
}

function CenterMessage({ children }: { children: ReactNode }) {
  return <div className="flex h-svh items-center justify-center p-6 text-center text-sm">{children}</div>;
}

export default App;
