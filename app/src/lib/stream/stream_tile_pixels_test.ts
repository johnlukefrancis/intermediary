// Path: app/src/lib/stream/stream_tile_pixels_test.ts
// Description: Revision-bound pixels: a read is accepted only for the announced bytes and mtime; the decode gate; the thumbnail fit; the mid-read rewrite refusal

import { test } from "node:test";
import assert from "node:assert/strict";
import { AgentResponseError } from "../agent/error_codes.js";
import { MAX_TILE_PIXELS, STRIP_THUMB_MAX_PX } from "./stream_bounds.js";
import { exceedsTilePixels, readRefusedAsChanged, sameRevision, thumbnailSize } from "./stream_tile_pixels.js";

void test("pixels are accepted only when both the byte count and the mtime match the tile's revision", () => {
  const tile = { bytes: 4096, mtimeMs: 1_700_000_000_000 };
  assert.equal(sameRevision(tile, { bytes: 4096, mtimeMs: 1_700_000_000_000 }), true);
  assert.equal(sameRevision(tile, { bytes: 4096, mtimeMs: 1_700_000_000_001 }), false);
  assert.equal(sameRevision(tile, { bytes: 4097, mtimeMs: 1_700_000_000_000 }), false);
  assert.equal(sameRevision(tile, { bytes: 0, mtimeMs: 0 }), false);
});

void test("the decode gate trips strictly past MAX_TILE_PIXELS: an 8K frame decodes, a bomb does not", () => {
  assert.equal(exceedsTilePixels(0, 0), false);
  assert.equal(exceedsTilePixels(7680, 4320), false);
  assert.equal(exceedsTilePixels(MAX_TILE_PIXELS, 1), false);
  assert.equal(exceedsTilePixels(MAX_TILE_PIXELS + 1, 1), true);
  assert.equal(exceedsTilePixels(10000, 8000), true);
});

void test("a source within STRIP_THUMB_MAX_PX is kept as-is; a larger one scales to that long edge, aspect kept, never below a pixel", () => {
  assert.equal(thumbnailSize(16, 16), null);
  assert.equal(thumbnailSize(STRIP_THUMB_MAX_PX, 10), null);
  assert.deepEqual(thumbnailSize(STRIP_THUMB_MAX_PX * 2, STRIP_THUMB_MAX_PX), { width: STRIP_THUMB_MAX_PX, height: STRIP_THUMB_MAX_PX / 2 });
  assert.deepEqual(thumbnailSize(3840, 2160), { width: 1024, height: 576 });
  assert.deepEqual(thumbnailSize(1080, 1920), { width: 576, height: 1024 });
  assert.deepEqual(thumbnailSize(100000, 1), { width: STRIP_THUMB_MAX_PX, height: 1 });
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
