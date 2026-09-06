// Path: app/src/lib/stream/stream_tile_pixels_test.ts
// Description: Revision-bound pixels: a read is accepted only for the announced bytes and mtime; the decoded-size gate; the mid-read rewrite refusal

import { test } from "node:test";
import assert from "node:assert/strict";
import { AgentResponseError } from "../agent/error_codes.js";
import { MAX_TILE_PIXELS } from "./stream_bounds.js";
import { exceedsTilePixels, readRefusedAsChanged, sameRevision } from "./stream_tile_pixels.js";

void test("pixels are accepted only when both the byte count and the mtime match the tile's revision", () => {
  const tile = { bytes: 4096, mtimeMs: 1_700_000_000_000 };
  assert.equal(sameRevision(tile, { bytes: 4096, mtimeMs: 1_700_000_000_000 }), true);
  assert.equal(sameRevision(tile, { bytes: 4096, mtimeMs: 1_700_000_000_001 }), false);
  assert.equal(sameRevision(tile, { bytes: 4097, mtimeMs: 1_700_000_000_000 }), false);
  assert.equal(sameRevision(tile, { bytes: 0, mtimeMs: 0 }), false);
});

void test("the decoded-size gate trips strictly past MAX_TILE_PIXELS", () => {
  assert.equal(exceedsTilePixels(0, 0), false);
  assert.equal(exceedsTilePixels(4000, 6000), false);
  assert.equal(exceedsTilePixels(MAX_TILE_PIXELS, 1), false);
  assert.equal(exceedsTilePixels(MAX_TILE_PIXELS + 1, 1), true);
  assert.equal(exceedsTilePixels(6000, 5000), true);
});

void test("only an UNSUPPORTED_IMAGE_FILE refusal naming a mid-read change reads as IMAGE CHANGED", () => {
  const changed = new AgentResponseError("UNSUPPORTED_IMAGE_FILE", "Image changed while it was being read", null);
  assert.equal(readRefusedAsChanged(changed), true);
  // The agent may capitalise the sentence differently; the fact is the same
  assert.equal(readRefusedAsChanged(new AgentResponseError("UNSUPPORTED_IMAGE_FILE", "Changed while reading", null)), true);
  // Every other refusal stays a failure: the slot must not claim the file moved when it did not
  assert.equal(readRefusedAsChanged(new AgentResponseError("UNSUPPORTED_IMAGE_FILE", "Image file is too large for the preview", null)), false);
  assert.equal(readRefusedAsChanged(new AgentResponseError("FILE_NOT_FOUND", "Image changed while it was being read", null)), false);
  assert.equal(readRefusedAsChanged(new Error("Image changed while it was being read")), false);
  assert.equal(readRefusedAsChanged(null), false);
});
