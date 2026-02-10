// Path: app/src/components/layout/handset_section_switcher.tsx
// Description: Bracketed tab switcher for handset mode sections (Docs | Code | Zips)

import type React from "react";

export type HandsetSection = "docs" | "code" | "zips";

const SECTIONS: ReadonlyArray<{ value: HandsetSection; label: string }> = [
  { value: "docs", label: "DOCS" },
  { value: "code", label: "CODE" },
  { value: "zips", label: "ZIPS" },
];

interface HandsetSectionSwitcherProps {
  active: HandsetSection;
  onChange: (section: HandsetSection) => void;
}

export function HandsetSectionSwitcher({
  active,
  onChange,
}: HandsetSectionSwitcherProps): React.JSX.Element {
  return (
    <div role="tablist" className="handset-switcher" aria-label="Content section">
      {SECTIONS.map(({ value, label }, index) => (
        <button
          key={value}
          type="button"
          role="tab"
          id={`handset-tab-${value}`}
          aria-selected={active === value}
          aria-controls="handset-panel"
          tabIndex={active === value ? 0 : -1}
          data-section={value}
          className={`handset-switcher__tab${active === value ? " handset-switcher__tab--active" : ""}`}
          onClick={() => { onChange(value); }}
          onKeyDown={(event) => {
            let next: HandsetSection | null = null;

            if (event.key === "ArrowRight" || event.key === "ArrowDown") {
              const entry = SECTIONS[(index + 1) % SECTIONS.length];
              next = entry?.value ?? null;
            } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
              const entry = SECTIONS[(index - 1 + SECTIONS.length) % SECTIONS.length];
              next = entry?.value ?? null;
            } else if (event.key === "Home") {
              next = SECTIONS[0]?.value ?? null;
            } else if (event.key === "End") {
              next = SECTIONS[SECTIONS.length - 1]?.value ?? null;
            }

            if (!next) return;

            event.preventDefault();
            onChange(next);

            const container = event.currentTarget.parentElement;
            const nextButton = container?.querySelector<HTMLButtonElement>(
              `[data-section="${next}"]`
            );
            nextButton?.focus();
          }}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
