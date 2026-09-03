// Path: app/src/components/layout/handset_deck.tsx
// Description: Handset deck layout switching between Auto files, zip bundles, and source control

import type React from "react";
import type { DeckSection } from "../../hooks/use_deck_section.js";
import {
  DeckSectionSwitcher,
  deckSectionTabId,
  type DeckSectionOption,
} from "./deck_section_switcher.js";
import { buildRailSections } from "./repo_rail.js";
import { FilesIcon } from "./deck_section_icons.js";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

const HANDSET_PANEL_ID = "handset-panel";
const HANDSET_ID_PREFIX = "handset";

interface HandsetDeckProps {
  active: DeckSection;
  sourceCount: number;
  onChange: (section: DeckSection) => void;
  filePanel: (sectionSwitcher: React.ReactNode) => React.ReactNode;
  zipsContent: React.ReactNode;
  sourceContent: React.ReactNode;
}

export function HandsetDeck({
  active,
  sourceCount,
  onChange,
  filePanel,
  zipsContent,
  sourceContent,
}: HandsetDeckProps): React.JSX.Element {
  const sections: DeckSectionOption<DeckSection>[] = [
    { value: "files", label: "FILES", icon: <FilesIcon /> },
    ...buildRailSections(sourceCount),
  ];
  const sectionSwitcher = (
    <DeckSectionSwitcher
      sections={sections}
      active={active}
      onChange={onChange}
      panelId={HANDSET_PANEL_ID}
      idPrefix={HANDSET_ID_PREFIX}
      ariaLabel="Content section"
    />
  );
  const labelledBy = deckSectionTabId(HANDSET_ID_PREFIX, active);

  if (active === "files") {
    return (
      <div className="handset-deck">
        <div className="handset-chassis">
          <div
            className="handset-deck__tabpanel"
            role="tabpanel"
            id={HANDSET_PANEL_ID}
            aria-labelledby={labelledBy}
          >
            {filePanel(sectionSwitcher)}
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="handset-deck">
      <div className="handset-chassis">
        <section className="panel handset-deck__panel" data-panel="rail" data-rail={active}>
          <header className="panel-header handset-header">
            {sectionSwitcher}
            <span className="panel-cue" aria-hidden="true" />
          </header>
          <div
            key={active}
            className="panel-content"
            role="tabpanel"
            id={HANDSET_PANEL_ID}
            aria-labelledby={labelledBy}
          >
            {active === "zips" ? zipsContent : sourceContent}
          </div>
        </section>
      </div>
    </div>
  );
}
