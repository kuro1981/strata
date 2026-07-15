// alias/ULID ジャンプ(D49 v0 体験 5.)。「検索は v1」なので完全一致のみ。

import { useState } from "react";
import { useGraph } from "@/state/GraphContext";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";

export function JumpBox() {
  const { jump, jumpError } = useGraph();
  const [value, setValue] = useState("");

  const submit = () => {
    if (value.trim()) jump(value);
  };

  return (
    <div className="flex items-center gap-1">
      <Input
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") submit();
        }}
        placeholder="alias / doc/alias / ULID へジャンプ"
        className="h-7 w-56 text-xs"
      />
      <Button size="sm" variant="outline" className="h-7 px-2 text-xs" onClick={submit}>
        Go
      </Button>
      {jumpError && <span className="text-xs text-destructive">{jumpError}</span>}
    </div>
  );
}
