// Path: app/src/lib/stream/stream_tile_targets_test.ts
// Description: Tile retention arithmetic: repo-scoped keys, newest MAX_IMAGE_TILES across strips, the byte budget, unfetchable tiles, BEFORE keys

import { test } from "node:test";
import assert from "node:assert/strict";
import { IMAGE_CARD_MAX_BYTES, IMAGE_TILE_BYTES_BUDGET, MAX_IMAGE_TILES } from "./stream_bounds.js";
import { beforeKeys, collectTileTargets, retainedKeys, tileKey } from "./stream_tile_targets.js";
import type { StreamImageStripCard, StreamStripTile } from "./stream_types.js";

interface TileSpec {
  path: string;
  bytes?: number;
  mimeType?: string | null;
  gone?: boolean;
}

function strip(id: number, specs: readonly TileSpec[], firstMs = 1000): StreamImageStripCard {
  const tiles: StreamStripTile[] = specs.map((spec, index) => ({
    path: spec.path,
    op: spec.gone === true ? "remove" : "add",
    tracked: null,
    outsideSelection: false,
    clock: "00:00:00",
    arrivedAtMs: firstMs + index,
    updatedAtMs: firstMs + index,
    edits: 1,
    body: spec.gone === true ? { status: "gone" } : { status: "image", bytes: spec.bytes ?? 1, mimeType: spec.mimeType === undefined ? "image/png" : spec.mimeType },
  }));
  return { kind: "images", id, tiles, arrivedAtMs: firstMs, updatedAtMs: firstMs, admittedAtMs: firstMs, expanded: false, exiting: false, static: true };
}

function paths(prefix: string, count: number): TileSpec[] {
  return Array.from({ length: count }, (_, index) => ({ path: `${prefix}${String(index)}.png` }));
}

void test("retention keeps the newest MAX_IMAGE_TILES fetchable tiles across strips; the rest keep their slot", () => {
  const cards = [strip(1, paths("old/", MAX_IMAGE_TILES), 1000), strip(2, paths("new/", 4), 2000)];
  const targets = collectTileTargets("r", cards);
  assert.equal(targets.length, MAX_IMAGE_TILES + 4);
  const kept = retainedKeys(targets);
  assert.equal(kept.size, MAX_IMAGE_TILES);
  assert.equal(kept.has(tileKey("r", 2, "new/3.png")), true);
  assert.equal(kept.has(tileKey("r", 1, "old/3.png")), false);
  assert.equal(kept.has(tileKey("r", 1, "old/4.png")), true);
});

void test("unfetchable tiles never take a retention slot: no mime, over the gate, or deleted", () => {
  const cards = [
    strip(1, [{ path: "a.heic", mimeType: null }, { path: "big.png", bytes: IMAGE_CARD_MAX_BYTES + 1 }, { path: "gone.png", gone: true }, { path: "ok.png" }]),
  ];
  const targets = collectTileTargets("r", cards);
  assert.deepEqual(targets.map((target) => target.fetchable), [false, false, false, true]);
  assert.deepEqual([...retainedKeys(targets)], [tileKey("r", 1, "ok.png")]);
});

void test("the byte budget releases before the count does", () => {
  const specs = Array.from({ length: 8 }, (_, index) => ({ path: `p${String(index)}.png`, bytes: IMAGE_CARD_MAX_BYTES }));
  const kept = retainedKeys(collectTileTargets("r", [strip(1, specs)]));
  const fits = Math.floor(IMAGE_TILE_BYTES_BUDGET / IMAGE_CARD_MAX_BYTES);
  assert.ok(fits < MAX_IMAGE_TILES);
  assert.equal(kept.size, fits);
  assert.equal(kept.has(tileKey("r", 1, "p7.png")), true);
  assert.equal(kept.has(tileKey("r", 1, "p0.png")), false);
});

void test("a before key resolves to the previous tile of the same path, across strips and for a deleted tile", () => {
  const cards = [strip(1, [{ path: "a.png" }, { path: "b.png" }], 1000), strip(2, [{ path: "a.png" }, { path: "b.png", gone: true }], 2000)];
  const targets = collectTileTargets("r", cards);
  assert.deepEqual(beforeKeys(targets), [null, null, tileKey("r", 1, "a.png"), tileKey("r", 1, "b.png")]);
});

void test("a tile key names its repo: the same strip id and path in another repo never share a pixel record", () => {
  assert.equal(tileKey("a", 1, "x.png"), "a:1:x.png");
  assert.notEqual(tileKey("a", 1, "x.png"), tileKey("b", 1, "x.png"));
  const targets = collectTileTargets("b", [strip(1, [{ path: "x.png" }])]);
  assert.deepEqual(targets.map((target) => target.key), [tileKey("b", 1, "x.png")]);
});
