// Path: app/src/components/bundles/bundle_file_context_menu.tsx
// Description: Context menu actions for bundle explorer file rows

import type React from "react";
import { buildSingleFileContextMenuItems } from "../file_context_menu_items.js";
import { ContextMenu } from "../context_menu.js";
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
  const items = buildSingleFileContextMenuItems({
    repoRoot,
    path,
    fileActions,
    logScope: "BundleFileContextMenu",
  });

  return <ContextMenu x={x} y={y} items={items} onClose={onClose} />;
}
