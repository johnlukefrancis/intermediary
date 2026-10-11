// Path: app/src/components/source_control/source_control_entry_face.tsx
// Description: Shared changed-path face for SOURCE rows and the Overview Git column: icon, name over dir, change badge

import type React from "react";
import type { SourceControlEntry } from "../../shared/protocol.js";
import { FileIcon, getFileFamily } from "../../lib/icons/index.js";
import { CHANGE_BADGES } from "../../lib/source_control/change_badges.js";
import { isPreviewImagePath } from "../../hooks/repo_workspace_types.js";

export function isDeletedEntry(entry: SourceControlEntry): boolean {
  return entry.change === "deleted";
}

/** A deleted text file has no diff to show; a deleted image still has its previous version */
export function opensDiff(entry: SourceControlEntry): boolean {
  return !isDeletedEntry(entry) || isPreviewImagePath(entry.path);
}

export function entryPathTitle(entry: SourceControlEntry): string {
  return entry.originalPath !== undefined ? `${entry.originalPath} → ${entry.path}` : entry.path;
}

function getFileName(path: string): string {
  return path.split("/").at(-1) ?? path;
}

function getDirectory(path: string): string {
  const index = path.lastIndexOf("/");
  return index === -1 ? "" : path.slice(0, index);
}

interface SourceControlEntryFaceProps {
  entry: SourceControlEntry;
  /** Trailing controls rendered after the badge in the meta cell */
  children?: React.ReactNode;
}

export function SourceControlEntryFace({ entry, children }: SourceControlEntryFaceProps): React.JSX.Element {
  const badge = CHANGE_BADGES[entry.change];
  const directory = getDirectory(entry.path);
  return (
    <>
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
        {children}
      </span>
    </>
  );
}
