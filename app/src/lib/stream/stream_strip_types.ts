// Path: app/src/lib/stream/stream_strip_types.ts
// Description: Ring member contracts for an image strip: the tiles one card holds and the body each tile carries

import type { DeltaOp } from "../../shared/protocol.js";

/** What the wire said about the image; pixels are fetched by the panel under the size and mime gate */
export type StreamStripTileBody =
  | { status: "image"; bytes: number; mimeType: string | null }
  | { status: "gone" };

export interface StreamStripTile {
  /** Repo-relative path; the tile's identity inside the strip */
  path: string;
  /** Net op since this strip opened: a remove always wins, an add survives a later modify */
  op: DeltaOp;
  /** Best-effort from the wire; null when the agent did not say */
  tracked: boolean | null;
  outsideSelection: boolean;
  /** Wall clock of arrival, formatted once */
  clock: string;
  arrivedAtMs: number;
  /** Newest delta folded into this tile; drives data-fresh and the pixel refetch */
  updatedAtMs: number;
  /** Deltas folded into this tile; 1 for a single edit */
  edits: number;
  body: StreamStripTileBody;
}

/**
 * One card per burst of image edits. Its height is a function of how many tiles it holds and
 * the panel's width only — never of whether a tile's pixels are loaded, released, or missing.
 */
export interface StreamImageStripCard {
  kind: "images";
  id: number;
  /** Oldest first: reading order, left to right, wrapping into rows */
  tiles: readonly StreamStripTile[];
  arrivedAtMs: number;
  /** Merge and memo key: the newest delta folded into any tile */
  updatedAtMs: number;
  /** Stamped by the store when the card enters the ring (0 while pending); the static clock starts here */
  admittedAtMs: number;
  /** Expanded = modified tiles with a retained BEFORE show BEFORE/AFTER pairs */
  expanded: boolean;
  exiting: boolean;
  /** Older than STATIC_AFTER_MS since admission: the arrival never replays, even when extended */
  static: boolean;
}
