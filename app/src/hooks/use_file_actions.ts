// Path: app/src/hooks/use_file_actions.ts
// Description: Hook for OS-level file operations (reveal in file manager, open file)

import { useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RepoRoot } from "../shared/config/repo_root.js";
import { useConfig } from "./use_config.js";

interface FileActions {
  revealInFileManager: (root: RepoRoot, relativePath: string) => Promise<void>;
  openFile: (root: RepoRoot, relativePath: string) => Promise<void>;
  openFiles: (root: RepoRoot, relativePaths: readonly string[]) => Promise<void>;
}

export function useFileActions(): FileActions {
  const {
    config: { agentDistro },
  } = useConfig();

  const revealInFileManager = useCallback(
    async (root: RepoRoot, relativePath: string): Promise<void> => {
      try {
        await invoke("reveal_in_file_manager", {
          root,
          relativePath,
          distroOverride: agentDistro,
        });
      } catch (err) {
        console.error("reveal_in_file_manager failed:", err);
      }
    },
    [agentDistro]
  );

  const openFile = useCallback(
    async (root: RepoRoot, relativePath: string): Promise<void> => {
      try {
        await invoke("open_file", {
          root,
          relativePath,
          distroOverride: agentDistro,
        });
      } catch (err) {
        console.error("open_file failed:", err);
      }
    },
    [agentDistro]
  );

  const openFiles = useCallback(
    async (root: RepoRoot, relativePaths: readonly string[]): Promise<void> => {
      if (relativePaths.length === 0) return;

      try {
        await invoke("open_files", {
          root,
          relativePaths,
          distroOverride: agentDistro,
        });
      } catch (err) {
        console.error("open_files failed:", err);
      }
    },
    [agentDistro]
  );

  return { revealInFileManager, openFile, openFiles };
}
