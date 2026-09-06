// Path: app/src/components/diff/diff_line_rows.tsx
// Description: The one renderer of parsed diff lines as `.diff-line` rows, shared by the workspace and the stream

import type React from "react";
import type { DiffLine } from "../../lib/diff/diff_lines.js";

interface DiffLineRowsProps {
  lines: readonly DiffLine[];
  /** Publishes each row's index as `--stream-line-index` so a sheet can stagger the print */
  staggerIndex?: boolean | undefined;
  /**
   * Rows from this index on are `data-fresh` — the newest lines of an extended card — and their
   * stagger index restarts at 0 there, so they print even inside a card that is otherwise static.
   */
  freshFrom?: number | undefined;
}

function lineIndexStyle(index: number): React.CSSProperties & {
  "--stream-line-index": string;
} {
  return { "--stream-line-index": String(index) };
}

export function DiffLineRows({ lines, staggerIndex = false, freshFrom }: DiffLineRowsProps): React.JSX.Element {
  const freshAt = freshFrom ?? lines.length;
  return (
    <>
      {lines.map((line, index) => (
        <div
          key={index}
          className="diff-line"
          data-kind={line.kind}
          data-fresh={index >= freshAt || undefined}
          style={staggerIndex ? lineIndexStyle(index >= freshAt ? index - freshAt : index) : undefined}
        >
          <span className="diff-line__no" aria-hidden="true">{line.oldNo ?? ""}</span>
          <span className="diff-line__no" aria-hidden="true">{line.newNo ?? ""}</span>
          <span className="diff-line__text">{line.text}</span>
        </div>
      ))}
    </>
  );
}
