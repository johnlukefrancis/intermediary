// Path: app/src/hooks/bundles/use_tree_clipboard.ts
// Description: Cut/copy clipboard state for the ZIPS tree — cut moves and clears itself after paste, copy persists

import { useCallback, useEffect, useState } from "react";

export interface TreeClipboardEntry {
  mode: "cut" | "copy";
  paths: string[];
}

export interface TreeClipboardState {
  clipboard: TreeClipboardEntry | null;
  cut: (paths: string[]) => void;
  copy: (paths: string[]) => void;
  clear: () => void;
}

export function useTreeClipboard(repoId: string): TreeClipboardState {
  const [clipboard, setClipboard] = useState<TreeClipboardEntry | null>(null);

  useEffect(() => {
    setClipboard(null);
  }, [repoId]);

  const cut = useCallback((paths: string[]): void => {
    setClipboard({ mode: "cut", paths });
  }, []);

  const copy = useCallback((paths: string[]): void => {
    setClipboard({ mode: "copy", paths });
  }, []);

  const clear = useCallback((): void => {
    setClipboard(null);
  }, []);

  return { clipboard, cut, copy, clear };
}
