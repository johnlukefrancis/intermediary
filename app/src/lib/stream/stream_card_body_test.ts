// Path: app/src/lib/stream/stream_card_body_test.ts
// Description: Body cap rules: a single oversized patch keeps its head; whole older segments fall out first

import { test } from "node:test";
import assert from "node:assert/strict";
import { EXPAND_CAP, LINE_CAP } from "./stream_bounds.js";
import { extendCard, textBody } from "./stream_card_body.js";
import { textDelta } from "./testing/stream_fixtures.js";
import type { StreamFileCard } from "./stream_types.js";

function addedPatch(rows: number): string {
  const lines = ["@@ -0,0 +1 @@"];
  for (let index = 1; index < rows; index += 1) lines.push(`+line${String(index)}`);
  return `${lines.join("\n")}\n`;
}

const ROWS = 215;

void test("a fresh oversized patch keeps its head: +line1 prints first and the cut counts as hidden", () => {
  const body = textBody({ kind: "text", patch: addedPatch(ROWS), stats: { added: ROWS - 1, removed: 0, hunks: 1, newLines: ROWS - 1 }, baseline: "none", truncated: false });
  const segment = body.segments[0];
  assert.ok(segment !== undefined);
  assert.equal(body.segments.length, 1);
  assert.equal(segment.length, EXPAND_CAP);
  assert.equal(segment[0]?.kind, "hunk");
  assert.equal(segment[1]?.text, "+line1");
  assert.equal(segment[EXPAND_CAP - 1]?.text, `+line${String(EXPAND_CAP - 1)}`);
  assert.equal(body.beyondCap, ROWS - EXPAND_CAP);
  assert.equal(body.hiddenLines, ROWS - LINE_CAP);
});

void test("extending drops whole older segments before it ever cuts the newest one", () => {
  const first = textBody({ kind: "text", patch: addedPatch(30), stats: { added: 29, removed: 0, hunks: 1, newLines: 29 }, baseline: "none", truncated: false });
  const card: StreamFileCard = {
    kind: "file", id: 1, path: "a.ts", fromPath: null, fileKind: "code", op: "add", tracked: false,
    outsideSelection: false, clock: "00:00:00", arrivedAtMs: 0, updatedAtMs: 0, admittedAtMs: 0,
    edits: 1, expanded: false, exiting: false, static: true, body: first,
  };
  const second = extendCard(card, textDelta("a.ts", {}, 30, 0), 100);
  const third = extendCard(second, textDelta("a.ts", {}, 30, 0), 200);
  assert.equal(third.body.status, "text");
  assert.equal(third.edits, 3);
  assert.equal(third.static, true);
  // 30 + 31 + 31 = 92 > 80: the first segment leaves whole, the two newest survive intact
  assert.deepEqual(third.body.segments.map((segment) => segment.length), [31, 31]);
  assert.equal(third.body.beyondCap, 30);
  assert.equal(third.body.segments[1]?.[1]?.text, "+new 0");
});
