// Path: app/src/components/bundles/bundle_explorer_row_menu.tsx
// Description: Right-click menu for a ZIPS tree row — file/folder actions plus cut/copy/paste/rename/delete

import type React from "react";
import { ContextMenu, type ContextMenuItem } from "../context_menu.js";
import { buildSingleFileContextMenuItems } from "../file_context_menu_items.js";
import { useFileActions } from "../../hooks/use_file_actions.js";
import type { RepoRoot } from "../../shared/config.js";
import { parentPath } from "../../lib/bundles/bundle_selection_visibility.js";
import type { TreeRowKind } from "./tree_interaction_context.js";

interface BundleExplorerRowMenuProps {
  x: number;
  y: number;
  path: string;
  kind: TreeRowKind;
  repoRoot: RepoRoot;
  selectionCount: number;
  clipboardEmpty: boolean;
  onClose: () => void;
  onCut: () => void;
  onCopy: () => void;
  onPaste: (directory: string) => void;
  onRename: () => void;
  onDelete: () => void;
}

export function BundleExplorerRowMenu({
  x,
  y,
  path,
  kind,
  repoRoot,
  selectionCount,
  clipboardEmpty,
  onClose,
  onCut,
  onCopy,
  onPaste,
  onRename,
  onDelete,
}: BundleExplorerRowMenuProps): React.JSX.Element {
  const fileActions = useFileActions();
  const pasteDirectory = kind === "dir" ? path : parentPath(path);

  const entryItems: ContextMenuItem[] = kind === "file"
    ? buildSingleFileContextMenuItems({
      repoRoot,
      path,
      fileActions,
      logScope: "BundleExplorerRowMenu",
    })
    : [
      {
        label: "Open Containing Folder",
        onClick: () => { void fileActions.revealInFileManager(repoRoot, path); },
      },
      {
        label: "Copy Relative Path",
        onClick: () => {
          void navigator.clipboard.writeText(path).catch((error: unknown) => {
            console.error("[BundleExplorerRowMenu] copy_relative_path failed:", error);
          });
        },
      },
    ];

  const items: ContextMenuItem[] = [
    ...entryItems,
    { label: "Cut", separatorBefore: true, onClick: onCut },
    { label: "Copy", onClick: onCopy },
    { label: "Paste", disabled: clipboardEmpty, onClick: () => { onPaste(pasteDirectory); } },
    { label: "Rename", disabled: selectionCount !== 1, onClick: onRename },
    { label: "Delete", destructive: true, onClick: onDelete },
  ];

  return <ContextMenu x={x} y={y} items={items} onClose={onClose} />;
}
