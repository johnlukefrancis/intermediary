// Path: app/src/components/layout/deck_section_switcher.tsx
// Description: Segmented icon-rocker tablist switching deck sections; the host renders the matching tabpanel

import type React from "react";
import { flushSync } from "react-dom";
import "../../styles/deck_section_switcher.css";

/** Exceptional state that outranks the ordinary count: error tone, alert mark, its own number */
export interface DeckSectionAlert {
  count: number;
  /** Appended to the tooltip and the accessible name, e.g. "46 merge conflicts" */
  title: string;
}

export interface DeckSectionOption<T extends string> {
  value: T;
  /** Accessible name and tooltip; not rendered visually */
  label: string;
  icon: React.JSX.Element;
  /** Rendered in accent after the icon; omitted at zero */
  count?: number;
  alert?: DeckSectionAlert;
}

interface DeckSectionSwitcherProps<T extends string> {
  sections: ReadonlyArray<DeckSectionOption<T>>;
  active: T;
  onChange: (section: T) => void;
  /** id of the host's `role="tabpanel"` element */
  panelId: string;
  /** Tab ids are `${idPrefix}-tab-${value}`; the host's tabpanel points at the active one */
  idPrefix: string;
  ariaLabel: string;
}

export function deckSectionTabId(idPrefix: string, value: string): string {
  return `${idPrefix}-tab-${value}`;
}

function nextSection<T extends string>(
  sections: ReadonlyArray<DeckSectionOption<T>>,
  index: number,
  key: string
): T | null {
  const count = sections.length;
  if (count === 0) return null;
  if (key === "ArrowRight" || key === "ArrowDown") {
    return sections[(index + 1) % count]?.value ?? null;
  }
  if (key === "ArrowLeft" || key === "ArrowUp") {
    return sections[(index - 1 + count) % count]?.value ?? null;
  }
  if (key === "Home") return sections[0]?.value ?? null;
  if (key === "End") return sections[count - 1]?.value ?? null;
  return null;
}

export function DeckSectionSwitcher<T extends string>({
  sections,
  active,
  onChange,
  panelId,
  idPrefix,
  ariaLabel,
}: DeckSectionSwitcherProps<T>): React.JSX.Element {
  return (
    <div role="tablist" className="deck-switcher" aria-label={ariaLabel}>
      {sections.map(({ value, label, icon, count, alert }, index) => {
        const isActive = active === value;
        const className = [
          "deck-switcher__tab",
          isActive ? "deck-switcher__tab--active" : "",
          alert !== undefined ? "deck-switcher__tab--alert" : "",
        ].filter(Boolean).join(" ");
        return (
          <button
            key={value}
            type="button"
            role="tab"
            id={deckSectionTabId(idPrefix, value)}
            aria-selected={isActive}
            aria-controls={panelId}
            tabIndex={isActive ? 0 : -1}
            data-section={value}
            title={alert === undefined ? label : `${label} · ${alert.title}`}
            className={className}
            onClick={() => { onChange(value); }}
            onKeyDown={(event) => {
              const next = nextSection(sections, index, event.key);
              if (next === null) return;
              event.preventDefault();
              // The host may remount this tablist for the new section (handset FILES vs
              // rail), so commit first and then focus the new tab by its stable id.
              flushSync(() => { onChange(next); });
              document.getElementById(deckSectionTabId(idPrefix, next))?.focus();
            }}
          >
            <span className="deck-switcher__icon" aria-hidden="true">{icon}</span>
            <span className="sr-only">{label}</span>
            {alert !== undefined ? (
              <>
                <span className="deck-switcher__alert" aria-hidden="true">!</span>
                <span className="deck-switcher__count" aria-hidden="true">{alert.count}</span>
                <span className="sr-only"> {alert.title}</span>
              </>
            ) : (
              count !== undefined && count > 0 && (
                <span className="deck-switcher__count">{count}</span>
              )
            )}
          </button>
        );
      })}
    </div>
  );
}
