// Path: app/src/components/bundles/bundle_explorer_file_row.tsx
// Description: File row for the bundle explorer with icon-driven include/exclude toggle

import type React from "react";
import { useCallback } from "react";
import { FileIcon, getFileFamily } from "../../lib/icons/index.js";
import { baseName } from "../../lib/bundles/bundle_selection_visibility.js";
import { useFileDecoration } from "../../hooks/source_control/use_tree_decorations.js";

interface BundleExplorerFileRowProps {
  path: string;
  depth: number;
  enabled: boolean;
  included: boolean;
  onToggle: (path: string) => void;
  onOpen: (path: string) => void;
  onContextMenu: (event: React.MouseEvent, path: string) => void;
}

export function BundleExplorerFileRow({
  path,
  depth,
  enabled,
  included,
  onToggle,
  onOpen,
  onContextMenu,
}: BundleExplorerFileRowProps): React.JSX.Element {
  const handleToggle = useCallback(
    (event: React.MouseEvent) => {
      event.stopPropagation();
      if (!enabled) return;
      onToggle(path);
    },
    [enabled, onToggle, path]
  );

  const handleDoubleClick = useCallback(
    (event: React.MouseEvent) => {
      if ((event.target as HTMLElement).closest("button")) return;
      event.preventDefault();
      onOpen(path);
    },
    [onOpen, path]
  );

  const handleContextMenu = useCallback(
    (event: React.MouseEvent) => {
      event.preventDefault();
      onContextMenu(event, path);
    },
    [onContextMenu, path]
  );

  const family = getFileFamily(path);
  const decoration = useFileDecoration(path);

  return (
    <div
      className={`bundle-explorer-file-row bundle-explorer-row--depth-${Math.min(depth, 4)}`}
      data-included={included || undefined}
      data-disabled={!enabled || undefined}
      data-change={decoration?.variant}
      onDoubleClick={handleDoubleClick}
      onContextMenu={handleContextMenu}
      title={decoration === null ? path : `${path} — ${decoration.label}`}
    >
      <button
        type="button"
        className="bundle-explorer-file-toggle"
        disabled={!enabled}
        aria-label={included ? `Exclude ${path}` : `Include ${path}`}
        aria-pressed={included}
        onClick={handleToggle}
      >
        <FileIcon family={family} />
      </button>
      <span className="bundle-explorer-file-name">{baseName(path)}</span>
      <span className="bundle-explorer-row__meta">
        {decoration !== null && (
          <span className={`badge badge--${decoration.variant}`} title={decoration.label}>
            {decoration.letter}
          </span>
        )}
      </span>
    </div>
  );
}
