// Path: app/src/lib/stream/stream_store_registry.ts
// Description: The per-repo store registry: LRU-bounded, visible store pinned, one event router for every store

import type { AgentEvent } from "../../shared/protocol.js";
import type { RepoRoot } from "../../shared/config.js";
import { STORE_MAX } from "./stream_bounds.js";
import { createStreamStore } from "./stream_store.js";
import type { StreamRepoRootKind, StreamStore, StreamTransport } from "./stream_types.js";

/** Transport facts shared by every repo; the root kind is per repo */
export type SharedTransport = Omit<StreamTransport, "repoRootKind">;

export interface StreamHostState {
  transport: SharedTransport;
  documentHidden: boolean;
  reducedMotion: boolean;
  rootKinds: ReadonlyMap<string, StreamRepoRootKind>;
}

const INITIAL_HOST_STATE: StreamHostState = {
  transport: { connected: false, helloOk: false, agentVersion: null, wslOnline: true },
  documentHidden: false,
  reducedMotion: false,
  rootKinds: new Map(),
};

export interface StreamStoreRegistry {
  getOrCreate(repoId: string, rootIdentity: string): StreamStore;
  get(repoId: string): StreamStore | undefined;
  /** Routing never creates stores or delivers another repository's activity. */
  routeAgentEvent(event: AgentEvent): void;
  setVisibleRepo(repoId: string | null): void;
  setHostState(state: StreamHostState): void;
  markRehydrated(): void;
  setRepositories(repos: readonly { repoId: string; root: RepoRoot }[]): void;
  disposeAll(): void;
}

export function streamRootIdentity(repoId: string, root: RepoRoot | undefined): string {
  return JSON.stringify([repoId, root?.kind, root?.path]);
}

export function createStreamStoreRegistry(makeStore: (repoId: string) => StreamStore = createStreamStore): StreamStoreRegistry {
  /** Insertion order is LRU order: the most recently viewed store is last */
  const stores = new Map<string, { rootIdentity: string; store: StreamStore }>();
  let visibleRepoId: string | null = null;
  let host = INITIAL_HOST_STATE;

  function push(store: StreamStore): void {
    store.setTransport({ ...host.transport, repoRootKind: host.rootKinds.get(store.repoId) ?? "host" });
    store.setDocumentHidden(host.documentHidden);
    store.setReducedMotion(host.reducedMotion);
  }

  function touch(repoId: string): void {
    const store = stores.get(repoId);
    if (store === undefined) return;
    stores.delete(repoId);
    stores.set(repoId, store);
  }

  function evict(): void {
    for (const [repoId, entry] of stores) {
      if (stores.size <= STORE_MAX) return;
      if (repoId === visibleRepoId) continue;
      entry.store.dispose();
      stores.delete(repoId);
    }
  }

  return {
    getOrCreate(repoId, rootIdentity) {
      const existing = stores.get(repoId);
      if (existing?.rootIdentity === rootIdentity) {
        touch(repoId);
        return existing.store;
      }
      existing?.store.dispose();
      stores.delete(repoId);
      const store = makeStore(repoId);
      stores.set(repoId, { rootIdentity, store });
      push(store);
      evict();
      return store;
    },
    get: (repoId) => stores.get(repoId)?.store,
    routeAgentEvent(event) {
      if (
        event.type === "fileDelta" || event.type === "fileDeltaCounters" ||
        event.type === "fileChanged" || event.type === "snapshot"
      ) {
        stores.get(event.repoId)?.store.intake(event);
      }
    },
    setVisibleRepo(repoId) {
      visibleRepoId = repoId;
      if (repoId !== null) touch(repoId);
    },
    setHostState(next) {
      host = next;
      for (const entry of stores.values()) push(entry.store);
    },
    markRehydrated() {
      for (const entry of stores.values()) entry.store.markRehydrated();
    },
    setRepositories(repos) {
      const current = new Map(repos.map((repo) => [repo.repoId, streamRootIdentity(repo.repoId, repo.root)] as const));
      for (const [repoId, entry] of stores) {
        if (entry.rootIdentity === current.get(repoId)) continue;
        entry.store.dispose();
        stores.delete(repoId);
      }
    },
    disposeAll() {
      for (const entry of stores.values()) entry.store.dispose();
      stores.clear();
    },
  };
}

/** The one registry the host hook and the per-repo binding share */
export const streamStoreRegistry: StreamStoreRegistry = createStreamStoreRegistry();
