// Path: app/src/lib/stream/stream_card_grammar.ts
// Description: The one authority for how a Stream card reads: badge, rail, baseline chip, line selection, clock

import type { DiffLine } from "../diff/diff_lines.js";
import { isPreviewImagePath } from "../../hooks/repo_workspace_types.js";
import type { DeltaBaseline, DeltaOp, SourceControlChange } from "../../shared/protocol.js";
import { EXPAND_CAP, IMAGE_CARD_MAX_BYTES, LINE_CAP, LINE_CAP_HANDSET } from "./stream_bounds.js";
import type { StreamFileCard, StreamRingCard } from "./stream_types.js";

export type StreamCardKind =
  | "text-modified"
  | "text-added"
  | "text-deleted"
  | "images"
  | "binary"
  | "burst"
  | "history";

export type StreamRailVariant = "success" | "info" | "error" | "warning" | "accent" | "muted";

export type StreamBaselineLabel = "SINCE LAST" | "VS INDEX" | "NEW" | "GONE" | "MOVED";

/** CHANGE_BADGES key for the head; an opaque body reads as a type change */
export function badgeFor(op: DeltaOp, tracked: boolean | null, opaque = false): SourceControlChange {
  if (opaque) return "typeChanged";
  switch (op) {
    case "add":
      return tracked === false ? "untracked" : "added";
    case "modify":
      return "modified";
    case "remove":
      return "deleted";
    case "rename":
      return "renamed";
  }
}

/** Image deltas always form strips, so a file card is text or opaque */
export function cardKind(card: StreamRingCard): StreamCardKind {
  if (card.kind === "burst") return "burst";
  if (card.kind === "history") return "history";
  if (card.kind === "images") return "images";
  if (card.body.status === "opaque") return "binary";
  if (card.op === "remove" || card.body.status === "gone") return "text-deleted";
  if (card.op === "add") return "text-added";
  return "text-modified";
}

/** Colour of the 3 px spine; the badge letter and chip carry the same fact without colour.
 *  A strip's rail follows its tiles (stream_strip_view.ts); "images" here is the mixed default. */
export function railVariant(kind: StreamCardKind): StreamRailVariant {
  switch (kind) {
    case "text-added":
      return "success";
    case "text-modified":
    case "images":
      return "info";
    case "text-deleted":
      return "error";
    case "binary":
      return "warning";
    case "burst":
      return "accent";
    case "history":
      return "muted";
  }
}

/** The invariant made visible: what the card's content is measured against */
export function baselineLabel(card: StreamFileCard): StreamBaselineLabel | null {
  const { body, op } = card;
  if (body.status === "gone") return "GONE";
  if (body.status !== "text") {
    if (op === "remove") return "GONE";
    if (op === "add") return "NEW";
    if (op === "rename") return "MOVED";
    return null;
  }
  if (op === "rename" && body.stats.added + body.stats.removed === 0) return "MOVED";
  return labelForBaseline(body.baseline);
}

export function labelForBaseline(baseline: DeltaBaseline): StreamBaselineLabel {
  switch (baseline) {
    case "previousSighting":
      return "SINCE LAST";
    case "index":
      return "VS INDEX";
    case "none":
      return "NEW";
  }
}

export function lineCap(handset: boolean): number {
  return handset ? LINE_CAP_HANDSET : LINE_CAP;
}

export interface SelectedLines {
  lines: readonly DiffLine[];
  /** Printable lines not printed, in the retained segments only (the body adds `beyondCap`) */
  hiddenLines: number;
  /** Index in `lines` where the newest segment's rows begin; `lines.length` when none of it shows */
  newestFrom: number;
}

/** The rows a segment can print: `meta` dropped, at most one hunk header */
function printable(lines: readonly DiffLine[]): DiffLine[] {
  let hunkSeen = false;
  return lines.filter((line) => {
    if (line.kind === "meta") return false;
    if (line.kind !== "hunk") return true;
    if (hunkSeen) return false;
    hunkSeen = true;
    return true;
  });
}

/**
 * Collapsed, one segment: the FIRST `cap` printable lines (the head of the change). Collapsed,
 * several segments: the LAST `cap` lines across all of them, so the newest edit is on screen
 * and older lines fill the cap above it. Expanded: every segment oldest first, up to EXPAND_CAP.
 */
export function selectLines(
  segments: readonly (readonly DiffLine[])[],
  expanded: boolean,
  cap: number
): SelectedLines {
  const printableSegments = segments.map(printable);
  const all = printableSegments.flat();
  const newestLength = printableSegments[printableSegments.length - 1]?.length ?? 0;
  let lines: DiffLine[];
  if (expanded) lines = all.slice(0, EXPAND_CAP);
  else if (printableSegments.length <= 1) lines = all.slice(0, cap);
  else lines = all.slice(Math.max(0, all.length - cap));
  // The newest segment's rows always sit at the tail of whatever printed
  const newestFrom = lines.length - Math.min(lines.length, newestLength);
  return { lines, hiddenLines: all.length - lines.length, newestFrom };
}

/** Local wall clock as `14:32:07`, computed once at arrival */
export function formatClock(ms: number): string {
  const date = new Date(ms);
  const pad = (value: number): string => String(value).padStart(2, "0");
  return `${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

/** Only a whitelisted extension with a wire mime type under the size gate ever fetches pixels */
export function previewableImage(path: string, mimeType: string | null, bytes: number): boolean {
  return mimeType !== null && bytes <= IMAGE_CARD_MAX_BYTES && isPreviewImagePath(path);
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${String(bytes)} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
