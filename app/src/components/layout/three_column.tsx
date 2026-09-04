// Path: app/src/components/layout/three_column.tsx
// Description: The one desktop shell: Auto Files or the shared workspace on the left, a drag divider, the rail on the right

import type React from "react";
import { useCallback, useRef } from "react";
import { DeckSplitter } from "./deck_splitter.js";
import "../../styles/columns.css";

interface ThreeColumnProps {
  /** `files` is the Auto Files grid, `workspace` the wider shared-workspace grid */
  variant?: "files" | "workspace";
  /** Persisted rail share of the deck width; the divider previews and commits it */
  railWidthPercent: number;
  onRailWidthChange: (percent: number) => void;
  fileContent?: React.ReactNode;
  railContent?: React.ReactNode;
}

/**
 * One shell for both desktop states. Opening or closing the workspace only flips the grid class
 * and swaps the left child, so the rail keeps its React position and the ZIPS tree keeps its
 * expansion, selection, and scroll instead of being remounted. The rail width is one CSS
 * variable on the grid: the divider writes it directly while dragging and persists on release.
 */
export function ThreeColumn({
  variant = "files",
  railWidthPercent,
  onRailWidthChange,
  fileContent,
  railContent,
}: ThreeColumnProps): React.JSX.Element {
  const gridRef = useRef<HTMLDivElement | null>(null);
  const previewRailWidth = useCallback((percent: number) => {
    gridRef.current?.style.setProperty("--rail-width", `${String(percent)}%`);
  }, []);

  return (
    <div
      ref={gridRef}
      className={variant === "workspace" ? "text-workspace-layout" : "three-column"}
      style={{ "--rail-width": `${String(railWidthPercent)}%` } as React.CSSProperties}
    >
      {fileContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
      <DeckSplitter
        gridRef={gridRef}
        percent={railWidthPercent}
        onPreview={previewRailWidth}
        onCommit={onRailWidthChange}
      />
      {railContent ?? (
        <section className="panel" data-panel="rail">
          <div className="panel-content">
            <p className="empty-state empty-state--waiting">Waiting for agent</p>
          </div>
        </section>
      )}
    </div>
  );
}
