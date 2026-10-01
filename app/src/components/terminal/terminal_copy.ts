// Path: app/src/components/terminal/terminal_copy.ts
// Description: Console-prompt copy, button labels, and tooltips for the terminal column

import { MAX_TERMINAL_SESSIONS, TERMINAL_SHELL_LABEL } from "../../lib/terminal/terminal_types.js";

export const STARTING_SHELL = `STARTING ${TERMINAL_SHELL_LABEL}`;
export const NO_TERMINAL = "NO TERMINAL";
export const SHELL_FAILED_TO_START = `${TERMINAL_SHELL_LABEL} FAILED TO START`;

export const NEW_TAB_LABEL = "+ New";
export const RESTART_LABEL = "Restart";
export const RETRY_LABEL = "Retry";
export const CLOSE_LABEL = "Close";

export const TAB_STRIP_LABEL = "Terminal tabs";
export const NEW_TAB_TITLE = "New terminal";
export const SESSION_CAP_TITLE = `Session cap reached (${MAX_TERMINAL_SESSIONS})`;

/** `PROCESS EXITED · CODE 1`; a null code (killed, or the console went away) drops the suffix */
export function processExitedHeading(code: number | null): string {
  return code === null ? "PROCESS EXITED" : `PROCESS EXITED · CODE ${code}`;
}

export function closeTabTitle(label: string): string {
  return `Close ${label}`;
}
