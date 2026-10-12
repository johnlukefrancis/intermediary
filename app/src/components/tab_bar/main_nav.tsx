// Path: app/src/components/tab_bar/main_nav.tsx
// Description: Icon keycap cluster for app-level destinations (Overview, Notes) ahead of the repo tabs

import type React from "react";

interface MainNavProps {
  overviewActive: boolean;
  notesOpen: boolean;
  onOverview: () => void;
  onToggleNotes: () => void;
}

export function MainNav({ overviewActive, notesOpen, onOverview, onToggleNotes }: MainNavProps): React.JSX.Element {
  // Notes overlays the current view, so it owns the lit state while open.
  const overviewLit = overviewActive && !notesOpen;
  return (
    <div className="main-nav" role="group" aria-label="Main navigation">
      <div className="main-nav__housing">
        <button type="button" className={`main-nav__key${overviewLit ? " main-nav__key--lit" : ""}`}
          onClick={onOverview} aria-current={overviewActive ? "page" : undefined}
          aria-label="Overview" title="Overview — activity across all repositories">
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <rect x="3" y="3" width="5.5" height="5.5" rx="1" />
            <rect x="11.5" y="3" width="5.5" height="5.5" rx="1" />
            <rect x="3" y="11.5" width="5.5" height="5.5" rx="1" />
            <rect x="11.5" y="11.5" width="5.5" height="5.5" rx="1" />
          </svg>
        </button>
        <button type="button" className={`main-nav__key${notesOpen ? " main-nav__key--lit" : ""}`}
          onClick={onToggleNotes} aria-pressed={notesOpen}
          aria-label="Notes" title="Notes — global scratchpad (Esc closes)">
          <svg viewBox="0 0 20 20" aria-hidden="true">
            <path d="M4.5 2.5h8L16 6v11.5H4.5z" />
            <path d="M12.5 2.5V6H16" />
            <path d="M7.5 10h5M7.5 13h3" />
          </svg>
        </button>
      </div>
      <span className="main-nav__divider" aria-hidden="true" />
    </div>
  );
}
