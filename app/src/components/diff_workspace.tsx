// Path: app/src/components/diff_workspace.tsx
// Description: Read-only unified/combined diff viewer inside the shared workspace shell; flags merge conflicts

import type React from "react";
import { useMemo } from "react";

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

type DiffLineKind = "hunk" | "add" | "del" | "meta" | "context" | "marker";

interface DiffLine {
  kind: DiffLineKind;
  text: string;
  /** Line numbers in the old and new file; absent for headers, hunks, and the missing side */
  oldNo: number | null;
  newNo: number | null;
}

/** `@@ -old[,n] +new[,n] @@` (combined diffs list several `-` ranges; the first old and the `+` count) */
function hunkStart(hunkHeader: string): { oldNo: number; newNo: number } {
  const oldMatch = /-(\d+)/.exec(hunkHeader);
  const newMatch = /\+(\d+)/.exec(hunkHeader);
  return {
    oldNo: oldMatch === null ? 1 : Number(oldMatch[1]),
    newNo: newMatch === null ? 1 : Number(newMatch[1]),
  };
}

interface ParsedPatch {
  lines: DiffLine[];
  /** `<<<<<<<` markers still in the file; zero means the markers are gone but the path is unstaged */
  conflictBlocks: number;
}

/**
 * Only consulted for conflicted files, and only inside an open `<<<<<<<` block for the inner
 * markers: elsewhere a bare `=======` is ordinary text.
 */
function conflictMarker(body: string, blockOpen: boolean): "open" | "inner" | "close" | null {
  if (body.startsWith("<<<<<<< ")) return "open";
  if (!blockOpen) return null;
  if (body.startsWith(">>>>>>> ")) return "close";
  if (body === "=======" || body.startsWith("||||||| ")) return "inner";
  return null;
}

interface ConflictNoticeInput {
  conflictBlocks: number;
  truncated: boolean;
  binary: boolean;
}

/** A cut patch may hide markers, so it never claims "resolved"; a binary conflict has no markers at all */
export function conflictNotice({ conflictBlocks, truncated, binary }: ConflictNoticeInput): string {
  if (binary) return "Merge conflict · binary file · keep one version, then stage it to mark it resolved";
  const blocks = `${conflictBlocks} unresolved block${conflictBlocks === 1 ? "" : "s"}`;
  if (truncated) {
    const seen = conflictBlocks === 0 ? "no markers in the first 2 MiB" : `at least ${blocks}`;
    return `Merge conflict · diff truncated · ${seen} · resolve the markers in the file, then stage it`;
  }
  if (conflictBlocks === 0) return "Merge conflict · markers resolved · stage the file to mark it resolved";
  return `Merge conflict · ${blocks} · resolve the markers in the file, then stage it`;
}

/** Number of leading `@` in a hunk header minus one: 1 for unified diffs, 2+ for combined (conflict) diffs */
function prefixWidth(hunkHeader: string): number {
  let count = 0;
  while (count < hunkHeader.length && hunkHeader[count] === "@") count += 1;
  return Math.max(1, count - 1);
}

/** Header lines (before the first hunk, or after a new `diff` header) are meta; hunk bodies are +/-/context */
function parsePatch(patch: string, conflict: boolean): ParsedPatch {
  const normalized = patch.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const raw = normalized.split("\n");
  if (raw.length > 0 && raw[raw.length - 1] === "") raw.pop();

  let inHunk = false;
  let width = 1;
  let oldNo = 0;
  let newNo = 0;
  let conflictBlocks = 0;
  let blockOpen = false;
  const bare = { oldNo: null, newNo: null };
  const lines = raw.map((text): DiffLine => {
    if (text.startsWith("diff ")) {
      inHunk = false;
      return { kind: "meta", text, ...bare };
    }
    if (text.startsWith("@@")) {
      inHunk = true;
      width = prefixWidth(text);
      ({ oldNo, newNo } = hunkStart(text));
      return { kind: "hunk", text, ...bare };
    }
    if (!inHunk || text.startsWith("\\ No newline")) return { kind: "meta", text, ...bare };
    // Combined diffs carry one prefix column per parent: `-` = in that parent but not the result,
    // `+` = in the result but not that parent, ` ` = in both (or in neither, on removed rows).
    // The old gutter follows parent 1; the new gutter follows the result.
    const prefix = text.slice(0, width);
    const inResult = !prefix.includes("-");
    const inOld = prefix[0] === "-" || (prefix[0] === " " && inResult);
    const oldSide = inOld ? oldNo++ : null;
    const newSide = inResult ? newNo++ : null;
    const marker = conflict ? conflictMarker(text.slice(width), blockOpen) : null;
    if (marker !== null) {
      if (marker === "open") conflictBlocks += 1;
      blockOpen = marker !== "close";
      return { kind: "marker", text, oldNo: oldSide, newNo: newSide };
    }
    const kind: DiffLineKind = prefix.includes("+") ? "add" : inResult ? "context" : "del";
    return { kind, text, oldNo: oldSide, newNo: newSide };
  });
  return { lines, conflictBlocks };
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
        {lines.map((line, index) => (
          <div key={index} className="diff-line" data-kind={line.kind}>
            <span className="diff-line__no" aria-hidden="true">{line.oldNo ?? ""}</span>
            <span className="diff-line__no" aria-hidden="true">{line.newNo ?? ""}</span>
            <span className="diff-line__text">{line.text}</span>
          </div>
        ))}
      </div>
      {truncated && (
        <p className="diff-workspace__footer" role="status">
          Diff truncated at 2 MiB
        </p>
      )}
    </div>
  );
}
