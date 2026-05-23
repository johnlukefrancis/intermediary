// Path: app/src/components/layout/handset_deck.tsx
// Description: Single-panel vertical deck layout for file feeds and zip bundles

import type React from "react";
import { useState, useCallback } from "react";
import {
  HandsetSectionSwitcher,
  type HandsetSection,
} from "./handset_section_switcher.js";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

interface HandsetDeckProps {
  latestHeader: React.ReactNode;
  activeHeader: React.ReactNode;
  latestContent: React.ReactNode;
  activeContent: React.ReactNode;
  zipsContent: React.ReactNode;
}

export function HandsetDeck({
  latestHeader,
  activeHeader,
  latestContent,
  activeContent,
  zipsContent,
}: HandsetDeckProps): React.JSX.Element {
  const [activeSection, setActiveSection] = useState<HandsetSection>("latest");

  const handleSectionChange = useCallback((section: HandsetSection) => {
    setActiveSection(section);
  }, []);

  let headerRight: React.ReactNode;
  if (activeSection === "latest") {
    headerRight = latestHeader;
  } else if (activeSection === "active") {
    headerRight = activeHeader;
  } else {
    headerRight = <span className="panel-cue" aria-hidden="true" />;
  }

  let content: React.ReactNode;
  if (activeSection === "latest") {
    content = latestContent;
  } else if (activeSection === "active") {
    content = activeContent;
  } else {
    content = zipsContent;
  }

  return (
    <div className="handset-deck">
      <div className="handset-chassis">
        <section className="panel handset-deck__panel">
          <header className="panel-header handset-header">
            <HandsetSectionSwitcher
              active={activeSection}
              onChange={handleSectionChange}
            />
            {headerRight}
          </header>
          <div
            key={activeSection}
            className="panel-content"
            role="tabpanel"
            id="handset-panel"
            aria-labelledby={`handset-tab-${activeSection}`}
          >
            {content}
          </div>
        </section>
      </div>
    </div>
  );
}
