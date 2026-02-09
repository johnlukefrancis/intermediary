// Path: app/src/hooks/use_file_actions.ts
// Description: Hook for OS-level file operations (reveal in file manager, open file)

import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RepoRoot } from "../shared/config/repo_root.js";

interface FileActions {
  revealInFileManager: (root: RepoRoot, relativePath: string) => Promise<void>;
  openFile: (root: RepoRoot, relativePath: string) => Promise<void>;
}

export function useFileActions(): FileActions {
  const revealInFileManager = useCallback(
    async (root: RepoRoot, relativePath: string): Promise<void> => {
      try {
        await invoke("reveal_in_file_manager", { root, relativePath });
      } catch (err) {
        console.error("reveal_in_file_manager failed:", err);
      }
    },
    []
  );

  const openFile = useCallback(
    async (root: RepoRoot, relativePath: string): Promise<void> => {
      try {
        await invoke("open_file", { root, relativePath });
      } catch (err) {
        console.error("open_file failed:", err);
      }
    },
    []
  );

  return { revealInFileManager, openFile };
}
