// Path: app/src/components/layout/deck_section_icons.tsx
// Description: Inline 24x24 stroke glyphs for the deck section switcher (stroke supplied by CSS)

import type React from "react";

export function ZipsIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M3 4h18v4.5H3z" />
      <path d="M5 8.5V20h14V8.5" />
      <path d="M10 12.5h4" />
    </svg>
  );
}

export function SourceIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="6" cy="18" r="2.5" />
      <circle cx="18" cy="6" r="2.5" />
      <path d="M6 3v12.5" />
      <path d="M18 8.5a9.5 9.5 0 0 1-9.5 9.5" />
    </svg>
  );
}

export function FilesIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M8 3h6l4 4v11H8z" />
      <path d="M14 3v4h4" />
      <path d="M5 6v13h9" />
    </svg>
  );
}

/** Prompt chevron and cursor bar: the bare `>_` reads cleanly at 15 px beside the box and branch */
export function TerminalIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M5 6l7 6-7 6" />
      <path d="M13 18h6" />
    </svg>
  );
}
