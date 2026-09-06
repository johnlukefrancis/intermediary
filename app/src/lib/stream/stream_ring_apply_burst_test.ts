// Path: app/src/lib/stream/stream_ring_apply_burst_test.ts
// Description: Burst reducer rules: open at the threshold, absorb, close on quiet into a grace, eviction fall-through, membership caps, collapse, notice TTL

import { test } from "node:test";
import assert from "node:assert/strict";
import { BURST_ABSORB_GRACE_MS, BURST_CLOSE_MS, BURST_MEMBER_CAP, BURST_THRESHOLD, BURST_TOP_DIRS_TRACKED, NOTICE_TTL_MS, RING_SIZE } from "./stream_bounds.js";
import { BURST_OTHER_DIR, newBurstCard } from "./stream_burst_card.js";
import { admit, openBurst } from "./stream_ring.js";
import { applyFileDelta } from "./stream_ring_apply.js";
import { applyFileChanged, collapsePending, settleReduce } from "./stream_ring_apply_burst.js";
import { initialReduceState } from "./stream_ring_apply_support.js";
import { changed, fileCard, resetSeq, textDelta } from "./testing/stream_fixtures.js";
import type { StreamBurstCard, StreamReduceState, StreamRing, StreamRingCard } from "./stream_types.js";

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

/** Admits `card`, then pushes it out the way the ring does: flagged exiting by a younger card, then spliced */
function admitThenEvict(ring: StreamRing, card: StreamRingCard, at: number): StreamRing {
  let next = admit(ring, card);
  for (let index = 0; index <= RING_SIZE; index += 1) {
    next = admit(next, fileCard(900 + index, `later/f${String(index)}.ts`, at + index));
  }
  assert.ok(!next.cards.some((entry) => entry.id === card.id));
  return next;
}

/** Opens a burst over BURST_THRESHOLD distinct paths and returns the state with its pending card */
function openedBurst(): StreamReduceState {
  let state = initialReduceState();
  for (let index = 0; index < BURST_THRESHOLD; index += 1) {
    state = applyFileChanged(state, changed(`src/f${String(index)}.ts`), 1000 + index);
  }
  return state;
}

void test("a member's delta prints an ordinary card once the burst card has been evicted from the ring", () => {
  resetSeq();
  let state = openedBurst();
  const burst = pendingBurst(state);
  state = { ...state, pending: [], ring: admitThenEvict(state.ring, burst, 1100) };
  // The burst is still open over its members, but its card can show nothing: the delta must not vanish
  assert.equal(state.ring.burstOpen?.id, burst.id);
  state = applyFileDelta(state, textDelta("src/f1.ts"), 1200, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(state.pending[0]?.kind, "file");
});

void test("evicting a closed burst's card drops its absorb grace; a member inside the grace prints a card", () => {
  resetSeq();
  let state = openedBurst();
  const burst = pendingBurst(state);
  const closedAt = 1000 + BURST_CLOSE_MS * 2;
  state = settleReduce(state, closedAt);
  assert.equal(state.ring.burstGrace?.id, burst.id);
  state = { ...state, pending: [], ring: admitThenEvict(state.ring, burst, closedAt) };
  assert.equal(state.ring.burstGrace, null);
  state = applyFileDelta(state, textDelta("src/f2.ts"), closedAt + 1, opts);
  const printed = state.pending.at(-1);
  assert.equal(printed?.kind, "file");
});

void test("a burst card still waiting in the pending FIFO keeps its grace across an unrelated eviction", () => {
  resetSeq();
  let state = openedBurst();
  const burst = pendingBurst(state);
  const closedAt = 1000 + BURST_CLOSE_MS * 2;
  state = settleReduce(state, closedAt);
  state = { ...state, ring: admitThenEvict(state.ring, fileCard(800, "other.ts", closedAt), closedAt) };
  assert.equal(state.ring.burstGrace?.id, burst.id);
  state = applyFileDelta(state, textDelta("src/f3.ts"), closedAt + 1, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(pendingBurst(state).resolved, 1);
});

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

void test("a closed burst keeps absorbing its members' deltas inside BURST_ABSORB_GRACE_MS; after it a card prints", () => {
  resetSeq();
  let state = initialReduceState();
  for (let index = 0; index < BURST_THRESHOLD; index += 1) {
    state = applyFileChanged(state, changed(`src/f${String(index)}.ts`), 1000 + index);
  }
  const closedAt = 1000 + BURST_CLOSE_MS * 2;
  state = settleReduce(state, closedAt);
  assert.equal(state.ring.burstOpen, null);
  assert.equal(state.ring.burstGrace?.untilMs, closedAt + BURST_ABSORB_GRACE_MS);
  state = applyFileDelta(state, textDelta("src/f1.ts"), closedAt + BURST_ABSORB_GRACE_MS - 1, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(pendingBurst(state).resolved, 1);
  // A path the burst never held prints as a card even inside the grace
  state = applyFileDelta(state, textDelta("elsewhere.ts"), closedAt + 1, opts);
  assert.equal(state.pending.length, 2);
  state = applyFileDelta(state, textDelta("src/f2.ts"), closedAt + BURST_ABSORB_GRACE_MS, opts);
  assert.equal(state.pending.length, 3);
  assert.equal(pendingBurst(state).resolved, 1);
  state = settleReduce(state, closedAt + BURST_ABSORB_GRACE_MS);
  assert.equal(state.ring.burstGrace, null);
});

void test("membership is capped: past BURST_MEMBER_CAP a new path counts on the card but its delta prints as a card", () => {
  resetSeq();
  let state = initialReduceState();
  for (let index = 0; index < BURST_MEMBER_CAP + 2; index += 1) {
    state = applyFileChanged(state, changed(`d${String(index % 40)}/f${String(index)}.ts`), 1000 + index);
  }
  assert.equal(state.ring.burstOpen?.paths.size, BURST_MEMBER_CAP);
  assert.equal(pendingBurst(state).files, BURST_MEMBER_CAP + 2);
  assert.equal(pendingBurst(state).byOp.modify, BURST_MEMBER_CAP + 2);
  // 40 directories arrive round-robin: 32 are tallied by name and the remaining 8 share the `other` bucket
  const dirCounts = state.ring.burstOpen.dirCounts;
  assert.equal(dirCounts.size, BURST_TOP_DIRS_TRACKED + 1);
  const named = [...dirCounts].reduce((sum, [dir, count]) => (dir === BURST_OTHER_DIR ? sum : sum + count), 0);
  assert.equal(named + (dirCounts.get(BURST_OTHER_DIR) ?? 0), BURST_MEMBER_CAP + 2);
  assert.ok((dirCounts.get(BURST_OTHER_DIR) ?? 0) > 0);
  const now = 1000 + BURST_MEMBER_CAP + 2;
  state = applyFileDelta(state, textDelta("d0/f0.ts"), now, opts);
  assert.equal(state.pending.length, 1);
  const uncounted = BURST_MEMBER_CAP + 1;
  state = applyFileDelta(state, textDelta(`d${String(uncounted % 40)}/f${String(uncounted)}.ts`), now, opts);
  assert.equal(state.pending.length, 2);
});
