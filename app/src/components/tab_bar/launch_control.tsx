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

function DemoIcon(): React.JSX.Element {
  return (
    <svg className="launch-demo-icon" viewBox="0 0 20 20" fill="none" aria-hidden="true" focusable="false">
      <rect x="2.25" y="3.25" width="15.5" height="13.5" rx="2" stroke="currentColor" strokeWidth="1.25" />
      <path d="M2.75 6.75h14.5" stroke="currentColor" strokeWidth="1.25" />
      <circle cx="4.75" cy="5" r=".65" fill="currentColor" />
      <circle cx="7" cy="5" r=".65" fill="currentColor" />
      <path d="m8 9 5 3-5 3V9Z" fill="currentColor" />
    </svg>
  );
}

interface LaunchControlProps {
  repoId: string;
  /** Distinguishes the tab's control from a dropdown row's for anchoring notices. */
  sourceId: string;
  withMenu: boolean;
  action?: "project" | "demo";
}

export function LaunchControl({
  repoId,
  sourceId,
  withMenu,
  action = "project",
}: LaunchControlProps): React.JSX.Element | null {
  const launch = useProjectLaunchContext();
  const target = action === "demo" ? launch.demoFor(repoId) : launch.targetFor(repoId)?.target;
  if (!target) return null;

  const feedback = launch.feedbackFor(sourceId);
  const phase = feedback?.phase ?? null;
  const fileName = launchFileName(target);
  const title = phase === "error" && feedback?.message
    ? feedback.message
    : phase === "focused" ? `Focused ${fileName}` : `Launch ${action === "demo" ? "HTML demo " : ""}${fileName}`;
  const showChevron = action === "project" && withMenu && launch.candidatesFor(repoId).length > 1;
  const isMenuOpen = launch.isMenuOpen(repoId);

  return (
    <span className="launch-control">
      <button
        type="button"
        className={`launch-button${action === "demo" ? " launch-button--demo" : ""}${phase ? ` is-${phase}` : ""}`}
        ref={(node) => {
          launch.registerLaunchTrigger(sourceId, node);
        }}
        onClick={(e) => {
          e.stopPropagation();
          launch.launch(repoId, sourceId, target);
        }}
        disabled={phase === "pending"}
        title={title}
        aria-label={title}
      >
        {action === "demo" && phase !== "error"
          ? <DemoIcon />
          : phase ? PHASE_GLYPH[phase] : "▶"}
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

export function RepoLaunchControls(props: Omit<LaunchControlProps, "action">): React.JSX.Element {
  return <>
    <LaunchControl {...props} />
    <LaunchControl repoId={props.repoId} sourceId={`demo:${props.sourceId}`} withMenu={false} action="demo" />
  </>;
}
