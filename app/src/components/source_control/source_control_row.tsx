// Path: app/src/components/source_control/source_control_row.tsx
// Description: One SOURCE changed-path row: the shared entry face plus a hover stage/unstage action

import type React from "react";
import { useCallback } from "react";
import type { SourceControlEntry } from "../../shared/protocol.js";
import { MinusIcon, PlusIcon } from "./source_control_icons.js";
import { SourceControlEntryFace, entryPathTitle, isDeletedEntry, opensDiff } from "./source_control_entry_face.js";

export type RowActionKind = "stage" | "unstage";

interface SourceControlRowProps {
  entry: SourceControlEntry;
  actionKind: RowActionKind;
  disabled: boolean;
  onAction: (entry: SourceControlEntry) => void;
  onOpenDiff: (entry: SourceControlEntry) => void;
  onContextMenu: (event: React.MouseEvent, entry: SourceControlEntry) => void;
}

export function SourceControlRow({
  entry,
  actionKind,
  disabled,
  onAction,
  onOpenDiff,
  onContextMenu,
}: SourceControlRowProps): React.JSX.Element {
  const deleted = isDeletedEntry(entry);
  const title = entryPathTitle(entry);
  const actionLabel = `${actionKind === "stage" ? "Stage" : "Unstage"} ${entry.path}`;
  const diffable = opensDiff(entry);
  const handleDoubleClick = useCallback(
    (event: React.MouseEvent) => {
      if (!diffable || (event.target as HTMLElement).closest("button")) return;
      event.preventDefault();
      onOpenDiff(entry);
    },
    [entry, onOpenDiff, diffable]
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
          ? `${title} (deleted)${diffable ? " — double-click for image diff" : ""}`
          : `${title} — double-click for diff`
      }
      onDoubleClick={handleDoubleClick}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(event, entry);
      }}
    >
      <SourceControlEntryFace entry={entry}>
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
      </SourceControlEntryFace>
    </div>
  );
}
