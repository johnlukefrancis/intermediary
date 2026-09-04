// Path: app/src/lib/terminal/terminal_types.ts
// Description: Frontend terminal session model: tab/group snapshots and the registry API the rail consumes

import type { RepoRoot } from "../../shared/config.js";
import type { TerminalCloseReason } from "./terminal_ipc.js";

/** Sessions the app will hold at once; mirrors the backend cap */
export const MAX_TERMINAL_SESSIONS = 12;

export type TerminalTabStatus = "starting" | "running" | "exited" | "failed";

/** Immutable view of one tab; a new object is produced on every change */
export interface TerminalTabSnapshot {
  readonly tabId: string;
  /** 1-based number within its repo group, stable for the tab's life (`PWSH 2`) */
  readonly ordinal: number;
  /** `PWSH n` */
  readonly label: string;
  /** Latest OSC title set by the shell, else the label */
  readonly title: string;
  readonly status: TerminalTabStatus;
  readonly exitCode: number | null;
  readonly exitReason: TerminalCloseReason | null;
  /** Open failure message (`status === "failed"`) */
  readonly error: string | null;
}

/** Immutable view of one repo's terminal group */
export interface TerminalGroupSnapshot {
  readonly tabs: readonly TerminalTabSnapshot[];
  readonly activeTabId: string | null;
  /** The first-visit auto-open already happened (never repeats after the user closes every tab) */
  readonly autoOpened: boolean;
}

export const EMPTY_TERMINAL_GROUP: TerminalGroupSnapshot = Object.freeze({
  tabs: [],
  activeTabId: null,
  autoOpened: false,
});

/**
 * The module-level registry: owns every xterm instance and its DOM element outside React.
 * React components only read snapshots and adopt/park the active element.
 */
export interface TerminalRegistryApi {
  subscribe(listener: () => void): () => void;
  /** Stable reference between changes (safe for `useSyncExternalStore`) */
  getGroupSnapshot(repoId: string): TerminalGroupSnapshot;
  getSessionCount(): number;
  /** First-visit auto-open: opens one tab once per repo; idempotent under StrictMode double effects */
  ensureFirstTab(repoId: string, repoRoot: RepoRoot): void;
  /** Opens a new tab in the repo root; returns null when the session cap is reached */
  openTab(repoId: string, repoRoot: RepoRoot): string | null;
  activateTab(repoId: string, tabId: string): void;
  /** Closes the pty (if live), disposes the xterm, removes the tab; the neighbour becomes active */
  closeTab(repoId: string, tabId: string): void;
  /** Opens a fresh pty only when the retained tab still belongs to this exact repo root */
  restartTab(repoId: string, tabId: string, repoRoot: RepoRoot): void;
  /** Moves the tab's element into `host`, attaches the renderer, fits, applies the deck theme */
  adopt(repoId: string, tabId: string, host: HTMLElement): void;
  /** Moves the tab's element back to the off-screen parking host; its renderer remains owned */
  park(tabId: string): void;
  fitTab(tabId: string): void;
  focusTab(tabId: string): void;
  /** Re-reads the deck tokens (accent or theme mode changed) for the adopted session */
  applyTheme(): void;
  /** Motion governor: cursor blink only while the window is foreground */
  setForeground(foreground: boolean): void;
  /** Closes groups that are removed or whose configured root identity changed */
  retainRepos(repoRoots: ReadonlyMap<string, RepoRoot>): void;
  /** Page unload: closes every session */
  closeAll(): void;
}
