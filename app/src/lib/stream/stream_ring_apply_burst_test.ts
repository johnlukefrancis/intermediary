// Path: app/src/lib/stream/stream_ring_apply_burst_test.ts
// Description: Burst reducer rules: open at the threshold, absorb, close on quiet, collapse against the open burst, notice TTL

import { test } from "node:test";
import assert from "node:assert/strict";
import { BURST_THRESHOLD, NOTICE_TTL_MS } from "./stream_bounds.js";
import { newBurstCard } from "./stream_burst_card.js";
import { admit, openBurst } from "./stream_ring.js";
import { applyFileDelta } from "./stream_ring_apply.js";
import { applyFileChanged, collapsePending, settleReduce } from "./stream_ring_apply_burst.js";
import { initialReduceState } from "./stream_ring_apply_support.js";
import { changed, resetSeq, textDelta } from "./testing/stream_fixtures.js";
import type { StreamBurstCard, StreamReduceState } from "./stream_types.js";

const opts = { outsideSelection: (): boolean => false };

/** The pending card at `index` as a burst card, or a failed assertion */
function pendingBurst(state: StreamReduceState, index = 0): StreamBurstCard {
  const card = state.pending[index];
  assert.equal(card?.kind, "burst");
  return card;
}

function withPendingFiles(state: StreamReduceState, count: number, firstMs: number): StreamReduceState {
  let next = state;
  for (let index = 0; index < count; index += 1) {
    next = applyFileDelta(next, textDelta(`app/f${String(index)}.ts`), firstMs + index, opts);
  }
  return next;
}

void test("distinct fileChanged paths open a burst; deltas for absorbed paths bump resolved, not cards", () => {
  resetSeq();
  let state = initialReduceState();
  for (let index = 0; index < BURST_THRESHOLD; index += 1) {
    state = applyFileChanged(state, changed(`src/f${String(index)}.ts`, index === 0 ? "add" : "change"), 1000 + index);
  }
  assert.notEqual(state.ring.burstOpen, null);
  assert.equal(state.pending.length, 1);
  const burst = pendingBurst(state);
  assert.equal(burst.files, BURST_THRESHOLD);
  assert.equal(burst.byOp.add, 1);
  assert.equal(burst.byKind.code, BURST_THRESHOLD);
  assert.equal(burst.topDirs[0]?.dir, "src");
  state = applyFileDelta(state, textDelta("src/f3.ts"), 1100, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(pendingBurst(state).resolved, 1);
  state = applyFileDelta(state, textDelta("elsewhere.ts"), 1100, opts);
  assert.equal(state.pending.length, 2);
  state = settleReduce(state, 1100);
  assert.notEqual(state.ring.burstOpen, null);
  state = settleReduce(state, 5000);
  assert.equal(state.ring.burstOpen, null);
});

void test("a pending backlog at the threshold collapses into one new burst spanning the oldest card", () => {
  resetSeq();
  let state = withPendingFiles(initialReduceState(), BURST_THRESHOLD, 1000);
  assert.equal(state.pending.length, BURST_THRESHOLD);
  state = collapsePending(state, 2000);
  assert.equal(state.pending.length, 1);
  const burst = pendingBurst(state);
  assert.equal(burst.files, BURST_THRESHOLD);
  assert.equal(burst.resolved, BURST_THRESHOLD);
  assert.equal(burst.arrivedAtMs, 1000);
  assert.equal(burst.elapsedMs, 1000);
  assert.equal(state.ring.burstOpen?.id, burst.id);
  assert.equal(state.ring.burstOpen.paths.size, BURST_THRESHOLD);
});

void test("collapse folds into the OPEN pending burst and keeps every other pending burst in the FIFO", () => {
  resetSeq();
  let state = initialReduceState();
  const closed = newBurstCard(900, 500);
  const open = newBurstCard(901, 800);
  state = { ...state, pending: [closed, open], ring: openBurst(state.ring, open.id, 2000) };
  state = withPendingFiles(state, BURST_THRESHOLD, 1000);
  state = collapsePending(state, 1500);
  assert.deepEqual(state.pending.map((card) => card.id), [closed.id, open.id]);
  assert.equal(pendingBurst(state, 0).files, 0);
  assert.equal(pendingBurst(state, 1).files, BURST_THRESHOLD);
  assert.equal(pendingBurst(state, 1).resolved, BURST_THRESHOLD);
  assert.equal(state.ring.burstOpen?.id, open.id);
});

void test("collapse with the open burst already in the ring opens a fresh burst card instead", () => {
  resetSeq();
  let state = initialReduceState();
  const admitted = newBurstCard(900, 500);
  state = { ...state, ring: openBurst(admit(state.ring, admitted), admitted.id, 2000) };
  state = withPendingFiles(state, BURST_THRESHOLD, 1000);
  state = collapsePending(state, 1500);
  assert.equal(state.pending.length, 1);
  const fresh = pendingBurst(state);
  assert.notEqual(fresh.id, admitted.id);
  assert.equal(fresh.files, BURST_THRESHOLD);
  assert.equal(fresh.arrivedAtMs, 1000);
  assert.equal(state.ring.burstOpen?.id, fresh.id);
  const ringBurst = state.ring.cards[0];
  assert.equal(ringBurst?.kind, "burst");
  assert.equal(ringBurst.files, 0);
});

void test("notices older than NOTICE_TTL_MS leave on settle; the same state comes back when none aged", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts", { withheld: 2 }), 1000, opts);
  assert.equal(state.ring.notices.length, 1);
  assert.equal(settleReduce(state, 1000 + NOTICE_TTL_MS - 1), state);
  state = settleReduce(state, 1000 + NOTICE_TTL_MS);
  assert.equal(state.ring.notices.length, 0);
});
