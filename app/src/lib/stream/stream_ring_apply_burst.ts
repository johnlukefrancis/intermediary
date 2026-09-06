// Path: app/src/lib/stream/stream_ring_apply_burst.ts
// Description: Pure reducers for fileChanged arrivals: burst detection and absorption, settling, backlog collapse

import { isVisibleFileKind, type VisibleFileKind } from "../files/file_feed.js";
import type { DeltaOp, FileChangedEvent } from "../../shared/protocol.js";
import { BURST_CLOSE_MS, SETTLING_MAX, SETTLING_TTL_MS } from "./stream_bounds.js";
import { noteArrival, shouldCloseBurst, shouldOpenBurst, windowArrivals } from "./stream_burst_detect.js";
import { absorbIntoBurstCard, newBurstCard, opForChangeType } from "./stream_burst_card.js";
import { shouldCollapse } from "./stream_cadence.js";
import { absorbIntoBurst, closeBurst, expireNotices, openBurst } from "./stream_ring.js";
import { takeId, updateBurst } from "./stream_ring_apply_support.js";
import type { StreamBurstCard, StreamFileCard, StreamImageStripCard, StreamReduceState } from "./stream_types.js";

/** Records one path in the open burst on both the ring state and the burst card */
function absorbPath(
  state: StreamReduceState,
  path: string,
  op: DeltaOp,
  fileKind: VisibleFileKind,
  now: number
): StreamReduceState {
  const absorbed = absorbIntoBurst(state.ring, path, now + BURST_CLOSE_MS);
  const open = absorbed.ring.burstOpen;
  if (open === null) return state;
  const next = { ...state, ring: absorbed.ring };
  const absorb = { op, fileKind, newPath: absorbed.newPath, dirCounts: open.dirCounts, now };
  return updateBurst(next, open.id, (card) => absorbIntoBurstCard(card, absorb));
}

/** Feeds burst detection and the settling line; a burst absorbs every path while it is open */
export function applyFileChanged(state: StreamReduceState, event: FileChangedEvent, now: number): StreamReduceState {
  if (!isVisibleFileKind(event.kind)) return state;
  const fileKind = event.kind;
  const op = opForChangeType(event.changeType);
  let next: StreamReduceState = {
    ...state,
    burstDetect: noteArrival(state.burstDetect, { path: event.path, op, fileKind }, now),
  };
  if (fileKind !== "image") {
    const settling = [...next.settling.filter((entry) => entry.path !== event.path), { path: event.path, atMs: now }];
    next = { ...next, settling: settling.slice(Math.max(0, settling.length - SETTLING_MAX)) };
  }
  if (next.ring.burstOpen !== null) return absorbPath(next, event.path, op, fileKind, now);
  if (!shouldOpenBurst(next.burstDetect, now)) return next;
  const [id, allocated] = takeId(next);
  next = {
    ...allocated,
    pending: [...allocated.pending, newBurstCard(id, now)],
    ring: openBurst(allocated.ring, id, now + BURST_CLOSE_MS),
  };
  for (const arrival of windowArrivals(next.burstDetect, now)) {
    next = absorbPath(next, arrival.path, arrival.op, arrival.fileKind, now);
  }
  return next;
}

/** Quiet closes the burst; stale settling paths and old notices are forgotten */
export function settleReduce(state: StreamReduceState, now: number): StreamReduceState {
  let next = state;
  if (next.ring.burstOpen !== null && shouldCloseBurst(next.burstDetect, now)) {
    next = { ...next, ring: closeBurst(next.ring) };
  }
  if (next.settling.some((entry) => now - entry.atMs >= SETTLING_TTL_MS)) {
    next = { ...next, settling: next.settling.filter((entry) => now - entry.atMs < SETTLING_TTL_MS) };
  }
  const ring = expireNotices(next.ring, now);
  return ring === next.ring ? next : { ...next, ring };
}

/**
 * A pending backlog at BURST_THRESHOLD is a DOM bound: the file cards and image strips fold
 * into one burst card, every tile of a strip counted as one absorbed path. The target is the
 * pending card of the open burst when there is one; otherwise a new burst card is opened, its
 * clock starting at the OLDEST folded card so the span is the real window. Every other pending
 * burst card keeps its place in the FIFO.
 */
export function collapsePending(state: StreamReduceState, now: number): StreamReduceState {
  if (!shouldCollapse(state.pending.length)) return state;
  const folded = state.pending.filter(
    (card): card is StreamFileCard | StreamImageStripCard => card.kind !== "burst"
  );
  const oldest = folded[0];
  if (oldest === undefined) return state;
  const bursts = state.pending.filter((card): card is StreamBurstCard => card.kind === "burst");
  const openId = state.ring.burstOpen?.id;
  let target = openId === undefined ? undefined : bursts.find((card) => card.id === openId);
  let next: StreamReduceState = state;
  if (target === undefined) {
    const [id, allocated] = takeId(next);
    target = newBurstCard(id, now, oldest.arrivedAtMs);
    bursts.push(target);
    next = { ...allocated, ring: openBurst(allocated.ring, id, now + BURST_CLOSE_MS) };
  }
  next = { ...next, pending: bursts };
  let resolved = 0;
  for (const card of folded) {
    if (card.kind === "file") {
      next = absorbPath(next, card.path, card.op, card.fileKind, now);
      resolved += 1;
      continue;
    }
    for (const tile of card.tiles) next = absorbPath(next, tile.path, tile.op, "image", now);
    resolved += card.tiles.length;
  }
  const targetId = target.id;
  return updateBurst(next, targetId, (card) => ({ ...card, resolved: card.resolved + resolved }));
}
