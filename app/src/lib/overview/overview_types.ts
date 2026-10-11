// Path: app/src/lib/overview/overview_types.ts
// Description: Bounded repository activity projections for the all-repository Overview

import type { RepoRoot } from "../../shared/config.js";
import type { AgentEvent, DeltaBaseline, DeltaOp, FileChangeType, FileKind } from "../../shared/protocol.js";

export const OVERVIEW_RECENT_MAX = 6;
export const OVERVIEW_EDITS_MAX = 12;
export const OVERVIEW_ACTIVE_MS = 30_000;
export const OVERVIEW_WORKING_MS = 15 * 60_000;

export type OverviewAvailability = "connecting" | "watching" | "offline" | "held" | "unwatched" | "error" | "update-required";

export interface OverviewSignal {
  path: string;
  kind: FileKind;
  changeType: FileChangeType;
  at: number;
  source: "live" | "history";
  count: number;
}

export interface OverviewEdit {
  id: number;
  path: string;
  fromPath: string | null;
  kind: FileKind;
  op: DeltaOp;
  at: number;
  lines: readonly { kind: "add" | "del" | "context"; text: string }[];
  image: { bytes: number; mimeType: string | null; mtimeMs: number } | null;
  added: number | null;
  removed: number | null;
  baseline: DeltaBaseline | null;
  detail: string;
  truncated: boolean;
}

/** Edits the agent withheld or dropped, or a sequence gap, disclosed where they happened */
export interface OverviewLoss {
  id: number;
  at: number;
  count: number;
}

export interface OverviewRepoSnapshot {
  repoId: string;
  root: RepoRoot;
  availability: OverviewAvailability;
  issue: string | null;
  recent: readonly OverviewSignal[];
  edits: readonly OverviewEdit[];
  lastChangedAt: number | null;
  lastWorkedAt: number | null;
  losses: readonly OverviewLoss[];
}

export interface OverviewSnapshot {
  repos: ReadonlyMap<string, OverviewRepoSnapshot>;
  revision: number;
}

export interface OverviewTransport {
  connected: boolean;
  helloOk: boolean;
  agentVersion: string | null;
  watchedRepoIds: readonly string[];
  wslOnline: boolean;
  error: string | null;
}

export interface OverviewStore {
  subscribe(listener: () => void): () => void;
  getSnapshot(): OverviewSnapshot;
  setRepositories(repos: readonly { repoId: string; root: RepoRoot }[]): void;
  setTransport(transport: OverviewTransport): void;
  intake(event: AgentEvent, now?: number): void;
  setRepoIssue(repoId: string, root: RepoRoot, issue: string, watchUnavailable?: boolean): void;
  markRehydrated(): void;
  flush(): void;
  dispose(): void;
}
