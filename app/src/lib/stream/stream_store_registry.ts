// Path: app/src/lib/stream/stream_store_registry.ts
// Description: The per-repo store registry: LRU-bounded, visible store pinned, one event router for every store

import type { AgentEvent } from "../../shared/protocol.js";
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
  getOrCreate(repoId: string): StreamStore;
  get(repoId: string): StreamStore | undefined;
  /**
   * fileDelta / fileDeltaCounters / fileChanged / snapshot route by the event's own repoId to the store
   * that exists for it; nothing is created and no store ever receives another repo's event (a snapshot
   * seeds history rows, so a cross-repo route would print one repo's files in another's feed)
   */
  routeAgentEvent(event: AgentEvent): void;
  setVisibleRepo(repoId: string | null): void;
  setHostState(state: StreamHostState): void;
  markRehydrated(): void;
  disposeRepo(repoId: string): void;
  disposeAll(): void;
}

export function createStreamStoreRegistry(makeStore: (repoId: string) => StreamStore = createStreamStore): StreamStoreRegistry {
  /** Insertion order is LRU order: the most recently viewed store is last */
  const stores = new Map<string, StreamStore>();
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
    for (const [repoId, store] of stores) {
      if (stores.size <= STORE_MAX) return;
      if (repoId === visibleRepoId) continue;
      store.dispose();
      stores.delete(repoId);
    }
  }

  return {
    getOrCreate(repoId) {
      const existing = stores.get(repoId);
      if (existing !== undefined) {
        touch(repoId);
        return existing;
      }
      const store = makeStore(repoId);
      stores.set(repoId, store);
      push(store);
      evict();
      return store;
    },
    get: (repoId) => stores.get(repoId),
    routeAgentEvent(event) {
      if (
        event.type === "fileDelta" || event.type === "fileDeltaCounters" ||
        event.type === "fileChanged" || event.type === "snapshot"
      ) {
        stores.get(event.repoId)?.intake(event);
      }
    },
    setVisibleRepo(repoId) {
      visibleRepoId = repoId;
      if (repoId !== null) touch(repoId);
    },
    setHostState(next) {
      host = next;
      for (const store of stores.values()) push(store);
    },
    markRehydrated() {
      for (const store of stores.values()) store.markRehydrated();
    },
    disposeRepo(repoId) {
      stores.get(repoId)?.dispose();
      stores.delete(repoId);
    },
    disposeAll() {
      for (const store of stores.values()) store.dispose();
      stores.clear();
    },
  };
}

/** The one registry the host hook and the per-repo binding share */
export const streamStoreRegistry: StreamStoreRegistry = createStreamStoreRegistry();
