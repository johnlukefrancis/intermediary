// Path: app/src/components/worktree_selector.tsx
// Description: Worktree selector dropdown for Triangle Rain

import type React from "react";
import type { WorktreeId } from "../shared/protocol.js";

interface WorktreeOption {
  id: WorktreeId;
  label: string;
}

interface WorktreeSelectorProps {
  value: WorktreeId;
  options: WorktreeOption[];
  onChange: (value: WorktreeId) => void;
}

export function WorktreeSelector({
  value,
  options,
  onChange,
}: WorktreeSelectorProps): React.JSX.Element {
  return (
    <div className="worktree-selector">
      <label htmlFor="worktree-select">Worktree:</label>
      <select
        id="worktree-select"
        value={value}
        onChange={(event) => {
          onChange(event.target.value as WorktreeId);
        }}
      >
        {options.map((option) => (
          <option key={option.id} value={option.id}>
            {option.label}
          </option>
        ))}
      </select>
    </div>
  );
}
