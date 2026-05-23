// Path: app/src/components/file_context_menu_items.ts
// Description: Shared context-menu item builders for repo-relative file actions

import type { useFileActions } from "../hooks/use_file_actions.js";
import type { RepoRoot } from "../shared/config.js";
import type { ContextMenuItem } from "./context_menu.js";

type FileActions = ReturnType<typeof useFileActions>;

interface SingleFileMenuInput {
  repoRoot: RepoRoot;
  path: string;
  fileActions: FileActions;
  logScope: string;
}

export function buildSingleFileContextMenuItems({
  repoRoot,
  path,
  fileActions,
  logScope,
}: SingleFileMenuInput): ContextMenuItem[] {
  return [
    {
      label: "Open Containing Folder",
      onClick: () => {
        void fileActions.revealInFileManager(repoRoot, path);
      },
    },
    {
      label: "Open File",
      onClick: () => {
        void fileActions.openFile(repoRoot, path);
      },
    },
    {
      label: "Copy Relative Path",
      onClick: () => {
        void navigator.clipboard.writeText(path).catch((error: unknown) => {
          console.error(`[${logScope}] copy_relative_path failed:`, error);
        });
      },
    },
  ];
}
