// Path: app/src/lib/stream/stream_burst_detect_test.ts
// Description: Distinct-path rate detection over fileChanged arrivals: open at the threshold, close on quiet

import { test } from "node:test";
import assert from "node:assert/strict";
import { BURST_CLOSE_MS, BURST_THRESHOLD, BURST_WINDOW_MS } from "./stream_bounds.js";
import {
  INITIAL_BURST_DETECT,
  distinctPaths,
  noteArrival,
  shouldCloseBurst,
  shouldOpenBurst,
  type BurstDetectState,
} from "./stream_burst_detect.js";

function arrivals(count: number, startMs: number, stepMs: number, prefix = "p"): BurstDetectState {
  let state = INITIAL_BURST_DETECT;
  for (let index = 0; index < count; index += 1) {
    state = noteArrival(state, { path: `${prefix}${String(index)}`, op: "modify", fileKind: "code" }, startMs + index * stepMs);
  }
  return state;
}

void test("the same path re-marked counts once", () => {
  let state = INITIAL_BURST_DETECT;
  for (let index = 0; index < 50; index += 1) {
    state = noteArrival(state, { path: "same.ts", op: "modify", fileKind: "code" }, 1000 + index);
  }
  assert.equal(distinctPaths(state, 1050), 1);
  assert.equal(shouldOpenBurst(state, 1050), false);
});

void test("distinct paths inside the window open a burst at the threshold", () => {
  const below = arrivals(BURST_THRESHOLD - 1, 1000, 10);
  assert.equal(shouldOpenBurst(below, 1000 + BURST_THRESHOLD * 10), false);
  const at = arrivals(BURST_THRESHOLD, 1000, 10);
  assert.equal(shouldOpenBurst(at, 1000 + BURST_THRESHOLD * 10), true);
});

void test("arrivals older than the window fall out of the count", () => {
  const spread = arrivals(BURST_THRESHOLD, 1000, BURST_WINDOW_MS);
  const now = 1000 + BURST_THRESHOLD * BURST_WINDOW_MS;
  assert.equal(distinctPaths(spread, now), 1);
  assert.equal(shouldOpenBurst(spread, now), false);
});

void test("a burst closes only after the quiet period", () => {
  const state = arrivals(3, 1000, 10);
  const last = 1020;
  assert.equal(shouldCloseBurst(state, last + BURST_CLOSE_MS - 1), false);
  assert.equal(shouldCloseBurst(state, last + BURST_CLOSE_MS), true);
  assert.equal(shouldCloseBurst(INITIAL_BURST_DETECT, 0), true);
});
