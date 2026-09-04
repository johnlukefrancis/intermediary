// Path: app/src/lib/terminal/terminal_registry.ts
// Description: Module-level owner of every terminal session grouped per repo: immutable snapshots for React, open/close/restart, adopt/park, lifecycle sweeps

import { repoRootKey, type RepoRoot } from "../../shared/config.js";
import { isForegroundWindow } from "../window/foreground.js";
import { TerminalSession } from "./terminal_session.js";
import {
  EMPTY_TERMINAL_GROUP,
  MAX_TERMINAL_SESSIONS,
  type TerminalGroupSnapshot,
  type TerminalRegistryApi,
} from "./terminal_types.js";

interface TerminalGroup {
  readonly repoId: string;
  readonly rootKey: string;
  readonly tabs: TerminalSession[];
  activeTabId: string | null;
  autoOpened: boolean;
  /** Cached until the next change so `useSyncExternalStore` sees a stable reference */
  snapshot: TerminalGroupSnapshot | null;
}

const groups = new Map<string, TerminalGroup>();
const listeners = new Set<() => void>();
/** The one session whose element sits in a visible host */
let adopted: TerminalSession | null = null;
let foreground = isForegroundWindow();

function notify(): void {
  for (const listener of listeners) listener();
}

function invalidate(group: TerminalGroup): void {
  group.snapshot = null;
  notify();
}

function disposeGroup(group: TerminalGroup): void {
  for (const tab of group.tabs) disposeSession(tab);
}

function groupFor(repoId: string, repoRoot: RepoRoot): TerminalGroup {
  let group = groups.get(repoId);
  const rootKey = repoRootKey(repoRoot);
  if (group !== undefined && group.rootKey !== rootKey) {
    // A reused id is not enough to preserve a terminal: its process is bound to
    // the old authority and must never be restarted into a stale root.
    disposeGroup(group);
    groups.delete(repoId);
    notify();
    group = undefined;
  }
  if (group === undefined) {
    group = {
      repoId,
      rootKey,
      tabs: [],
      activeTabId: null,
      autoOpened: false,
      snapshot: null,
    };
    groups.set(repoId, group);
  }
  return group;
}

function findSession(repoId: string, tabId: string): TerminalSession | null {
  return groups.get(repoId)?.tabs.find((tab) => tab.tabId === tabId) ?? null;
}

function findSessionByTab(tabId: string): TerminalSession | null {
  for (const group of groups.values()) {
    const session = group.tabs.find((tab) => tab.tabId === tabId);
    if (session !== undefined) return session;
  }
  return null;
}

/** Every retained xterm/pty owner consumes a product slot, including exited or failed tabs. */
function sessionCount(): number {
  let count = 0;
  for (const group of groups.values()) {
    count += group.tabs.length;
  }
  return count;
}

function buildSnapshot(group: TerminalGroup): TerminalGroupSnapshot {
  return Object.freeze({
    tabs: Object.freeze(group.tabs.map((tab) => tab.getSnapshot())),
    activeTabId: group.activeTabId,
    autoOpened: group.autoOpened,
  });
}

/** Lowest free 1-based number in the group, stable for the tab's life */
function nextOrdinal(group: TerminalGroup): number {
  const used = new Set(group.tabs.map((tab) => tab.ordinal));
  let ordinal = 1;
  while (used.has(ordinal)) ordinal += 1;
  return ordinal;
}

function onSessionChange(session: TerminalSession): void {
  const group = groups.get(session.repoId);
  if (group !== undefined) invalidate(group);
}

function disposeSession(session: TerminalSession): void {
  if (adopted === session) adopted = null;
  session.dispose();
}

function openTabIn(group: TerminalGroup, repoRoot: RepoRoot): string | null {
  if (sessionCount() >= MAX_TERMINAL_SESSIONS) return null;
  const session = new TerminalSession({
    repoId: group.repoId,
    ordinal: nextOrdinal(group),
    repoRoot,
    foreground,
    onChange: onSessionChange,
  });
  group.tabs.push(session);
  group.activeTabId = session.tabId;
  invalidate(group);
  return session.tabId;
}

export const terminalRegistry: TerminalRegistryApi = {
  subscribe(listener) {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  },

  getGroupSnapshot(repoId) {
    const group = groups.get(repoId);
    if (group === undefined) return EMPTY_TERMINAL_GROUP;
    group.snapshot ??= buildSnapshot(group);
    return group.snapshot;
  },

  getSessionCount() {
    return sessionCount();
  },

  ensureFirstTab(repoId, repoRoot) {
    const group = groupFor(repoId, repoRoot);
    if (group.autoOpened) return;
    // Set before opening so a StrictMode double effect cannot open twice
    group.autoOpened = true;
    // At the session cap nothing opened, so a later visit may still auto-open
    if (openTabIn(group, repoRoot) === null) group.autoOpened = false;
  },

  openTab(repoId, repoRoot) {
    return openTabIn(groupFor(repoId, repoRoot), repoRoot);
  },

  activateTab(repoId, tabId) {
    const group = groups.get(repoId);
    if (group === undefined || group.activeTabId === tabId) return;
    if (!group.tabs.some((tab) => tab.tabId === tabId)) return;
    group.activeTabId = tabId;
    invalidate(group);
  },

  closeTab(repoId, tabId) {
    const group = groups.get(repoId);
    if (group === undefined) return;
    const index = group.tabs.findIndex((tab) => tab.tabId === tabId);
    if (index === -1) return;
    const [session] = group.tabs.splice(index, 1);
    if (session !== undefined) disposeSession(session);
    if (group.activeTabId === tabId) {
      const neighbour = group.tabs[index] ?? group.tabs[index - 1] ?? null;
      group.activeTabId = neighbour?.tabId ?? null;
    }
    invalidate(group);
  },

  restartTab(repoId, tabId, repoRoot) {
    // Enforce root authority at the action boundary as well as in the passive
    // lifecycle sweep. A restart callback can run before that effect.
    groupFor(repoId, repoRoot).tabs.find((tab) => tab.tabId === tabId)?.restart();
  },

  adopt(repoId, tabId, host) {
    const session = findSession(repoId, tabId);
    if (session === null) return;
    if (adopted !== null && adopted !== session) adopted.park();
    adopted = session;
    session.adopt(host);
  },

  park(tabId) {
    const session = findSessionByTab(tabId);
    if (session === null) return;
    if (adopted === session) adopted = null;
    session.park();
  },

  fitTab(tabId) {
    findSessionByTab(tabId)?.fit();
  },

  focusTab(tabId) {
    findSessionByTab(tabId)?.focus();
  },

  applyTheme() {
    adopted?.applyTheme();
  },

  setForeground(next) {
    if (foreground === next) return;
    foreground = next;
    for (const group of groups.values()) {
      for (const tab of group.tabs) tab.setCursorBlink(next);
    }
  },

  retainRepos(repoRoots) {
    let changed = false;
    for (const [repoId, group] of groups) {
      const repoRoot = repoRoots.get(repoId);
      if (repoRoot !== undefined && repoRootKey(repoRoot) === group.rootKey) continue;
      disposeGroup(group);
      groups.delete(repoId);
      changed = true;
    }
    if (changed) notify();
  },

  closeAll() {
    for (const group of groups.values()) {
      disposeGroup(group);
    }
    groups.clear();
    notify();
  },
};
