// Path: app/src/components/source_control/source_control_icons.tsx
// Description: Inline 24x24 stroke glyphs for source-control controls (stroke supplied by CSS)

import type React from "react";

export function RefreshIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M20 12a8 8 0 1 1-2.34-5.66" />
      <path d="M20 4v5h-5" />
    </svg>
  );
}

export function PullIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 4v13" />
      <path d="M6 11l6 6 6-6" />
      <path d="M5 20h14" />
    </svg>
  );
}

export function PushIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 20V7" />
      <path d="M6 13l6-6 6 6" />
      <path d="M5 4h14" />
    </svg>
  );
}

export function PlusIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

export function MinusIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M5 12h14" />
    </svg>
  );
}

export function DiscardIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 10h11a5 5 0 0 1 0 10h-5" />
      <path d="M8 6l-4 4 4 4" />
    </svg>
  );
}

export function ChevronIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M9 6l6 6-6 6" />
    </svg>
  );
}
