// Path: app/src/components/terminal/terminal_column.tsx
// Description: TERMINAL rail body for one repo: tab strip over the imperative xterm host, plus the starting/exited/failed/empty notices

import type React from "react";
import { useCallback, useEffect, useSyncExternalStore } from "react";
import { useConfig } from "../../hooks/use_config.js";
import { useTerminalGroup } from "../../hooks/terminal/use_terminal_group.js";
import { useTerminalHost } from "../../hooks/terminal/use_terminal_host.js";
import { terminalRegistry } from "../../lib/terminal/terminal_registry.js";
import { MAX_TERMINAL_SESSIONS } from "../../lib/terminal/terminal_types.js";
import { DEFAULT_ACCENT_HEX } from "../../lib/theme/accent_utils.js";
import type { RepoRoot } from "../../shared/config.js";
import { NEW_TAB_LABEL, NO_TERMINAL } from "./terminal_copy.js";
import { TerminalExitNotice } from "./terminal_exit_notice.js";
import { TerminalTabStrip, terminalTabDomId } from "./terminal_tab_strip.js";
import "../../styles/terminal_column.css";

interface TerminalColumnProps {
  repoId: string;
  /** Undefined only while the repo is not (yet) in the config: nothing can open then */
  repoRoot: RepoRoot | undefined;
}

function subscribeRegistry(listener: () => void): () => void {
  return terminalRegistry.subscribe(listener);
}

function readSessionCount(): number {
  return terminalRegistry.getSessionCount();
}

export function TerminalColumn({ repoId, repoRoot }: TerminalColumnProps): React.JSX.Element {
  const { config } = useConfig();
  const group = useTerminalGroup(repoId);
  const sessionCount = useSyncExternalStore(subscribeRegistry, readSessionCount, readSessionCount);
  const repo = config.repos.find((entry) => entry.repoId === repoId);
  const accentHex = config.tabThemes[repo?.groupId ?? repoId]?.accentHex ?? DEFAULT_ACCENT_HEX;
  const themeKey = `${config.themeMode}:${accentHex}:${String(config.windowOpacityPercent)}`;
  const hostRef = useTerminalHost(repoId, group.activeTabId, themeKey);

  // First visit opens PWSH 1 once per repo; the registry makes the double effect a no-op
  useEffect(() => {
    if (repoRoot !== undefined) terminalRegistry.ensureFirstTab(repoId, repoRoot);
  }, [repoId, repoRoot]);

  const canOpen = repoRoot !== undefined && sessionCount < MAX_TERMINAL_SESSIONS;
  const activeTab = group.tabs.find((tab) => tab.tabId === group.activeTabId) ?? null;
  const panelId = `terminal-panel-${repoId}`;

  const openTab = useCallback(() => {
    if (repoRoot !== undefined) terminalRegistry.openTab(repoId, repoRoot);
  }, [repoId, repoRoot]);
  const activateTab = useCallback(
    (tabId: string) => { terminalRegistry.activateTab(repoId, tabId); },
    [repoId]
  );
  const closeTab = useCallback(
    (tabId: string) => { terminalRegistry.closeTab(repoId, tabId); },
    [repoId]
  );

  return (
    <div className="terminal-column" data-status={activeTab?.status}>
      <TerminalTabStrip
        tabs={group.tabs}
        activeTabId={group.activeTabId}
        panelId={panelId}
        canOpen={canOpen}
        onActivate={activateTab}
        onClose={closeTab}
        onOpen={openTab}
      />
      <div
        className="terminal-column__body"
        onContextMenu={(event) => { event.preventDefault(); }}
      >
        {/* React never renders children here: the registry moves the session element in and out */}
        <div
          ref={hostRef}
          id={panelId}
          className="terminal-column__host"
          data-terminal-host=""
          role="tabpanel"
          aria-labelledby={activeTab === null ? undefined : terminalTabDomId(activeTab.tabId)}
        />
        {activeTab === null ? (
          <div className="terminal-column__notice">
            <div className="terminal-column__card">
              <p className="empty-state">{NO_TERMINAL}</p>
              <div className="terminal-column__actions">
                <button
                  type="button"
                  className="dir-action-btn"
                  disabled={!canOpen}
                  onClick={openTab}
                >
                  {NEW_TAB_LABEL}
                </button>
              </div>
            </div>
          </div>
        ) : (
          <TerminalExitNotice
            tab={activeTab}
            onRestart={() => {
              if (repoRoot !== undefined) {
                terminalRegistry.restartTab(repoId, activeTab.tabId, repoRoot);
              }
            }}
            onClose={() => { closeTab(activeTab.tabId); }}
          />
        )}
      </div>
    </div>
  );
}
