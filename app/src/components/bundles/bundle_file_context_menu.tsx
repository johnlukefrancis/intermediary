// Path: app/src/components/bundles/bundle_file_context_menu.tsx
// Description: Context menu actions for bundle explorer file rows

import type React from "react";
import { ContextMenu, type ContextMenuItem } from "../context_menu.js";
import { useFileActions } from "../../hooks/use_file_actions.js";
import type { RepoRoot } from "../../shared/config.js";

interface BundleFileContextMenuProps {
  x: number;
  y: number;
  path: string;
  repoRoot: RepoRoot;
  onClose: () => void;
}

export function BundleFileContextMenu({
  x,
  y,
  path,
  repoRoot,
  onClose,
}: BundleFileContextMenuProps): React.JSX.Element {
  const fileActions = useFileActions();
  const items: ContextMenuItem[] = [
    {
      label: "Open Containing Folder",
      onClick: () => { void fileActions.revealInFileManager(repoRoot, path); },
    },
    {
      label: "Open File",
      onClick: () => { void fileActions.openFile(repoRoot, path); },
    },
    {
      label: "Copy Relative Path",
      onClick: () => {
        void navigator.clipboard.writeText(path).catch((error: unknown) => {
          console.error("[BundleFileContextMenu] copy_relative_path failed:", error);
        });
      },
    },
  ];

  return <ContextMenu x={x} y={y} items={items} onClose={onClose} />;
}
