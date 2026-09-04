// Path: app/src/components/layout/repo_rail.tsx
// Description: Right-rail instrument panel: slim icon-rocker header over the active rail body (zips | source | terminal)

import type React from "react";
import type { ActiveRail } from "../../shared/config.js";
import {
  DeckSectionSwitcher,
  deckSectionTabId,
  type DeckSectionOption,
} from "./deck_section_switcher.js";
import { ZipsIcon, SourceIcon, TerminalIcon } from "./deck_section_icons.js";
import { conflictAlertTitle } from "../source_control/source_control_copy.js";
import "../../styles/repo_rail.css";

const RAIL_PANEL_ID = "repo-rail-panel";
const RAIL_ID_PREFIX = "repo-rail";

/** One body per rail section; the record keeps the union exhaustive when a section is added */
export type RailBodies = Readonly<Record<ActiveRail, React.ReactNode>>;

/** The body the standard rail and the handset deck render for the active section */
export function railBody(rail: ActiveRail, bodies: RailBodies): React.ReactNode {
  return bodies[rail];
}

/** Rail sections shared by the standard rail and the handset deck (which prepends FILES) */
export function buildRailSections(
  sourceCount: number,
  conflictCount: number
): DeckSectionOption<ActiveRail>[] {
  return [
    { value: "zips", label: "ZIPS", icon: <ZipsIcon /> },
    {
      value: "source",
      label: "SOURCE",
      icon: <SourceIcon />,
      count: sourceCount,
      // A conflicted worktree outranks the ordinary count: the cell turns into an alert
      ...(conflictCount > 0
        ? { alert: { count: conflictCount, title: conflictAlertTitle(conflictCount) } }
        : {}),
    },
    { value: "terminal", label: "TERMINAL", icon: <TerminalIcon /> },
  ];
}

interface RepoRailProps {
  activeRail: ActiveRail;
  sourceCount: number;
  sourceConflictCount: number;
  onChangeRail: (rail: ActiveRail) => void;
  bodies: RailBodies;
}

export function RepoRail({
  activeRail,
  sourceCount,
  sourceConflictCount,
  onChangeRail,
  bodies,
}: RepoRailProps): React.JSX.Element {
  return (
    <section className="panel repo-rail" data-panel="rail" data-rail={activeRail}>
      <header className="repo-rail__header">
        <DeckSectionSwitcher
          sections={buildRailSections(sourceCount, sourceConflictCount)}
          active={activeRail}
          onChange={onChangeRail}
          panelId={RAIL_PANEL_ID}
          idPrefix={RAIL_ID_PREFIX}
          ariaLabel="Rail section"
        />
      </header>
      <div
        className="panel-content repo-rail__content"
        role="tabpanel"
        id={RAIL_PANEL_ID}
        aria-labelledby={deckSectionTabId(RAIL_ID_PREFIX, activeRail)}
      >
        {railBody(activeRail, bodies)}
      </div>
    </section>
  );
}
