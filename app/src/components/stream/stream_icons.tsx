// Path: app/src/components/stream/stream_icons.tsx
// Description: Stroke glyphs owned by the Stream panel, drawn in the Auto files header idiom

import type React from "react";

/**
 * Three flowing rules of decreasing length plus a fast-forward caret. Stroke width, caps,
 * and colour come from `.auto-files-icon-button svg` in auto_files_controls.css, exactly
 * like the sibling rocker glyphs, so this file carries geometry only.
 */
export function StreamIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 7h9M4 12h13M4 17h7" />
      <path d="M17 5.5l3.5 3.5-3.5 3.5" />
    </svg>
  );
}

/** Downward caret for the follow pill: same stroke style, the sheet sizes it */
export function CaretDownIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M6 9l6 6 6-6" />
    </svg>
  );
}
