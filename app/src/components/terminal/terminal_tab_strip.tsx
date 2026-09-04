// Path: app/src/components/terminal/terminal_tab_strip.tsx
// Description: Tablist of one repo's terminal tabs (PWSH n, x per tab, + at the end); arrows move focus, click or Enter activates

import type React from "react";
import type { TerminalTabSnapshot } from "../../lib/terminal/terminal_types.js";
import {
  NEW_TAB_TITLE,
  SESSION_CAP_TITLE,
  TAB_STRIP_LABEL,
  closeTabTitle,
} from "./terminal_copy.js";

export function terminalTabDomId(tabId: string): string {
  return `terminal-tab-${tabId}`;
}

interface TerminalTabStripProps {
  tabs: readonly TerminalTabSnapshot[];
  activeTabId: string | null;
  /** id of the host element (`role="tabpanel"`) */
  panelId: string;
  /** False at the session cap: `+` stays visible but disabled */
  canOpen: boolean;
  onActivate: (tabId: string) => void;
  onClose: (tabId: string) => void;
  onOpen: () => void;
}

function neighbourTab(
  tabs: readonly TerminalTabSnapshot[],
  index: number,
  key: string
): TerminalTabSnapshot | null {
  const count = tabs.length;
  if (count === 0) return null;
  if (key === "ArrowRight") return tabs[(index + 1) % count] ?? null;
  if (key === "ArrowLeft") return tabs[(index - 1 + count) % count] ?? null;
  if (key === "Home") return tabs[0] ?? null;
  if (key === "End") return tabs[count - 1] ?? null;
  return null;
}

export function TerminalTabStrip({
  tabs,
  activeTabId,
  panelId,
  canOpen,
  onActivate,
  onClose,
  onOpen,
}: TerminalTabStripProps): React.JSX.Element {
  return (
    <div className="terminal-strip" role="tablist" aria-label={TAB_STRIP_LABEL}>
      {tabs.map((tab, index) => {
        const isActive = tab.tabId === activeTabId;
        return (
          <div
            key={tab.tabId}
            className="terminal-strip__tab"
            data-active={isActive ? "" : undefined}
            data-status={tab.status}
          >
            <button
              type="button"
              role="tab"
              id={terminalTabDomId(tab.tabId)}
              className="terminal-strip__label"
              aria-selected={isActive}
              aria-controls={panelId}
              tabIndex={isActive ? 0 : -1}
              title={tab.title}
              onClick={() => { onActivate(tab.tabId); }}
              onKeyDown={(event) => {
                // Manual activation: arrows only move focus, because activating adopts the tab
                // and the host then focuses the terminal, which would end the arrow walk
                const next = neighbourTab(tabs, index, event.key);
                if (next === null) return;
                event.preventDefault();
                document.getElementById(terminalTabDomId(next.tabId))?.focus();
              }}
            >
              {tab.label}
            </button>
            <button
              type="button"
              className="terminal-strip__close"
              aria-label={closeTabTitle(tab.label)}
              title={closeTabTitle(tab.label)}
              tabIndex={-1}
              onClick={() => { onClose(tab.tabId); }}
            >
              ×
            </button>
          </div>
        );
      })}
      <button
        type="button"
        className="terminal-strip__add"
        aria-label={canOpen ? NEW_TAB_TITLE : SESSION_CAP_TITLE}
        title={canOpen ? NEW_TAB_TITLE : SESSION_CAP_TITLE}
        disabled={!canOpen}
        onClick={onOpen}
      >
        +
      </button>
    </div>
  );
}
