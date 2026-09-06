// Path: app/src/lib/stream/stream_image_strip.ts
// Description: Pure reducer folding image deltas into the tail strip (or replacing a tile in the newest one), else opening a new strip

import type { DeltaOp, FileDeltaEvent } from "../../shared/protocol.js";
import { IMAGE_STRIP_MAX } from "./stream_bounds.js";
import { formatClock } from "./stream_card_grammar.js";
import { isTailCard, newestStrip, replaceCard, takeId, type ApplyOptions } from "./stream_ring_apply_support.js";
import type { StreamImageStripCard, StreamStripTile, StreamStripTileBody } from "./stream_strip_types.js";
import type { StreamReduceState } from "./stream_types.js";

/** Whatever the wire said about an image path; every payload kind becomes a tile, never a file card */
export type ImagePayload = FileDeltaEvent["payload"];

/** Net op of one path inside one strip: a delete always wins; added-then-edited is still a new file */
export function mergeOp(previous: DeltaOp, next: DeltaOp): DeltaOp {
  if (next === "remove") return "remove";
  if (previous === "add") return "add";
  return next;
}

/** An opaque (or, from an older agent, text) payload is a NO PREVIEW tile: no mime means no fetch */
export function stripTileBody(payload: ImagePayload): StreamStripTileBody {
  switch (payload.kind) {
    case "image":
      return { status: "image", bytes: payload.bytes, mimeType: payload.mimeType };
    case "gone":
      return { status: "gone" };
    case "opaque":
      return { status: "image", bytes: payload.bytes, mimeType: null };
    case "text":
      return { status: "image", bytes: 0, mimeType: null };
  }
}

/**
 * A live strip always takes a repeat path (replacing a tile never grows the strip, wherever the
 * strip sits), and takes a NEW path only while it has room AND is the tail entry of the feed —
 * whatever its age: only a card printed after it closes it to new images, so the row keeps growing
 * left to right while images are the newest thing and the reading order above the tail never moves.
 */
export function stripAccepts(card: StreamImageStripCard, path: string, atTail: boolean): boolean {
  if (card.exiting) return false;
  if (card.tiles.some((tile) => tile.path === path)) return true;
  return atTail && card.tiles.length < IMAGE_STRIP_MAX;
}

function newTile(event: FileDeltaEvent, payload: ImagePayload, now: number, opts: ApplyOptions): StreamStripTile {
  return {
    path: event.path,
    op: event.op,
    tracked: event.tracked ?? null,
    outsideSelection: opts.outsideSelection(event.path),
    clock: formatClock(now),
    arrivedAtMs: now,
    updatedAtMs: now,
    edits: 1,
    body: stripTileBody(payload),
  };
}

/**
 * The tile with the same path is replaced IN PLACE (its reading position survives); otherwise
 * a tile is appended at the tail. Admission facts (`admittedAtMs`, `static`) are untouched, so
 * an extended strip stays static and only the fresh tile animates.
 */
export function foldTile(
  card: StreamImageStripCard,
  event: FileDeltaEvent,
  payload: ImagePayload,
  now: number,
  opts: ApplyOptions
): StreamImageStripCard {
  const index = card.tiles.findIndex((tile) => tile.path === event.path);
  const existing = card.tiles[index];
  if (existing === undefined) {
    return { ...card, tiles: [...card.tiles, newTile(event, payload, now, opts)], updatedAtMs: now };
  }
  const replaced: StreamStripTile = {
    ...existing,
    op: mergeOp(existing.op, event.op),
    tracked: event.tracked ?? existing.tracked,
    outsideSelection: opts.outsideSelection(event.path),
    updatedAtMs: now,
    edits: existing.edits + 1,
    body: stripTileBody(payload),
  };
  const tiles = card.tiles.map((tile, at) => (at === index ? replaced : tile));
  return { ...card, tiles, updatedAtMs: now };
}

export function openStrip(
  id: number,
  event: FileDeltaEvent,
  payload: ImagePayload,
  now: number,
  opts: ApplyOptions
): StreamImageStripCard {
  return {
    kind: "images",
    id,
    tiles: [newTile(event, payload, now, opts)],
    arrivedAtMs: now,
    updatedAtMs: now,
    admittedAtMs: 0,
    expanded: false,
    exiting: false,
    static: false,
  };
}

/** Called after the burst check: an open burst has already swallowed the path by the time we get here */
export function applyImageDelta(
  state: StreamReduceState,
  event: FileDeltaEvent,
  payload: ImagePayload,
  now: number,
  opts: ApplyOptions
): StreamReduceState {
  const target = newestStrip(state);
  if (target !== null && stripAccepts(target.card, event.path, isTailCard(state, target.card.id))) {
    return replaceCard(state, target.where, foldTile(target.card, event, payload, now, opts));
  }
  const [id, next] = takeId(state);
  return { ...next, pending: [...next.pending, openStrip(id, event, payload, now, opts)] };
}
