// Path: app/src/lib/stream/stream_card_body.ts
// Description: Builds and extends card bodies from fileDelta payloads under the EXPAND_CAP line bound

import { parsePatch, type DiffLine } from "../diff/diff_lines.js";
import type { FileDeltaEvent } from "../../shared/protocol.js";
import { EXPAND_CAP, LINE_CAP } from "./stream_bounds.js";
import { selectLines } from "./stream_card_grammar.js";
import type { StreamCardBody, StreamFileCard, StreamTextBody } from "./stream_types.js";

type Segments = readonly (readonly DiffLine[])[];

interface CappedSegments {
  segments: Segments;
  /** Lines the cap cut, so the footer can still count them */
  beyondCap: number;
}

/**
 * At most EXPAND_CAP lines: whole OLDER segments fall out first (newest last), and when the
 * newest segment alone still exceeds the cap it is cut from its END so the head of the change
 * (the first thing the reader looks for) survives.
 */
function capSegments(segments: Segments): CappedSegments {
  let total = segments.reduce((sum, segment) => sum + segment.length, 0);
  let beyondCap = 0;
  let first = 0;
  while (first < segments.length - 1 && total > EXPAND_CAP) {
    const dropped = segments[first]?.length ?? 0;
    total -= dropped;
    beyondCap += dropped;
    first += 1;
  }
  const kept = segments.slice(first);
  const newest = kept[kept.length - 1];
  if (newest === undefined || newest.length <= EXPAND_CAP) return { segments: kept, beyondCap };
  beyondCap += newest.length - EXPAND_CAP;
  return { segments: [...kept.slice(0, -1), newest.slice(0, EXPAND_CAP)], beyondCap };
}

function hiddenFor(segments: Segments, beyondCap: number): number {
  return selectLines(segments, false, LINE_CAP).hiddenLines + beyondCap;
}

export function textBody(payload: Extract<FileDeltaEvent["payload"], { kind: "text" }>): StreamTextBody {
  const parsed = parsePatch(payload.patch, false).lines.filter((line) => line.kind !== "meta");
  const { segments, beyondCap } = capSegments([parsed]);
  return {
    status: "text",
    segments,
    hiddenLines: hiddenFor(segments, beyondCap),
    beyondCap,
    stats: payload.stats,
    baseline: payload.baseline,
    truncated: payload.truncated,
  };
}

export function bodyFor(payload: FileDeltaEvent["payload"]): StreamCardBody {
  switch (payload.kind) {
    case "text":
      return textBody(payload);
    case "image":
      return { status: "image", bytes: payload.bytes, mimeType: payload.mimeType };
    case "opaque":
      return { status: "opaque", bytes: payload.bytes, reason: payload.reason };
    case "gone":
      return { status: "gone" };
  }
}

/**
 * A re-edit inside the merge window: one more segment, summed stats, the newest baseline.
 * `static` is sticky: an extended old card prints only its fresh rows, never its whole arrival.
 */
export function extendCard(card: StreamFileCard, event: FileDeltaEvent, now: number): StreamFileCard {
  if (card.body.status !== "text" || event.payload.kind !== "text") return card;
  const fresh = textBody(event.payload);
  const capped = capSegments([...card.body.segments, ...fresh.segments]);
  const beyondCap = card.body.beyondCap + fresh.beyondCap + capped.beyondCap;
  const stats = {
    added: card.body.stats.added + fresh.stats.added,
    removed: card.body.stats.removed + fresh.stats.removed,
    hunks: card.body.stats.hunks + fresh.stats.hunks,
    newLines: fresh.stats.newLines,
  };
  const body: StreamTextBody = {
    ...fresh,
    segments: capped.segments,
    stats,
    hiddenLines: hiddenFor(capped.segments, beyondCap),
    beyondCap,
    truncated: card.body.truncated || fresh.truncated,
  };
  return { ...card, body, edits: card.edits + 1, updatedAtMs: now };
}
