// Path: app/src/lib/source_control/status_registry_types.ts
// Description: Public contract of the source-control status registry: slot view, read phase, writer, host state

import type { AgentEvent, SourceControlStatus } from "../../shared/protocol.js";
import type { RepoRoot } from "../../shared/config.js";
import type { RefreshScheduler } from "../../hooks/source_control/source_control_refresh.js";

export type SourceControlReadPhase =
  | { kind: "waiting" }
  | { kind: "loading" }
  | { kind: "ready" }
  | { kind: "error"; code: string | null; message: string };

export interface StatusSlotView {
  /** Last known status; kept through loading and release, cleared on hard errors and root changes */
  status: SourceControlStatus | null;
  phase: SourceControlReadPhase;
}

/** The SOURCE write side: sees every read so reconciliation can end on the status that settles it */
export interface StatusWriter {
  acceptStatus(status: SourceControlStatus): void;
  /** True when the writer scheduled its own retry and the read failure must not surface */
  acceptFailure(): boolean;
}

export interface StatusHostState<C> {
  client: C | null;
  ready: boolean;
  documentHidden: boolean;
  wslOnline: boolean;
  rootKinds: ReadonlyMap<string, RepoRoot["kind"]>;
}

export interface StatusRegistryDeps<C> {
  read(client: C, repoId: string): Promise<SourceControlStatus>;
  listenFocus(onFocus: () => void): () => void;
}

export interface StatusRegistry<C> {
  retain(repoId: string): () => void;
  subscribe(repoId: string, listener: () => void): () => void;
  getSlot(repoId: string): StatusSlotView;
  routeAgentEvent(event: AgentEvent): void;
  setHostState(state: StatusHostState<C>): void;
  markRehydrated(): void;
  setRepositories(repos: readonly { repoId: string; root: RepoRoot }[]): void;
  attachWriter(repoId: string, writer: StatusWriter): () => void;
  applyStatus(repoId: string, status: SourceControlStatus): void;
  applyError(repoId: string, phase: Extract<SourceControlReadPhase, { kind: "error" }>): void;
  actionStarted(repoId: string): void;
  actionFinished(repoId: string, appliedAt: number | null): void;
  schedulerFor(repoId: string): RefreshScheduler;
  /** Manual refresh: a fresh transient-retry budget, then a forced read */
  refresh(repoId: string): void;
  readsInFlight(): number;
}
