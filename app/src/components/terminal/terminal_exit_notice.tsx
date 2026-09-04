// Path: app/src/components/terminal/terminal_exit_notice.tsx
// Description: Console-prompt notice floating over a tab that is starting, has exited, or failed to start

import type React from "react";
import type { TerminalTabSnapshot } from "../../lib/terminal/terminal_types.js";
import {
  CLOSE_LABEL,
  PWSH_FAILED_TO_START,
  RESTART_LABEL,
  RETRY_LABEL,
  STARTING_PWSH,
  processExitedHeading,
} from "./terminal_copy.js";

interface TerminalExitNoticeProps {
  tab: TerminalTabSnapshot;
  /** Fresh pty into the same xterm (scrollback kept) */
  onRestart: () => void;
  onClose: () => void;
}

/** Nothing while the shell runs; the notice sits above the scrollback otherwise */
export function TerminalExitNotice({
  tab,
  onRestart,
  onClose,
}: TerminalExitNoticeProps): React.JSX.Element | null {
  if (tab.status === "running") return null;

  if (tab.status === "starting") {
    return (
      <div className="terminal-column__notice" role="status">
        <p className="empty-state empty-state--waiting">{STARTING_PWSH}</p>
      </div>
    );
  }

  const failed = tab.status === "failed";
  return (
    <div className="terminal-column__notice" role={failed ? "alert" : "status"}>
      <div className="terminal-column__card" data-tone={failed ? "error" : undefined}>
        <p className="empty-state">
          {failed ? PWSH_FAILED_TO_START : processExitedHeading(tab.exitCode)}
        </p>
        {failed && tab.error !== null && tab.error.length > 0 && (
          <div className="build-error terminal-column__message">{tab.error}</div>
        )}
        <div className="terminal-column__actions">
          <button type="button" className="dir-action-btn" onClick={onRestart}>
            {failed ? RETRY_LABEL : RESTART_LABEL}
          </button>
          <button type="button" className="dir-action-btn" onClick={onClose}>
            {CLOSE_LABEL}
          </button>
        </div>
      </div>
    </div>
  );
}
