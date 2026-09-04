// Path: app/src/components/layout/three_column.tsx
// Description: The one desktop shell: Auto Files or the shared workspace on the left, the rail on the right

import type React from "react";
import "../../styles/columns.css";

interface ThreeColumnProps {
  /** `files` is the Auto Files grid, `workspace` the wider shared-workspace grid */
  variant?: "files" | "workspace";
  fileContent?: React.ReactNode;
  railContent?: React.ReactNode;
}

/**
 * One shell for both desktop states. Opening or closing the workspace only flips the grid class
 * and swaps the left child, so the rail keeps its React position and the ZIPS tree keeps its
 * expansion, selection, and scroll instead of being remounted.
 */
export function ThreeColumn({
  variant = "files",
  fileContent,
  railContent,
}: ThreeColumnProps): React.JSX.Element {
  return (
    <div className={variant === "workspace" ? "text-workspace-layout" : "three-column"}>
      {fileContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
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
