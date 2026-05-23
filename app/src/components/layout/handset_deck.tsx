// Path: app/src/components/layout/handset_deck.tsx
// Description: Handset deck layout for Auto files and zip bundles

import type React from "react";
import { useState, useCallback } from "react";
import {
  HandsetSectionSwitcher,
  type HandsetSection,
} from "./handset_section_switcher.js";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

interface HandsetDeckProps {
  filePanel: (sectionSwitcher: React.ReactNode) => React.ReactNode;
  zipsContent: React.ReactNode;
}

export function HandsetDeck({
  filePanel,
  zipsContent,
}: HandsetDeckProps): React.JSX.Element {
  const [activeSection, setActiveSection] = useState<HandsetSection>("files");

  const handleSectionChange = useCallback((section: HandsetSection) => {
    setActiveSection(section);
  }, []);

  const sectionSwitcher = (
    <HandsetSectionSwitcher
      active={activeSection}
      onChange={handleSectionChange}
    />
  );

  if (activeSection === "files") {
    return (
      <div className="handset-deck">
        <div className="handset-chassis">{filePanel(sectionSwitcher)}</div>
      </div>
    );
  }

  return (
    <div className="handset-deck">
      <div className="handset-chassis">
        <section className="panel handset-deck__panel">
          <header className="panel-header handset-header">
            {sectionSwitcher}
            <span className="panel-cue" aria-hidden="true" />
          </header>
          <div
            key={activeSection}
            className="panel-content"
            role="tabpanel"
            id="handset-panel"
            aria-labelledby={`handset-tab-${activeSection}`}
          >
            {zipsContent}
          </div>
        </section>
      </div>
    </div>
  );
}
