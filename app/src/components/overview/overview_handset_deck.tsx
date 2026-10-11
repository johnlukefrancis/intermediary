// Path: app/src/components/overview/overview_handset_deck.tsx
// Description: Handset Overview deck: filter toolbar, then FEED and GIT sections in the handset chassis, both kept mounted behind the switcher

import type React from "react";
import { useState } from "react";
import type { RepoConfig } from "../../shared/config.js";
import { useOverviewGitTotals } from "../../hooks/overview/use_overview_git_totals.js";
import { conflictAlertTitle } from "../source_control/source_control_copy.js";
import { DeckSectionSwitcher, deckSectionTabId, type DeckSectionOption } from "../layout/deck_section_switcher.js";
import { FilesIcon, SourceIcon } from "../layout/deck_section_icons.js";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

type OverviewSection = "feed" | "git";

const PANEL_ID = "overview-handset-panel";
const ID_PREFIX = "overview-handset";

interface OverviewHandsetDeckProps {
  shown: readonly RepoConfig[];
  toolbar: React.ReactNode;
  feed: React.ReactNode;
  git: React.ReactNode;
}

export function OverviewHandsetDeck({ shown, toolbar, feed, git }: OverviewHandsetDeckProps): React.JSX.Element {
  const [active, setActive] = useState<OverviewSection>("feed");
  const totals = useOverviewGitTotals(shown);
  const sections: DeckSectionOption<OverviewSection>[] = [
    { value: "feed", label: "FEED", icon: <FilesIcon /> },
    {
      value: "git", label: "GIT", icon: <SourceIcon />, count: totals.changes,
      ...(totals.conflicts > 0 ? { alert: { count: totals.conflicts, title: conflictAlertTitle(totals.conflicts) } } : {}),
    },
  ];
  return (
    <div className="handset-deck overview-handset">
      <div className="handset-chassis">
        {toolbar}
        <div className="overview-handset__switch">
          <DeckSectionSwitcher sections={sections} active={active} onChange={setActive}
            panelId={PANEL_ID} idPrefix={ID_PREFIX} ariaLabel="Overview section" />
        </div>
        <div className="handset-deck__tabpanel" role="tabpanel" id={PANEL_ID} aria-labelledby={deckSectionTabId(ID_PREFIX, active)}>
          <div className="overview-handset__section" hidden={active !== "feed"}>{feed}</div>
          <div className="overview-handset__section" hidden={active !== "git"}>{git}</div>
        </div>
      </div>
    </div>
  );
}
