// Path: app/src/components/source_control/source_control_context_menu.ts
// Description: Right-click menu items for a source-control row, composed over the shared file items

import type { SourceControlEntry } from "../../shared/protocol.js";
import type { RepoRoot } from "../../shared/config.js";
import type { useFileActions } from "../../hooks/use_file_actions.js";
import type { ContextMenuItem } from "../context_menu.js";
import { buildSingleFileContextMenuItems } from "../file_context_menu_items.js";
import { isDeletedEntry } from "./source_control_row.js";

const DELETED_DISABLED_LABELS = new Set(["Open File", "Open Containing Folder"]);

interface SourceControlMenuInput {
  entry: SourceControlEntry;
  repoRoot: RepoRoot;
  fileActions: ReturnType<typeof useFileActions>;
  /** An action is pending or status is not ready: stage/unstage/discard are disabled */
  actionsDisabled: boolean;
  onStage: (entry: SourceControlEntry) => void;
  onUnstage: (entry: SourceControlEntry) => void;
  onOpenDiff: (entry: SourceControlEntry) => void;
  onDiscard: (entry: SourceControlEntry) => void;
}

export function buildSourceControlContextMenuItems({
  entry,
  repoRoot,
  fileActions,
  actionsDisabled,
  onStage,
  onUnstage,
  onOpenDiff,
  onDiscard,
}: SourceControlMenuInput): ContextMenuItem[] {
  const deleted = isDeletedEntry(entry);
  const stageItem: ContextMenuItem =
    entry.area === "index"
      ? { label: "Unstage", disabled: actionsDisabled, onClick: () => { onUnstage(entry); } }
      : { label: "Stage", disabled: actionsDisabled, onClick: () => { onStage(entry); } };

  const fileItems = buildSingleFileContextMenuItems({
    repoRoot,
    path: entry.path,
    fileActions,
    logScope: "SourceControlColumn",
  }).map((item) =>
    deleted && DELETED_DISABLED_LABELS.has(item.label) ? { ...item, disabled: true } : item
  );

  const items: ContextMenuItem[] = [
    stageItem,
    // A deleted row has no working-tree side to open.
    { label: "Open Diff", disabled: deleted, onClick: () => { onOpenDiff(entry); } },
    ...fileItems,
  ];

  if (entry.area === "worktree") {
    items.push({
      label: "Discard Changes",
      disabled: actionsDisabled,
      onClick: () => { onDiscard(entry); },
    });
  }

  return items;
}
