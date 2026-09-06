// Path: app/src/lib/stream/stream_ring_apply.ts
// Description: Pure reducers turning fileDelta and fileDeltaCounters events into cards, merges, and honesty notices

import { isVisibleFileKind } from "../files/file_feed.js";
import type { FileDeltaCountersEvent, FileDeltaEvent } from "../../shared/protocol.js";
import { MERGE_WINDOW_MS } from "./stream_bounds.js";
import { bodyFor, extendCard } from "./stream_card_body.js";
import { formatClock } from "./stream_card_grammar.js";
import { applyImageDelta } from "./stream_image_strip.js";
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

/** The agent's honesty counters print as notices; the same keys merge across deltas and counter events */
function applyCounters(state: StreamReduceState, counters: DeltaCounters, now: number): StreamReduceState {
  let next = state;
  if (counters.withheld > 0) {
    next = notice(next, "withheld", "warning", counters.withheld, (n) => `${String(n)} EDITS WITHHELD · BURST`, now);
  }
  if (counters.dropped > 0) {
    next = notice(next, "dropped", "error", counters.dropped, (n) => `${String(n)} EDITS DROPPED`, now);
  }
  return next;
}

/** seq restarts are a new stream; a forward gap is a bus drop the user should know about */
function applySeq(state: StreamReduceState, event: FileDeltaEvent, now: number): StreamReduceState {
  const last = state.ring.lastSeq;
  let next: StreamReduceState = { ...state, ring: { ...state.ring, lastSeq: event.seq } };
  if (last !== null && event.seq !== 1 && event.seq > last) {
    const gap = event.seq - last - 1;
    if (gap > 0) next = notice(next, "gap", "warning", gap, (n) => `${String(n)} EDITS NOT SHOWN`, now);
  }
  return applyCounters(next, event, now);
}

/** Counters the agent would otherwise strand when its queue goes quiet or a burst window closes */
export function applyDeltaCounters(
  state: StreamReduceState,
  event: FileDeltaCountersEvent,
  now: number
): StreamReduceState {
  return applyCounters(state, event, now);
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
  let next = applySeq(withoutSettling(state, event.path), event, now);

  const burst = next.ring.burstOpen;
  if (burst !== null && burst.paths.has(event.path)) {
    return updateBurst(next, burst.id, (card) => ({ ...card, resolved: card.resolved + 1, updatedAtMs: now }));
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
