// Path: app/src/components/tab_bar/launch_menu.tsx
// Description: Launch-target chooser listing a repo's candidates by kind and file name, default marked

import type React from "react";
import type { LaunchCandidate } from "../../shared/protocol_launch.js";
import { launchFileName } from "../../lib/launch/launch_target.js";

interface LaunchMenuProps {
  candidates: readonly LaunchCandidate[];
  target: LaunchCandidate;
  accentStyle: React.CSSProperties;
  onChoose: (target: string) => void;
}

export function LaunchMenu({
  candidates,
  target,
  accentStyle,
  onChoose,
}: LaunchMenuProps): React.JSX.Element {
  return (
    <div className="group-dropdown launch-menu" style={accentStyle} role="menu">
      {candidates.map((candidate) => {
        const isDefault = candidate.target === target.target;
        return (
          <button
            key={candidate.target}
            className={`group-dropdown-item ${isDefault ? "selected" : ""}`}
            onClick={() => {
              onChoose(candidate.target);
            }}
            type="button"
            role="menuitem"
            title={`Launch ${candidate.target} and make it the default`}
          >
            <span className="group-radio">{isDefault ? "●" : "○"}</span>
            <span className="launch-menu-kind">{candidate.kind}</span>
            {launchFileName(candidate.target)}
          </button>
        );
      })}
    </div>
  );
}
