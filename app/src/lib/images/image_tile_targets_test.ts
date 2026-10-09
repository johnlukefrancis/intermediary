// Path: app/src/lib/images/image_tile_targets_test.ts
// Description: Cross-repository/root BEFORE isolation and one thumbnail budget across all source identities

import { test } from "node:test";
import assert from "node:assert/strict";
import { MAX_IMAGE_TILES } from "./image_tile_bounds.js";
import { beforeKeys, retainedKeys, type TileTarget } from "./image_tile_targets.js";

function target(key: string, repoId: string, root: string): TileTarget {
  const path = "shared.png";
  return { key, sourceKey: JSON.stringify([repoId, root, path]), repoId, path, bytes: 10, mtimeMs: 20, fetchable: true, stamp: 30 };
}

void test("BEFORE pairs only identical repository/root/path sources, and retention shares one newest-tile budget", () => {
  const targets = [
    target("a-old", "a", "/first"),
    target("b-old", "b", "/first"),
    target("a-other-root", "a", "/second"),
    target("a-new", "a", "/first"),
    target("b-new", "b", "/first"),
  ];
  assert.deepEqual(beforeKeys(targets), [null, null, null, "a-old", "b-old"]);
  const newest = Array.from({ length: MAX_IMAGE_TILES }, (_, index) => target(`new-${String(index)}`, index % 2 === 0 ? "a" : "b", "/first"));
  assert.deepEqual([...retainedKeys([...targets, ...newest])], newest.map((entry) => entry.key).reverse());
});
