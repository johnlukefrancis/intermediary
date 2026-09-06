// Path: app/src/lib/stream/stream_cadence_test.ts
// Description: Cadence clamp and pressure band arithmetic

import { test } from "node:test";
import assert from "node:assert/strict";
import { cadenceMs, pressureBand, shouldCollapse } from "./stream_cadence.js";
import { BURST_THRESHOLD, CADENCE_BASE_MS, CADENCE_MIN_MS, LAG_BUDGET_MS } from "./stream_bounds.js";

void test("cadence rests at the base with no backlog and never drops below the floor", () => {
  assert.equal(cadenceMs(0), CADENCE_BASE_MS);
  assert.equal(cadenceMs(1), CADENCE_BASE_MS);
  assert.equal(cadenceMs(1000), CADENCE_MIN_MS);
});

void test("cadence spreads the lag budget over a mid backlog", () => {
  const backlog = 10;
  assert.equal(cadenceMs(backlog), LAG_BUDGET_MS / backlog);
});

void test("pressure bands: calm 0-3, busy 4-11, flood 12+", () => {
  assert.equal(pressureBand(0), "calm");
  assert.equal(pressureBand(3), "calm");
  assert.equal(pressureBand(4), "busy");
  assert.equal(pressureBand(11), "busy");
  assert.equal(pressureBand(12), "flood");
});

void test("a backlog at the burst threshold collapses", () => {
  assert.equal(shouldCollapse(BURST_THRESHOLD - 1), false);
  assert.equal(shouldCollapse(BURST_THRESHOLD), true);
});
