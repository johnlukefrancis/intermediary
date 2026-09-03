// Path: app/src/components/diff_workspace.tsx
// Description: Read-only unified diff viewer inside the shared workspace shell

import type React from "react";
import { useMemo } from "react";

interface DiffWorkspaceViewerProps {
  path: string;
  isLoading: boolean;
  error: string | null;
  patch?: string | undefined;
  truncated?: boolean | undefined;
  binary?: boolean | undefined;
}

type DiffLineKind = "hunk" | "add" | "del" | "meta" | "context";

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

/** Number of leading `@` in a hunk header minus one: 1 for unified diffs, 2+ for combined (conflict) diffs */
function prefixWidth(hunkHeader: string): number {
  let count = 0;
  while (count < hunkHeader.length && hunkHeader[count] === "@") count += 1;
  return Math.max(1, count - 1);
}

/** Header lines (before the first hunk, or after a new `diff` header) are meta; hunk bodies are +/-/context */
function parsePatch(patch: string): DiffLine[] {
  const normalized = patch.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  const raw = normalized.split("\n");
  if (raw.length > 0 && raw[raw.length - 1] === "") raw.pop();

  let inHunk = false;
  let width = 1;
  let oldNo = 0;
  let newNo = 0;
  const bare = { oldNo: null, newNo: null };
  return raw.map((text): DiffLine => {
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
    const kind: DiffLineKind = prefix.includes("+") ? "add" : inResult ? "context" : "del";
    return { kind, text, oldNo: oldSide, newNo: newSide };
  });
}

export function DiffWorkspaceViewer({
  path,
  isLoading,
  error,
  patch,
  truncated = false,
  binary = false,
}: DiffWorkspaceViewerProps): React.JSX.Element {
  const lines = useMemo(() => (patch === undefined ? [] : parsePatch(patch)), [patch]);

  if (isLoading) {
    return <p className="empty-state empty-state--waiting">Loading diff</p>;
  }

  if (error !== null) {
    return <p className="text-workspace-error text-workspace-error--inline">{error}</p>;
  }

  if (binary) {
    return (
      <div className="diff-workspace">
        <p className="empty-state">Binary file</p>
      </div>
    );
  }

  if (lines.length === 0) {
    return (
      <div className="diff-workspace">
        <p className="empty-state">No diff</p>
      </div>
    );
  }

  return (
    <div className="diff-workspace" role="region" aria-label={`Diff for ${path}`}>
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
