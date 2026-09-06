// Path: app/src/lib/stream/stream_ring_apply.ts
// Description: Pure reducers turning fileDelta and fileDeltaCounters events into cards, merges, and honesty notices

import { isVisibleFileKind } from "../files/file_feed.js";
import type { FileDeltaCountersEvent, FileDeltaEvent } from "../../shared/protocol.js";
import { MERGE_WINDOW_MS } from "./stream_bounds.js";
import { bodyFor, extendCard } from "./stream_card_body.js";
import { formatClock } from "./stream_card_grammar.js";
import { applyImageDelta } from "./stream_image_strip.js";
import { burstOwning } from "./stream_ring.js";
import {
  newestCardOfPath,
  notice,
  replaceCard,
  takeId,
  updateBurst,
  withoutSettling,
  type ApplyOptions,
} from "./stream_ring_apply_support.js";
import type { StreamFileCard, StreamReduceState } from "./stream_types.js";

interface DeltaCounters {
  withheld: number;
  dropped: number;
}

/** Edits lost anywhere on the way (agent queue, bus, or the store's own intake) share one notice */
export function applyDropped(state: StreamReduceState, dropped: number, now: number): StreamReduceState {
  if (dropped <= 0) return state;
  return notice(state, "dropped", "error", dropped, (n) => `${String(n)} EDITS DROPPED`, now);
}

/** The agent's honesty counters print as notices; the same keys merge across deltas and counter events */
function applyCounters(state: StreamReduceState, counters: DeltaCounters, now: number): StreamReduceState {
  let next = state;
  if (counters.withheld > 0) {
    next = notice(next, "withheld", "warning", counters.withheld, (n) => `${String(n)} EDITS WITHHELD · BURST`, now);
  }
  return applyDropped(next, counters.dropped, now);
}

/**
 * Deltas and counters events consume one sequence: a restart is a new stream, a forward gap is a
 * drop (bus or queue) the user should know about, and either event advances `lastSeq`.
 */
function applySeq(state: StreamReduceState, seq: number, now: number): StreamReduceState {
  const last = state.ring.lastSeq;
  const next: StreamReduceState = { ...state, ring: { ...state.ring, lastSeq: seq } };
  if (last === null || seq === 1 || seq <= last) return next;
  const gap = seq - last - 1;
  return gap > 0 ? notice(next, "gap", "warning", gap, (n) => `${String(n)} EDITS NOT SHOWN`, now) : next;
}

/** Counters the agent would otherwise strand when its queue goes quiet or a burst window closes */
export function applyDeltaCounters(
  state: StreamReduceState,
  event: FileDeltaCountersEvent,
  now: number
): StreamReduceState {
  return applyCounters(applySeq(state, event.seq, now), event, now);
}

function canExtend(card: StreamFileCard, event: FileDeltaEvent, now: number): boolean {
  return (
    card.body.status === "text" &&
    event.payload.kind === "text" &&
    card.op !== "remove" &&
    event.op !== "remove" &&
    now - card.updatedAtMs <= MERGE_WINDOW_MS
  );
}

export function applyFileDelta(
  state: StreamReduceState,
  event: FileDeltaEvent,
  now: number,
  opts: ApplyOptions
): StreamReduceState {
  if (!isVisibleFileKind(event.kind)) return state;
  const fileKind = event.kind;
  let next = applyCounters(applySeq(withoutSettling(state, event.path), event.seq, now), event, now);

  // A member's delta lands on its burst while the burst is open or inside its grace after closing
  const owner = burstOwning(next.ring, event.path, now);
  if (owner !== null) {
    const bumped = updateBurst(next, owner, (card) => ({ ...card, resolved: card.resolved + 1, updatedAtMs: now }));
    // The burst card already left the ring: it can show nothing, so the delta takes the ordinary path
    if (bumped.matched) return bumped.state;
  }
  const payload = event.payload;
  // An image path always lands in a strip, whatever its payload: an opaque one is a NO PREVIEW tile
  if (fileKind === "image") return applyImageDelta(next, event, payload, now, opts);
  if (payload.kind === "text" && event.op === "modify" && payload.stats.added + payload.stats.removed === 0) {
    return next;
  }
  // A re-edit extends the newest card of the same path in place, even with other paths after it
  const newest = newestCardOfPath(next, event.path);
  if (newest !== null && canExtend(newest.card, event, now)) {
    return replaceCard(next, newest.where, extendCard(newest.card, event, now));
  }
  const [id, allocated] = takeId(next);
  next = allocated;
  const card: StreamFileCard = {
    kind: "file",
    id,
    path: event.path,
    fromPath: event.fromPath ?? null,
    fileKind,
    op: event.op,
    tracked: event.tracked ?? null,
    outsideSelection: opts.outsideSelection(event.path),
    clock: formatClock(now),
    arrivedAtMs: now,
    updatedAtMs: now,
    admittedAtMs: 0,
    edits: 1,
    expanded: false,
    exiting: false,
    static: false,
    body: bodyFor(payload),
  };
  return { ...next, pending: [...next.pending, card] };
}
