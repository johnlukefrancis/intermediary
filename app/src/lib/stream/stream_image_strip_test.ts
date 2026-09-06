// Path: app/src/lib/stream/stream_image_strip_test.ts
// Description: Strip reducer rules: open, append in order whatever the gap, replace in place, remove wins, new strip on max or a card printed after, ring extend, burst and collapse

import { test } from "node:test";
import assert from "node:assert/strict";
import { BURST_THRESHOLD, IMAGE_STRIP_MAX } from "./stream_bounds.js";
import { newBurstCard } from "./stream_burst_card.js";
import { mergeOp } from "./stream_image_strip.js";
import { absorbIntoBurst, admit, openBurst } from "./stream_ring.js";
import { applyFileDelta } from "./stream_ring_apply.js";
import { collapsePending } from "./stream_ring_apply_burst.js";
import { initialReduceState, takeId } from "./stream_ring_apply_support.js";
import { IMAGE_MTIME_MS, fileCard, imageDelta, resetSeq, textDelta } from "./testing/stream_fixtures.js";
import type { StreamImageStripCard, StreamReduceState } from "./stream_types.js";

const opts = { outsideSelection: (): boolean => false };

function pendingStrip(state: StreamReduceState, index = 0): StreamImageStripCard {
  const card = state.pending[index];
  assert.equal(card?.kind, "images");
  return card;
}

function withImages(state: StreamReduceState, count: number, firstMs: number): StreamReduceState {
  let next = state;
  for (let index = 0; index < count; index += 1) {
    next = applyFileDelta(next, imageDelta(`assets/i${String(index)}.png`), firstMs + index * 100, opts);
  }
  return next;
}

void test("the first image delta opens a strip with one tile and never a file card", () => {
  resetSeq();
  const state = applyFileDelta(initialReduceState(), imageDelta("assets/a.png"), 1000, opts);
  assert.equal(state.pending.length, 1);
  const strip = pendingStrip(state);
  assert.equal(strip.tiles.length, 1);
  const tile = strip.tiles[0];
  assert.ok(tile !== undefined);
  assert.equal(tile.path, "assets/a.png");
  assert.equal(tile.op, "add");
  assert.deepEqual(tile.body, { status: "image", bytes: 1024, mimeType: "image/png", mtimeMs: IMAGE_MTIME_MS });
  assert.equal(strip.admittedAtMs, 0);
  assert.equal(strip.expanded, false);
});

void test("consecutive images append left to right in arrival order", () => {
  resetSeq();
  const state = withImages(initialReduceState(), 4, 1000);
  assert.equal(state.pending.length, 1);
  const strip = pendingStrip(state);
  assert.deepEqual(strip.tiles.map((tile) => tile.path), ["assets/i0.png", "assets/i1.png", "assets/i2.png", "assets/i3.png"]);
  assert.equal(strip.updatedAtMs, 1300);
});

void test("a repeat path replaces its tile in place: same index, edits x2, net op add, new bytes", () => {
  resetSeq();
  let state = withImages(initialReduceState(), 2, 1000);
  state = applyFileDelta(state, imageDelta("assets/i0.png", { op: "modify" }, 2048), 1200, opts);
  const strip = pendingStrip(state);
  assert.equal(state.pending.length, 1);
  assert.equal(strip.tiles.length, 2);
  const replaced = strip.tiles[0];
  assert.ok(replaced !== undefined);
  assert.equal(replaced.path, "assets/i0.png");
  assert.equal(replaced.edits, 2);
  assert.equal(replaced.op, "add");
  assert.equal(replaced.body.status === "image" ? replaced.body.bytes : 0, 2048);
  assert.equal(replaced.updatedAtMs, 1200);
  assert.equal(strip.tiles[1]?.edits, 1);
  assert.equal(strip.updatedAtMs, 1200);
});

void test("a remove wins the merged op and the tile keeps its slot with a gone body", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  state = applyFileDelta(state, imageDelta("a.png", { op: "remove", payload: { kind: "gone" } }), 1100, opts);
  const strip = pendingStrip(state);
  assert.equal(strip.tiles.length, 1);
  const tile = strip.tiles[0];
  assert.ok(tile !== undefined);
  assert.equal(tile.op, "remove");
  assert.deepEqual(tile.body, { status: "gone" });
  assert.equal(mergeOp("modify", "modify"), "modify");
  assert.equal(mergeOp("remove", "add"), "add");
});

void test("the strip stops at IMAGE_STRIP_MAX and the next image opens a new one", () => {
  resetSeq();
  const state = withImages(initialReduceState(), IMAGE_STRIP_MAX + 1, 1000);
  assert.equal(state.pending.length, 2);
  assert.equal(pendingStrip(state, 0).tiles.length, IMAGE_STRIP_MAX);
  assert.equal(pendingStrip(state, 1).tiles.length, 1);
});

void test("a repeat path still replaces inside a full strip", () => {
  resetSeq();
  let state = withImages(initialReduceState(), IMAGE_STRIP_MAX, 1000);
  state = applyFileDelta(state, imageDelta("assets/i0.png", { op: "modify" }), 1000 + IMAGE_STRIP_MAX * 100, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(pendingStrip(state).tiles.length, IMAGE_STRIP_MAX);
  assert.equal(pendingStrip(state).tiles[0]?.edits, 2);
});

void test("there is no quiet window: an image a minute after the last one still joins the tail strip", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  state = applyFileDelta(state, imageDelta("b.png"), 1000 + 12_000, opts);
  assert.equal(state.pending.length, 1);
  state = applyFileDelta(state, imageDelta("c.png"), 1000 + 60_000, opts);
  assert.equal(state.pending.length, 1);
  assert.deepEqual(pendingStrip(state).tiles.map((tile) => tile.path), ["a.png", "b.png", "c.png"]);
  assert.equal(pendingStrip(state).updatedAtMs, 61_000);
});

void test("a text card in between opens a new strip however close the images are in time", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  state = applyFileDelta(state, textDelta("src/a.ts"), 1010, opts);
  state = applyFileDelta(state, imageDelta("b.png"), 1020, opts);
  assert.deepEqual(state.pending.map((card) => card.kind), ["images", "file", "images"]);
  assert.equal(pendingStrip(state, 0).tiles.length, 1);
  assert.equal(pendingStrip(state, 2).tiles[0]?.path, "b.png");
});

void test("an exiting strip never accepts; a strip already in the ring extends in place and stays static", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  const [strip, ...rest] = state.pending;
  assert.ok(strip !== undefined);
  state = { ...state, pending: rest, ring: admit(state.ring, { ...strip, exiting: true }) };
  state = applyFileDelta(state, imageDelta("b.png"), 1100, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(pendingStrip(state).tiles.length, 1);

  state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  const [pending, ...others] = state.pending;
  assert.ok(pending !== undefined);
  state = { ...state, pending: others, ring: admit(state.ring, { ...pending, admittedAtMs: 1050, static: true }) };
  state = applyFileDelta(state, imageDelta("b.png"), 1200, opts);
  assert.equal(state.pending.length, 0);
  const ringCard = state.ring.cards[0];
  assert.equal(ringCard?.kind, "images");
  assert.equal(ringCard.id, pending.id);
  assert.equal(ringCard.tiles.length, 2);
  assert.equal(ringCard.static, true);
  assert.equal(ringCard.admittedAtMs, 1050);
});

void test("a text card printed after a strip closes it to new paths: the next image opens a new strip at the tail", () => {
  resetSeq();
  let state = withImages(initialReduceState(), 2, 1000);
  state = applyFileDelta(state, textDelta("src/a.ts"), 1100, opts);
  state = applyFileDelta(state, textDelta("src/b.ts"), 1200, opts);
  state = applyFileDelta(state, imageDelta("assets/i9.png"), 1300, opts);
  assert.deepEqual(state.pending.map((card) => card.kind), ["images", "file", "file", "images"]);
  assert.equal(pendingStrip(state, 0).tiles.length, 2);
  assert.equal(pendingStrip(state, 3).tiles.length, 1);
  assert.equal(pendingStrip(state, 3).tiles[0]?.path, "assets/i9.png");
});

void test("a path already in the older strip still replaces in place behind later text cards", () => {
  resetSeq();
  let state = withImages(initialReduceState(), 2, 1000);
  state = applyFileDelta(state, textDelta("src/a.ts"), 1100, opts);
  state = applyFileDelta(state, textDelta("src/b.ts"), 1200, opts);
  state = applyFileDelta(state, imageDelta("assets/i0.png", { op: "modify" }, 4096), 1300, opts);
  assert.deepEqual(state.pending.map((card) => card.kind), ["images", "file", "file"]);
  const strip = pendingStrip(state, 0);
  assert.equal(strip.tiles.length, 2);
  const replaced = strip.tiles[0];
  assert.ok(replaced !== undefined);
  assert.equal(replaced.edits, 2);
  assert.equal(replaced.body.status === "image" ? replaced.body.bytes : 0, 4096);
  assert.equal(strip.updatedAtMs, 1300);
});

void test("a strip at the ring's tail takes a new path, but not once a text card sits after it in the ring", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  const [strip, ...rest] = state.pending;
  assert.ok(strip !== undefined);
  state = { ...state, pending: rest, ring: admit(state.ring, { ...strip, admittedAtMs: 1050 }) };
  state = applyFileDelta(state, imageDelta("b.png"), 1100, opts);
  assert.equal(state.pending.length, 0);
  const tailStrip = state.ring.cards[0];
  assert.equal(tailStrip?.kind, "images");
  assert.equal(tailStrip.tiles.length, 2);

  state = { ...state, ring: admit(state.ring, fileCard(99, "src/a.ts", 1150)) };
  state = applyFileDelta(state, imageDelta("c.png"), 1200, opts);
  assert.equal(state.pending.length, 1);
  assert.equal(pendingStrip(state).tiles[0]?.path, "c.png");
  const older = state.ring.cards[0];
  assert.equal(older?.kind, "images");
  assert.equal(older.tiles.length, 2);
});

void test("a strip in the ring, a text card waiting in pending, then an image: the image opens a new pending strip", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  const [strip, ...rest] = state.pending;
  assert.ok(strip !== undefined);
  state = { ...state, pending: rest, ring: admit(state.ring, { ...strip, admittedAtMs: 1050 }) };
  state = applyFileDelta(state, textDelta("src/a.ts"), 1100, opts);
  state = applyFileDelta(state, imageDelta("b.png"), 1200, opts);
  assert.deepEqual(state.pending.map((card) => card.kind), ["file", "images"]);
  assert.equal(pendingStrip(state, 1).tiles[0]?.path, "b.png");
  const ringStrip = state.ring.cards[0];
  assert.equal(ringStrip?.kind, "images");
  assert.equal(ringStrip.tiles.length, 1);
  assert.equal(ringStrip.updatedAtMs, 1000);
});

void test("an opaque payload for an image path is a NO PREVIEW tile in the strip, never a file card", () => {
  resetSeq();
  let state = applyFileDelta(initialReduceState(), imageDelta("a.png"), 1000, opts);
  state = applyFileDelta(state, imageDelta("b.png", { payload: { kind: "opaque", bytes: 0, reason: "unreadable" } }), 1100, opts);
  assert.equal(state.pending.length, 1);
  assert.deepEqual(pendingStrip(state).tiles[1]?.body, { status: "image", bytes: 0, mimeType: null, mtimeMs: 0 });
});

void test("an open burst still swallows image paths: resolved bumps and no strip is created", () => {
  resetSeq();
  const [id, allocated] = takeId(initialReduceState());
  const ring = absorbIntoBurst(openBurst(allocated.ring, id, 2000), "a.png", 2000).ring;
  let state: StreamReduceState = { ...allocated, pending: [newBurstCard(id, 1000)], ring };
  state = applyFileDelta(state, imageDelta("a.png"), 1100, opts);
  assert.equal(state.pending.length, 1);
  const burst = state.pending[0];
  assert.equal(burst?.kind, "burst");
  assert.equal(burst.resolved, 1);
});

void test("the backlog collapse absorbs a pending strip, one resolved path per tile", () => {
  resetSeq();
  let state = withImages(initialReduceState(), 3, 1000);
  for (let index = 0; index < BURST_THRESHOLD - 1; index += 1) {
    state = applyFileDelta(state, textDelta(`src/f${String(index)}.ts`), 1400 + index, opts);
  }
  assert.equal(state.pending.length, BURST_THRESHOLD);
  state = collapsePending(state, 2000);
  assert.equal(state.pending.length, 1);
  const burst = state.pending[0];
  assert.equal(burst?.kind, "burst");
  assert.equal(burst.resolved, BURST_THRESHOLD - 1 + 3);
  assert.equal(burst.files, BURST_THRESHOLD - 1 + 3);
  assert.equal(burst.byKind.image, 3);
  assert.equal(state.ring.burstOpen?.paths.has("assets/i2.png"), true);
});
