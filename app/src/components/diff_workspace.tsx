// Path: app/src/components/diff_workspace.tsx
// Description: Read-only unified/combined diff viewer inside the shared workspace shell; flags merge conflicts

import type React from "react";
import { useMemo } from "react";
import { conflictNotice, parsePatch, type ParsedPatch } from "../lib/diff/diff_lines.js";
import { DiffLineRows } from "./diff/diff_line_rows.js";

interface DiffWorkspaceViewerProps {
  path: string;
  isLoading: boolean;
  error: string | null;
  patch?: string | undefined;
  truncated?: boolean | undefined;
  binary?: boolean | undefined;
  /** Unmerged path: the patch is a combined diff and its conflict markers are flagged */
  conflict?: boolean | undefined;
}

export function DiffWorkspaceViewer({
  path,
  isLoading,
  error,
  patch,
  truncated = false,
  binary = false,
  conflict = false,
}: DiffWorkspaceViewerProps): React.JSX.Element {
  const { lines, conflictBlocks } = useMemo(
    (): ParsedPatch =>
      patch === undefined ? { lines: [], conflictBlocks: 0 } : parsePatch(patch, conflict),
    [conflict, patch]
  );

  if (isLoading) {
    return <p className="empty-state empty-state--waiting">Loading diff</p>;
  }

  if (error !== null) {
    return <p className="text-workspace-error text-workspace-error--inline">{error}</p>;
  }

  // The conflict affordance belongs to the conflicted path, whatever shape its patch takes
  const notice = conflict ? (
    <p className="diff-workspace__notice" role="alert">
      {conflictNotice({ conflictBlocks, truncated, binary })}
    </p>
  ) : null;

  if (binary) {
    return (
      <div className="diff-workspace">
        {notice}
        <p className="empty-state">Binary file</p>
      </div>
    );
  }

  if (lines.length === 0) {
    return (
      <div className="diff-workspace">
        {notice}
        <p className="empty-state">No diff</p>
      </div>
    );
  }

  return (
    <div className="diff-workspace" role="region" aria-label={`Diff for ${path}`}>
      {notice}
      <div className="diff-workspace__lines">
        <DiffLineRows lines={lines} />
      </div>
      {truncated && (
        <p className="diff-workspace__footer" role="status">
          Diff truncated at 2 MiB
        </p>
      )}
    </div>
  );
}
