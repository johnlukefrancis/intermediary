// Path: app/src/lib/source_control/status_registry.ts
// Description: The one app-lifetime source-control status read owner: per-repo demand, LRU-kept statuses, capped reads, writer attach

import type { AgentClient } from "../agent/agent_client.js";
import { sendSourceControlStatus } from "../agent/messages_source_control.js";
import { streamRootIdentity } from "../stream/stream_store_registry.js";
import { createStatusReadQueue } from "./status_read_queue.js";
import { createStatusSlot, EMPTY_VIEW, resetSlotRoot, setSlotPhase, type SlotContext, type StatusSlot } from "./status_slot.js";
import type { StatusHostState, StatusRegistry, StatusRegistryDeps } from "./status_registry_types.js";

const MAX_READS_IN_FLIGHT = 2;
const KEPT_SLOTS = 8;

export function createStatusRegistry<C>(deps: StatusRegistryDeps<C>): StatusRegistry<C> {
  /** Insertion order is LRU order: the most recently retained slot is last */
  const slots = new Map<string, StatusSlot>();
  const listeners = new Map<string, Set<() => void>>();
  const identities = new Map<string, string>();
  const queue = createStatusReadQueue(MAX_READS_IN_FLIGHT);
  let host: StatusHostState<C> = { client: null, ready: false, documentHidden: false, wslOnline: true, rootKinds: new Map() };
  let totalDemand = 0;
  let stopFocus: (() => void) | null = null;

  const context: SlotContext<C> = {
    host: () => host,
    isCurrent: (slot) => slots.get(slot.repoId) === slot,
    read: (client, repoId) => deps.read(client, repoId),
    setView(slot, view) {
      slot.view = view;
      for (const listener of listeners.get(slot.repoId) ?? []) listener();
    },
  };

  function identityOf(repoId: string): string {
    return identities.get(repoId) ?? streamRootIdentity(repoId, undefined);
  }

  function ensure(repoId: string): StatusSlot {
    const existing = slots.get(repoId);
    if (existing !== undefined) return existing;
    const slot = createStatusSlot(context, queue, repoId, identityOf(repoId));
    slots.set(repoId, slot);
    return slot;
  }

  function demanded(): StatusSlot[] {
    return [...slots.values()].filter((slot) => slot.demand > 0);
  }

  function evict(): void {
    for (const [repoId, slot] of slots) {
      if (slots.size <= KEPT_SLOTS) return;
      if (slot.demand > 0) continue;
      slot.scheduler.dispose();
      slots.delete(repoId);
    }
  }

  function forceRead(slot: StatusSlot): void {
    slot.generation += 1;
    slot.scheduler.requestRefresh();
  }

  function onFocus(): void {
    for (const slot of demanded()) if (!slot.actionPending) slot.scheduler.requestRefresh();
  }

  function release(slot: StatusSlot): void {
    slot.demand -= 1;
    totalDemand -= 1;
    if (totalDemand === 0) {
      stopFocus?.();
      stopFocus = null;
    }
    if (slot.demand > 0) return;
    queue.cancel(slot);
    slot.scheduler.dispose();
    slot.dirty = false;
    evict();
  }

  return {
    retain(repoId) {
      const slot = ensure(repoId);
      slots.delete(repoId);
      slots.set(repoId, slot);
      slot.demand += 1;
      totalDemand += 1;
      stopFocus ??= deps.listenFocus(onFocus);
      if (slot.demand === 1) {
        slot.scheduler.reset();
        slot.retryAttempt = 0;
        if (host.ready) slot.scheduler.requestRefresh();
        else setSlotPhase(context, slot, { kind: "waiting" });
      }
      evict();
      let released = false;
      return () => {
        if (released) return;
        released = true;
        release(slot);
      };
    },
    subscribe(repoId, listener) {
      const set = listeners.get(repoId) ?? new Set<() => void>();
      listeners.set(repoId, set);
      set.add(listener);
      return () => {
        set.delete(listener);
        if (set.size === 0 && listeners.get(repoId) === set) listeners.delete(repoId);
      };
    },
    getSlot: (repoId) => slots.get(repoId)?.view ?? EMPTY_VIEW,
    routeAgentEvent(event) {
      if (event.type !== "sourceControlChanged") return;
      const slot = slots.get(event.repoId);
      if (slot !== undefined && slot.demand > 0) slot.scheduler.notifyChanged(Date.now());
    },
    setHostState(next) {
      const previous = host;
      host = next;
      if (!next.ready) {
        for (const slot of slots.values()) setSlotPhase(context, slot, { kind: "waiting" });
        return;
      }
      if (!previous.ready || previous.client !== next.client) {
        for (const slot of demanded()) forceRead(slot);
      } else if (previous.documentHidden && !next.documentHidden) {
        for (const slot of demanded()) if (slot.dirty) slot.scheduler.requestRefresh();
      }
    },
    markRehydrated() {
      for (const slot of demanded()) forceRead(slot);
    },
    setRepositories(repos) {
      identities.clear();
      for (const repo of repos) identities.set(repo.repoId, streamRootIdentity(repo.repoId, repo.root));
      for (const [repoId, slot] of slots) {
        const identity = identityOf(repoId);
        if (slot.rootIdentity === identity) continue;
        if (slot.demand > 0) {
          resetSlotRoot(context, queue, slot, identity);
          continue;
        }
        slot.scheduler.dispose();
        slots.delete(repoId);
      }
    },
    attachWriter(repoId, writer) {
      const slot = ensure(repoId);
      slot.writer = writer;
      return () => {
        if (slot.writer === writer) slot.writer = null;
      };
    },
    applyStatus(repoId, status) {
      const slot = slots.get(repoId);
      if (slot === undefined) return;
      slot.generation += 1;
      slot.retryAttempt = 0;
      context.setView(slot, { status, phase: { kind: "ready" } });
      slot.writer?.acceptStatus(status);
    },
    applyError(repoId, phase) {
      const slot = slots.get(repoId);
      if (slot === undefined) return;
      slot.generation += 1;
      slot.retryAttempt = 0;
      context.setView(slot, { status: null, phase });
    },
    actionStarted(repoId) {
      const slot = ensure(repoId);
      slot.actionPending = true;
      slot.scheduler.actionStarted();
    },
    actionFinished(repoId, appliedAt) {
      const slot = slots.get(repoId);
      if (slot === undefined) return;
      slot.actionPending = false;
      slot.scheduler.actionFinished(appliedAt);
    },
    schedulerFor: (repoId) => ensure(repoId).scheduler,
    refresh(repoId) {
      const slot = ensure(repoId);
      slot.retryAttempt = 0;
      slot.scheduler.requestRefresh();
    },
    readsInFlight: () => queue.activeCount(),
  };
}

/** The one registry the activity host feeds and every status consumer shares */
export const statusRegistry: StatusRegistry<AgentClient> = createStatusRegistry<AgentClient>({
  read: async (client, repoId) => (await sendSourceControlStatus(client, repoId)).status,
  listenFocus(onFocus) {
    window.addEventListener("focus", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
    };
  },
});
