// Path: app/src/components/tab_bar/launch_control.tsx
// Description: Per-repo launch control: the play button with transient feedback and, optionally, the target-menu chevron

import type React from "react";
import { launchFileName } from "../../lib/launch/launch_target.js";
import type { LaunchPhase } from "../../hooks/use_project_launch.js";
import { useProjectLaunchContext } from "./project_launch_context.js";

const PHASE_GLYPH: Record<LaunchPhase, string> = {
  pending: "▶",
  launched: "▶",
  starting: "▶",
  focused: "◉",
  error: "!",
};

interface LaunchControlProps {
  repoId: string;
  /** Distinguishes the tab's control from a dropdown row's for anchoring notices. */
  sourceId: string;
  withMenu: boolean;
}

export function LaunchControl({
  repoId,
  sourceId,
  withMenu,
}: LaunchControlProps): React.JSX.Element | null {
  const launch = useProjectLaunchContext();
  const target = launch.targetFor(repoId);
  if (!target) return null;

  const feedback = launch.feedbackFor(repoId);
  const phase = feedback?.phase ?? null;
  const fileName = launchFileName(target.relativePath);
  const title = phase === "error" && feedback?.message
    ? feedback.message
    : phase === "focused" ? `Focused ${fileName}` : `Launch ${fileName}`;
  const showChevron = withMenu && launch.candidatesFor(repoId).length > 1;
  const isMenuOpen = launch.isMenuOpen(repoId);

  return (
    <span className="launch-control">
      <button
        type="button"
        className={`launch-button${phase ? ` is-${phase}` : ""}`}
        ref={(node) => {
          launch.registerLaunchTrigger(sourceId, node);
        }}
        onClick={(e) => {
          e.stopPropagation();
          launch.launch(repoId, sourceId);
        }}
        disabled={phase === "pending"}
        title={title}
        aria-label={title}
      >
        {phase ? PHASE_GLYPH[phase] : "▶"}
      </button>
      {showChevron && (
        <button
          type="button"
          className="launch-chevron"
          ref={(node) => {
            launch.registerMenuTrigger(repoId, node);
          }}
          onClick={(e) => {
            launch.toggleMenu(e, repoId);
          }}
          aria-expanded={isMenuOpen}
          aria-haspopup="menu"
          title="Choose launch target"
          aria-label="Choose launch target"
        >
          ▾
        </button>
      )}
    </span>
  );
}
