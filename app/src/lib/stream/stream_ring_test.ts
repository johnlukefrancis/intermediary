// Path: app/src/lib/stream/stream_ring_test.ts
// Description: Ring bounds: admit and evict order, history first, expanded exempt, notice bound, static sweep

import { test } from "node:test";
import assert from "node:assert/strict";
import { MAX_EXPANDED, NOTICE_MAX, RING_SIZE, STATIC_AFTER_MS } from "./stream_bounds.js";
import { EMPTY_RING, admit, expand, markStatic, pushNotice, seedHistory, spliceExited } from "./stream_ring.js";
import { fileCard } from "./testing/stream_fixtures.js";
import type { StreamRing } from "./stream_types.js";

function fill(count: number, ring: StreamRing = EMPTY_RING, firstId = 1): StreamRing {
  let next = ring;
  for (let index = 0; index < count; index += 1) {
    next = admit(next, fileCard(firstId + index, `f${String(firstId + index)}.ts`, 0));
  }
  return next;
}

void test("admit appends; over RING_SIZE the oldest card is flagged exiting, then spliced on the next admit", () => {
  const full = fill(RING_SIZE);
  assert.equal(full.cards.length, RING_SIZE);
  assert.equal(full.cards.some((card) => card.exiting), false);

  const over = admit(full, fileCard(99, "over.ts", 0));
  assert.equal(over.cards.length, RING_SIZE + 1);
  assert.equal(over.cards[0]?.exiting, true);
  assert.equal(over.cards[0].id, 1);

  const next = admit(over, fileCard(100, "next.ts", 0));
  assert.equal(next.cards.length, RING_SIZE + 1);
  assert.equal(next.cards.some((card) => card.id === 1), false);
  assert.equal(next.cards[0]?.id, 2);
  assert.equal(next.cards[0].exiting, true);
  assert.equal(spliceExited(next).cards.length, RING_SIZE);
});

void test("history rows evict before any file card", () => {
  const seeded = seedHistory(EMPTY_RING, [
    { path: "old.ts", fileKind: "code", lastSeenAtIso: "2026-01-01T00:00:00Z" },
    { path: "new.ts", fileKind: "code", lastSeenAtIso: "2026-01-02T00:00:00Z" },
  ], 1);
  assert.deepEqual(seeded.cards.map((card) => card.kind === "history" ? card.path : ""), ["old.ts", "new.ts"]);
  assert.equal(seedHistory(seeded, [{ path: "x", fileKind: "docs", lastSeenAtIso: "2026-01-03T00:00:00Z" }], 9), seeded);

  const ring = fill(RING_SIZE - 1, seeded, 10);
  const oldest = ring.cards.find((card) => card.exiting);
  assert.equal(oldest?.kind, "history");
  assert.equal(oldest.path, "old.ts");
});

void test("expanded cards are eviction-exempt and the oldest expanded collapses past MAX_EXPANDED", () => {
  let ring = fill(RING_SIZE);
  ring = expand(ring, 1);
  const over = admit(ring, fileCard(50, "over.ts", 0));
  assert.equal(over.cards.find((card) => card.exiting)?.id, 2);

  ring = fill(5);
  ring = expand(ring, 1);
  ring = expand(ring, 2);
  ring = expand(ring, 3);
  const expanded = ring.cards.filter((card) => card.kind === "file" && card.expanded).map((card) => card.id);
  assert.equal(expanded.length, MAX_EXPANDED);
  assert.deepEqual(expanded, [2, 3]);
  assert.equal(expand(ring, 3).cards.filter((card) => card.kind === "file" && card.expanded).length, 1);
});

void test("notices are bounded and a fresh notice with the same key accumulates in place", () => {
  const text = (n: number): string => `${String(n)} EDITS NOT SHOWN`;
  let ring = pushNotice(EMPTY_RING, { key: "gap", tone: "warning", count: 3, textFor: text }, 1000, 1);
  ring = pushNotice(ring, { key: "gap", tone: "warning", count: 2, textFor: text }, 1500, 2);
  assert.equal(ring.notices.length, 1);
  assert.equal(ring.notices[0]?.count, 5);
  assert.equal(ring.notices[0].text, "5 EDITS NOT SHOWN");

  ring = pushNotice(ring, { key: "gap", tone: "warning", count: 1, textFor: text }, 4000, 3);
  assert.equal(ring.notices.length, 2);
  for (let index = 0; index < NOTICE_MAX + 2; index += 1) {
    ring = pushNotice(ring, { key: `k${String(index)}`, tone: "accent", count: 1, textFor: () => "x" }, 5000, 10 + index);
  }
  assert.equal(ring.notices.length, NOTICE_MAX);
});

void test("cards turn static once older than STATIC_AFTER_MS; a ring with nothing to age is returned as is", () => {
  const ring = admit(EMPTY_RING, fileCard(1, "a.ts", 1000));
  assert.equal(markStatic(ring, 1000 + STATIC_AFTER_MS - 1), ring);
  const aged = markStatic(ring, 1000 + STATIC_AFTER_MS);
  assert.equal(aged.cards[0]?.kind === "file" ? aged.cards[0].static : false, true);
  assert.equal(markStatic(aged, 5000), aged);
});
