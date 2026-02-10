// Path: app/src/components/layout/handset_deck.tsx
// Description: Single-panel vertical deck layout for handset mode with chassis framing

import type React from "react";
import { useState, useCallback } from "react";
import {
  HandsetSectionSwitcher,
  type HandsetSection,
} from "./handset_section_switcher.js";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

interface HandsetDeckProps {
  /** Star toggle for docs pane */
  docsHeaderRight: React.ReactNode;
  /** Star toggle for code pane */
  codeHeaderRight: React.ReactNode;
  /** FileListColumn for docs */
  docsContent: React.ReactNode;
  /** FileListColumn for code */
  codeContent: React.ReactNode;
  /** BundleColumn */
  zipsContent: React.ReactNode;
}

export function HandsetDeck({
  docsHeaderRight,
  codeHeaderRight,
  docsContent,
  codeContent,
  zipsContent,
}: HandsetDeckProps): React.JSX.Element {
  const [activeSection, setActiveSection] = useState<HandsetSection>("docs");

  const handleSectionChange = useCallback((section: HandsetSection) => {
    setActiveSection(section);
  }, []);

  // Resolve section-specific header control
  let headerRight: React.ReactNode;
  if (activeSection === "docs") {
    headerRight = docsHeaderRight;
  } else if (activeSection === "code") {
    headerRight = codeHeaderRight;
  } else {
    headerRight = <span className="panel-cue" aria-hidden="true" />;
  }

  // Resolve active content
  let content: React.ReactNode;
  if (activeSection === "docs") {
    content = docsContent;
  } else if (activeSection === "code") {
    content = codeContent;
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
