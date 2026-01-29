// Path: app/src/tabs/triangle_rain_tab.tsx
// Description: Triangle Rain project tab with worktree selector

import React, { useState } from "react";
import { ThreeColumn } from "../components/layout/three_column";
import type { WorktreeId } from "../shared/protocol";

const WORKTREES: { id: WorktreeId; label: string }[] = [
  { id: "tr-engine", label: "tr-engine" },
];

export function TriangleRainTab(): React.JSX.Element {
  const [selectedWorktree, setSelectedWorktree] =
    useState<WorktreeId>("tr-engine");

  return (
    <div className="tab triangle-rain-tab">
      <div className="worktree-selector">
        <label htmlFor="worktree-select">Worktree:</label>
        <select
          id="worktree-select"
          value={selectedWorktree}
          onChange={(e) => {
            setSelectedWorktree(e.target.value as WorktreeId);
          }}
        >
          {WORKTREES.map((wt) => (
            <option key={wt.id} value={wt.id}>
              {wt.label}
            </option>
          ))}
        </select>
      </div>
      <ThreeColumn />
    </div>
  );
}
