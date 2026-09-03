// Path: app/src/components/source_control/source_control_row.tsx
// Description: One changed-path row: icon, name over dir, change badge, and a hover stage/unstage action

import type React from "react";
import { useCallback } from "react";
import type { SourceControlEntry } from "../../shared/protocol.js";
import { FileIcon, getFileFamily } from "../../lib/icons/index.js";
import { CHANGE_BADGES } from "../../lib/source_control/change_badges.js";
import { isPreviewImagePath } from "../../hooks/repo_workspace_types.js";
import { MinusIcon, PlusIcon } from "./source_control_icons.js";

export type RowActionKind = "stage" | "unstage";

interface SourceControlRowProps {
  entry: SourceControlEntry;
  actionKind: RowActionKind;
  disabled: boolean;
  onAction: (entry: SourceControlEntry) => void;
  onOpenDiff: (entry: SourceControlEntry) => void;
  onContextMenu: (event: React.MouseEvent, entry: SourceControlEntry) => void;
}

function getFileName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

function getDirectory(path: string): string {
  const index = path.lastIndexOf("/");
  return index === -1 ? "" : path.slice(0, index);
}

export function isDeletedEntry(entry: SourceControlEntry): boolean {
  return entry.change === "deleted";
}

export function SourceControlRow({
  entry,
  actionKind,
  disabled,
  onAction,
  onOpenDiff,
  onContextMenu,
}: SourceControlRowProps): React.JSX.Element {
  const badge = CHANGE_BADGES[entry.change];
  const deleted = isDeletedEntry(entry);
  const directory = getDirectory(entry.path);
  const title =
    entry.originalPath !== undefined ? `${entry.originalPath} → ${entry.path}` : entry.path;
  const actionLabel = `${actionKind === "stage" ? "Stage" : "Unstage"} ${entry.path}`;

  // A deleted text file has no diff to show; a deleted image still has its previous version.
  const opensDiff = !deleted || isPreviewImagePath(entry.path);
  const handleDoubleClick = useCallback(
    (event: React.MouseEvent) => {
      if (!opensDiff || (event.target as HTMLElement).closest("button")) return;
      event.preventDefault();
      onOpenDiff(entry);
    },
    [entry, onOpenDiff, opensDiff]
  );

  const handleAction = useCallback(
    (event: React.MouseEvent) => {
      event.stopPropagation();
      if (disabled) return;
      onAction(entry);
    },
    [disabled, entry, onAction]
  );

  return (
    <div
      className="source-control-row"
      role="listitem"
      data-change={entry.change}
      data-deleted={deleted || undefined}
      title={
        deleted
          ? `${title} (deleted)${opensDiff ? " — double-click for image diff" : ""}`
          : `${title} — double-click for diff`
      }
      onDoubleClick={handleDoubleClick}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(event, entry);
      }}
    >
      <span className="source-control-row__icon">
        <FileIcon family={getFileFamily(entry.path)} />
      </span>
      <span className="source-control-row__copy">
        <span className="source-control-row__name">{getFileName(entry.path)}</span>
        {directory && <span className="source-control-row__dir">{directory}</span>}
      </span>
      <span className="source-control-row__meta">
        <span className={`badge badge--${badge.variant}`} title={badge.label}>
          {badge.letter}
        </span>
        <button
          type="button"
          className="source-control-row__action"
          disabled={disabled}
          aria-label={actionLabel}
          title={actionLabel}
          onClick={handleAction}
        >
          {actionKind === "stage" ? <PlusIcon /> : <MinusIcon />}
        </button>
      </span>
    </div>
  );
}
