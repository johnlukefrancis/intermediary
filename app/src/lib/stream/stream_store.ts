// Path: app/src/lib/stream/stream_store.ts
// Description: Per-repo Stream store outside React: intake buffer, reducers, the cadence conductor, and the snapshot

import type { AgentEvent } from "../../shared/protocol.js";
import { isVisibleFileKind } from "../files/file_feed.js";
import { FLUSH_MS, IDLE_WAKE_MS, STATIC_AFTER_MS } from "./stream_bounds.js";
import { cadenceMs } from "./stream_cadence.js";
import { admit, expand, markStatic, needsSettle, nextNoticeExpiryMs, pushNotice, seedHistory, spliceExited } from "./stream_ring.js";
import { applyDeltaCounters, applyFileDelta } from "./stream_ring_apply.js";
import { applyFileChanged, collapsePending, settleReduce } from "./stream_ring_apply_burst.js";
import { initialReduceState } from "./stream_ring_apply_support.js";
import { OFFLINE_TRANSPORT, browserStoreDeps, buildSnapshot, isHeld, remapSelection } from "./stream_store_support.js";
import type {
  StreamHistorySeed,
  StreamReduceState,
  StreamSelectionFilter,
  StreamSnapshot,
  StreamStore,
  StreamStoreDeps,
  StreamTimerHandle,
} from "./stream_types.js";

type SnapshotEvent = Extract<AgentEvent, { type: "snapshot" }>;

/**
 * The only seed route: the repo's own snapshot (flush drops every foreign event), and only while
 * the ring holds no card — a repeat snapshot, a reconnect, or a late arrival never rewrites live cards.
 */
function seedFromSnapshot(state: StreamReduceState, event: SnapshotEvent): StreamReduceState {
  const seeds = event.recent.flatMap((file): StreamHistorySeed[] =>
    isVisibleFileKind(file.kind)
      ? [{ path: file.path, fileKind: file.kind, lastSeenAtIso: file.activity?.lastSeenAtIso ?? file.mtime }]
      : []
  );
  const ring = seedHistory(state.ring, seeds, state.nextId);
  return ring === state.ring ? state : { ...state, ring, nextId: state.nextId + ring.cards.length };
}

export function createStreamStore(repoId: string, deps: StreamStoreDeps = browserStoreDeps()): StreamStore {
  let state: StreamReduceState = initialReduceState();
  let intake: AgentEvent[] = [];
  let flushTimer: StreamTimerHandle | null = null;
  let tickTimer: StreamTimerHandle | null = null;
  /** The armed tick only sweeps; a new arrival may preempt it for an admit */
  let tickIsSettle = false;
  let visible = false;
  let documentHidden = false;
  let reducedMotion = false;
  let transport = OFFLINE_TRANSPORT;
  let selectionFilter: StreamSelectionFilter = null;
  let admittedWhileAway = 0;
  let lastAdmitAt = Number.NEGATIVE_INFINITY;
  let disposed = false;
  let seq = 0;
  const listeners = new Set<() => void>();

  const outsideSelection = (path: string): boolean => selectionFilter !== null && !selectionFilter(path);

  const build = (): StreamSnapshot =>
    buildSnapshot({ state, visible, documentHidden, transport, admittedWhileAway, seq });
  let snapshot = build();

  function commit(): void {
    seq += 1;
    snapshot = build();
    for (const listener of listeners) listener();
  }

  function addNotice(key: string, tone: "accent" | "success" | "warning", count: number, text: (n: number) => string): void {
    const id = state.nextId;
    state = { ...state, nextId: id + 1, ring: pushNotice(state.ring, { key, tone, count, textFor: text }, deps.now(), id) };
  }

  function clearTick(): void {
    if (tickTimer === null) return;
    deps.clearTimer(tickTimer);
    tickTimer = null;
  }

  /** Admission stamps the static clock; the card is resolved already */
  function admitOne(now: number): void {
    const [card, ...rest] = state.pending;
    if (card === undefined) return;
    state = { ...state, pending: rest, ring: markStatic(admit(state.ring, { ...card, admittedAtMs: now }), now) };
    lastAdmitAt = now;
    if (!visible) admittedWhileAway += 1;
  }

  function admitAll(now: number): void {
    while (state.pending.length > 0) admitOne(now);
  }

  function sweep(now: number): void {
    state = settleReduce(state, now);
    state = { ...state, ring: markStatic(spliceExited(state.ring), now) };
  }

  function canCascade(): boolean {
    return visible && !documentHidden && !reducedMotion;
  }

  function arm(ms: number, settle: boolean): void {
    tickTimer = deps.setTimer(tick, ms);
    tickIsSettle = settle;
  }

  /** One timer chain: the next admit at cadence, or a settle tick once the backlog is empty */
  function schedule(now: number): void {
    if (disposed || documentHidden) return;
    if (tickTimer !== null && (!tickIsSettle || state.pending.length === 0)) return;
    clearTick();
    if (state.pending.length > 0) {
      if (!canCascade()) {
        admitAll(now);
        commit();
      } else {
        const idle = now - lastAdmitAt >= IDLE_WAKE_MS;
        const wait = idle ? 0 : Math.max(0, lastAdmitAt + cadenceMs(state.pending.length) - now);
        if (wait > 0) {
          arm(wait, false);
          return;
        }
        admitOne(now);
        commit();
        if (state.pending.length > 0) {
          arm(cadenceMs(state.pending.length), false);
          return;
        }
      }
    }
    if (needsSettle(state.ring) || state.settling.length > 0 || state.ring.burstOpen !== null) {
      arm(STATIC_AFTER_MS, true);
      return;
    }
    const expiry = nextNoticeExpiryMs(state.ring, now);
    if (expiry !== null) arm(expiry, true);
  }

  function tick(): void {
    tickTimer = null;
    if (disposed) return;
    const now = deps.now();
    if (state.pending.length > 0 && canCascade()) admitOne(now);
    else sweep(now);
    commit();
    schedule(now);
  }

  function flush(): void {
    flushTimer = null;
    if (disposed) return;
    const now = deps.now();
    const events = intake;
    intake = [];
    state = settleReduce(state, now);
    for (const event of events) {
      // Every event this store folds names this repo; a misrouted or stale one never lands a card
      if (!("repoId" in event) || event.repoId !== repoId) continue;
      if (event.type === "fileDelta") state = applyFileDelta(state, event, now, { outsideSelection });
      else if (event.type === "fileChanged") state = applyFileChanged(state, event, now);
      else if (event.type === "fileDeltaCounters") state = applyDeltaCounters(state, event, now);
      else if (event.type === "snapshot") state = seedFromSnapshot(state, event);
    }
    state = collapsePending(state, now);
    commit();
    schedule(now);
  }

  return {
    repoId,
    intake(event) {
      if (disposed) return;
      intake.push(event);
      flushTimer ??= deps.setTimer(flush, FLUSH_MS);
    },
    setVisible(next) {
      if (visible === next) return;
      visible = next;
      const now = deps.now();
      if (next) {
        sweep(now);
        if (admittedWhileAway > 0) {
          addNotice("away", "accent", admittedWhileAway, (n) => `${String(n)} CHANGES WHILE AWAY`);
          admittedWhileAway = 0;
        }
      }
      commit();
      schedule(now);
    },
    setDocumentHidden(hidden) {
      if (documentHidden === hidden) return;
      documentHidden = hidden;
      if (hidden) {
        clearTick();
        commit();
        return;
      }
      const now = deps.now();
      state = collapsePending(state, now);
      commit();
      schedule(now);
    },
    setTransport(next) {
      const becameHeld = isHeld(next) && !isHeld(transport);
      transport = next;
      if (becameHeld) addNotice("held", "warning", 1, () => "WSL BACKEND OFFLINE — STREAM HELD");
      commit();
      // An idle store has no timer chain: the notice needs its own expiry tick
      schedule(deps.now());
    },
    setReducedMotion(reduced) {
      if (reducedMotion === reduced) return;
      reducedMotion = reduced;
      schedule(deps.now());
    },
    setSelectionFilter(filter) {
      selectionFilter = filter;
      state = remapSelection(state, outsideSelection);
      commit();
    },
    markRehydrated() {
      state = { ...state, ring: { ...state.ring, lastSeq: null } };
      addNotice("reconnected", "success", 1, () => "RECONNECTED — RESUMING");
      commit();
      schedule(deps.now());
    },
    expand(id) {
      const ring = expand(state.ring, id);
      if (ring === state.ring) return;
      state = { ...state, ring };
      commit();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => { listeners.delete(listener); };
    },
    getSnapshot: () => snapshot,
    dispose() {
      disposed = true;
      clearTick();
      if (flushTimer !== null) deps.clearTimer(flushTimer);
      flushTimer = null;
      listeners.clear();
    },
  };
}
