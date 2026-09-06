// Path: app/src/lib/stream/stream_ring_apply_test.ts
// Description: Delta reducer rules: create vs extend-newest-of-path, zero-stat skip, seq gap vs restart on deltas and counters, counters

import { test } from "node:test";
import assert from "node:assert/strict";
import { EXPAND_CAP, MERGE_WINDOW_MS } from "./stream_bounds.js";
import { admit } from "./stream_ring.js";
import { applyDeltaCounters, applyFileDelta } from "./stream_ring_apply.js";
import { applyFileChanged } from "./stream_ring_apply_burst.js";
import { initialReduceState } from "./stream_ring_apply_support.js";
import { changed, resetSeq, textDelta } from "./testing/stream_fixtures.js";
import type { StreamFileCard, StreamReduceState, StreamTextBody } from "./stream_types.js";

const opts = { outsideSelection: (path: string): boolean => path.startsWith("out/") };

function pendingFile(state: StreamReduceState, index = 0): StreamFileCard {
  const card = state.pending[index];
  assert.equal(card?.kind, "file");
  return card;
}

/** A file card's text body, or a failed assertion */
function textBodyOf(card: StreamFileCard): StreamTextBody {
  assert.equal(card.body.status, "text");
  return card.body;
}

void test("a text delta becomes one resolved pending card with a parsed body", () => {
  resetSeq();
  const state = applyFileDelta(initialReduceState(), textDelta("out/a.ts"), 1000, opts);
  assert.equal(state.pending.length, 1);
  const card = pendingFile(state);
  assert.equal(card.outsideSelection, true);
  assert.equal(card.edits, 1);
  assert.equal(card.admittedAtMs, 0);
  const body = textBodyOf(card);
  assert.equal(body.segments.length, 1);
  assert.equal(body.segments[0]?.length, 4);
  assert.equal(body.hiddenLines, 0);
  assert.equal(state.ring.lastSeq, 1);
});

void test("a re-edit inside MERGE_WINDOW_MS extends the newest card of that path; past it a new card prints", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts"), 1000, opts);
  state = applyFileDelta(state, textDelta("a.ts"), 1000 + MERGE_WINDOW_MS, opts);
  assert.equal(state.pending.length, 1);
  const card = pendingFile(state);
  assert.equal(card.edits, 2);
  assert.equal(card.updatedAtMs, 1000 + MERGE_WINDOW_MS);
  const body = textBodyOf(card);
  assert.equal(body.segments.length, 2);
  assert.equal(body.stats.added, 4);
  assert.equal(body.stats.removed, 2);

  state = applyFileDelta(state, textDelta("a.ts"), 1000 + MERGE_WINDOW_MS * 2 + 1, opts);
  assert.equal(state.pending.length, 2);
});

void test("interleaved paths each extend their own newest card: a, b, a yields a x2 and b x1", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts"), 1000, opts);
  state = applyFileDelta(state, textDelta("b.ts"), 1100, opts);
  state = applyFileDelta(state, textDelta("a.ts"), 1200, opts);
  assert.equal(state.pending.length, 2);
  assert.equal(pendingFile(state, 0).path, "a.ts");
  assert.equal(pendingFile(state, 0).edits, 2);
  assert.equal(pendingFile(state, 1).path, "b.ts");
  assert.equal(pendingFile(state, 1).edits, 1);
});

void test("extend reaches the newest ring card of the path, keeps static sticky, and caps at EXPAND_CAP", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts", {}, 60, 0), 1000, opts);
  const [first, ...rest] = state.pending;
  assert.ok(first !== undefined);
  state = { ...state, pending: rest, ring: admit(state.ring, { ...first, static: true }) };
  state = applyFileDelta(state, textDelta("b.ts"), 1050, opts);
  state = applyFileDelta(state, textDelta("a.ts", {}, 60, 0), 1100, opts);
  assert.equal(state.pending.length, 1);
  const card = state.ring.cards[0];
  assert.equal(card?.kind, "file");
  assert.equal(card.edits, 2);
  assert.equal(card.static, true);
  const body = textBodyOf(card);
  // Two 61-line segments: the older one falls out whole, the newest survives intact
  assert.equal(body.segments.length, 1);
  assert.equal(body.segments[0]?.length, 61);
  assert.equal(body.beyondCap, 61);
  assert.ok(body.segments.reduce((sum, segment) => sum + segment.length, 0) <= EXPAND_CAP);
});

void test("an exiting or removed card never extends", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts"), 1000, opts);
  const [first, ...rest] = state.pending;
  assert.ok(first !== undefined);
  state = { ...state, pending: rest, ring: admit(state.ring, { ...first, exiting: true }) };
  state = applyFileDelta(state, textDelta("a.ts"), 1100, opts);
  assert.equal(state.pending.length, 1);
  state = applyFileDelta(state, textDelta("a.ts", { op: "remove" }), 1200, opts);
  assert.equal(state.pending.length, 2);
});

void test("a zero-stat modify creates no card but clears the settling line", () => {
  resetSeq();
  let state = applyFileChanged(initialReduceState(), changed("a.ts"), 900);
  assert.deepEqual(state.settling.map((entry) => entry.path), ["a.ts"]);
  state = applyFileDelta(state, textDelta("a.ts", {}, 0, 0), 1000, opts);
  assert.equal(state.pending.length, 0);
  assert.equal(state.settling.length, 0);
  // A zero-stat rename is a MOVED card
  state = applyFileDelta(state, textDelta("b.ts", { op: "rename", fromPath: "a.ts" }, 0, 0), 1000, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(pendingFile(state).fromPath, "a.ts");
});

void test("a seq gap prints a notice; a restart or first delta does not", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts", { seq: 7 }), 1000, opts);
  assert.equal(state.ring.notices.length, 0);
  state = applyFileDelta(state, textDelta("b.ts", { seq: 10 }), 1001, opts);
  assert.equal(state.ring.notices[0]?.text, "2 EDITS NOT SHOWN");
  state = applyFileDelta(state, textDelta("c.ts", { seq: 1 }), 1002, opts);
  assert.equal(state.ring.notices.length, 1);
  state = applyFileDelta(state, textDelta("d.ts", { seq: 2, withheld: 5 }), 1003, opts);
  state = applyFileDelta(state, textDelta("e.ts", { seq: 3, withheld: 4 }), 1004, opts);
  assert.equal(state.ring.notices.at(-1)?.text, "9 EDITS WITHHELD · BURST");
});

void test("a counters event consumes the shared sequence: a gap before it prints, and it advances lastSeq", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts", { seq: 4 }), 1000, opts);
  state = applyDeltaCounters(state, { type: "fileDeltaCounters", repoId: "r", seq: 6, withheld: 0, dropped: 0 }, 1001);
  assert.equal(state.ring.lastSeq, 6);
  assert.equal(state.ring.notices.at(-1)?.text, "1 EDITS NOT SHOWN");
  state = applyFileDelta(state, textDelta("b.ts", { seq: 7 }), 1002, opts);
  assert.equal(state.ring.notices.length, 1);
  assert.equal(state.ring.lastSeq, 7);
  // A counters event is a restart too when its seq is 1
  state = applyDeltaCounters(state, { type: "fileDeltaCounters", repoId: "r", seq: 1, withheld: 0, dropped: 0 }, 1003);
  assert.equal(state.ring.notices.length, 1);
  assert.equal(state.ring.lastSeq, 1);
});

void test("a counters event merges into the same withheld and dropped notices as a delta", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), textDelta("a.ts", { withheld: 3 }), 1000, opts);
  state = applyDeltaCounters(state, { type: "fileDeltaCounters", repoId: "r", seq: 2, withheld: 2, dropped: 0 }, 1500);
  assert.equal(state.ring.notices.length, 1);
  assert.equal(state.ring.notices[0]?.text, "5 EDITS WITHHELD · BURST");
  state = applyDeltaCounters(state, { type: "fileDeltaCounters", repoId: "r", seq: 3, withheld: 0, dropped: 4 }, 1600);
  assert.equal(state.ring.notices.at(-1)?.text, "4 EDITS DROPPED");
  assert.equal(state.ring.notices.at(-1)?.tone, "error");
  assert.equal(state.pending.length, 1);
});
