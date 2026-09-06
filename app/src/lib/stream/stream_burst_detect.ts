// Path: app/src/lib/stream/stream_burst_detect.ts
// Description: Pure distinct-path rate detector over fileChanged arrivals that opens and closes bursts

import type { VisibleFileKind } from "../files/file_feed.js";
import type { DeltaOp } from "../../shared/protocol.js";
import { BURST_CLOSE_MS, BURST_THRESHOLD, BURST_WINDOW_MS } from "./stream_bounds.js";

export interface BurstArrival {
  path: string;
  atMs: number;
  /** What the last fileChanged for the path said, so an opening burst can absorb the whole window */
  op: DeltaOp;
  fileKind: VisibleFileKind;
}

export interface BurstDetectState {
  /** Newest last; one entry per distinct path inside the window */
  readonly recent: readonly BurstArrival[];
  readonly lastArrivalMs: number | null;
}

export const INITIAL_BURST_DETECT: BurstDetectState = { recent: [], lastArrivalMs: null };

/** Precision past the threshold buys nothing, so the window never holds more than this */
const RECENT_MAX = BURST_THRESHOLD * 2;

/** Record one fileChanged path at `now`, keeping only the distinct paths still inside the window */
export function noteArrival(
  state: BurstDetectState,
  arrival: Omit<BurstArrival, "atMs">,
  now: number
): BurstDetectState {
  const floor = now - BURST_WINDOW_MS;
  const kept = state.recent.filter((entry) => entry.atMs >= floor && entry.path !== arrival.path);
  kept.push({ ...arrival, atMs: now });
  const recent = kept.length > RECENT_MAX ? kept.slice(kept.length - RECENT_MAX) : kept;
  return { recent, lastArrivalMs: now };
}

/** Distinct paths seen inside the window ending at `now` */
export function distinctPaths(state: BurstDetectState, now: number): number {
  const floor = now - BURST_WINDOW_MS;
  return state.recent.reduce((count, entry) => (entry.atMs >= floor ? count + 1 : count), 0);
}

export function shouldOpenBurst(state: BurstDetectState, now: number): boolean {
  return distinctPaths(state, now) >= BURST_THRESHOLD;
}

/** The arrivals that crossed the threshold together; the burst card absorbs them from the start */
export function windowArrivals(state: BurstDetectState, now: number): readonly BurstArrival[] {
  const floor = now - BURST_WINDOW_MS;
  return state.recent.filter((entry) => entry.atMs >= floor);
}

/** An open burst closes after BURST_CLOSE_MS without an arrival */
export function shouldCloseBurst(state: BurstDetectState, now: number): boolean {
  return state.lastArrivalMs === null || now - state.lastArrivalMs >= BURST_CLOSE_MS;
}
