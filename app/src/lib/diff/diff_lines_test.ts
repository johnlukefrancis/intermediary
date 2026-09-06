// Path: app/src/lib/diff/diff_lines_test.ts
// Description: Golden line model for parsePatch over a unified patch and a combined conflict patch

import { test } from "node:test";
import assert from "node:assert/strict";
import { parsePatch, type DiffLine } from "./diff_lines.js";

/** The load-bearing part of the model: kind plus both gutters, in order */
function model(lines: readonly DiffLine[]): string[] {
  return lines.map((line) => `${line.kind} ${String(line.oldNo)} ${String(line.newNo)}`);
}

const UNIFIED_PATCH = [
  "diff --git a/app.ts b/app.ts",
  "index 1111111..2222222 100644",
  "--- a/app.ts",
  "+++ b/app.ts",
  "@@ -3,4 +3,5 @@ header text",
  " const a = 1;",
  "-const b = 2;",
  "+const b = 3;",
  "+const c = 4;",
  " const d = 5;",
  "",
].join("\n");

const COMBINED_PATCH = [
  "diff --cc merged.txt",
  "index 1111111,2222222..0000000",
  "--- a/merged.txt",
  "+++ b/merged.txt",
  "@@@ -1,3 -1,3 +1,7 @@@",
  "  common line",
  "++<<<<<<< HEAD",
  " +ours line",
  "++=======",
  "+ theirs line",
  "++>>>>>>> branch",
  "  tail line",
  "",
].join("\n");

void test("unified patch: headers are meta, hunk bodies carry both gutters", () => {
  const parsed = parsePatch(UNIFIED_PATCH, false);
  assert.deepEqual(model(parsed.lines), [
    "meta null null",
    "meta null null",
    "meta null null",
    "meta null null",
    "hunk null null",
    "context 3 3",
    "del 4 null",
    "add null 4",
    "add null 5",
    "context 5 6",
  ]);
  assert.equal(parsed.conflictBlocks, 0);
});

void test("combined patch: two prefix columns, conflict markers counted once per block", () => {
  const parsed = parsePatch(COMBINED_PATCH, true);
  assert.deepEqual(model(parsed.lines), [
    "meta null null",
    "meta null null",
    "meta null null",
    "meta null null",
    "hunk null null",
    "context 1 1",
    "marker null 2",
    "add 2 3",
    "marker null 4",
    "add null 5",
    "marker null 6",
    "context 3 7",
  ]);
  assert.equal(parsed.conflictBlocks, 1);
});

void test("CRLF input normalizes and a trailing newline adds no empty row", () => {
  const parsed = parsePatch("@@ -1,1 +1,1 @@\r\n-old\r\n+new\r\n", false);
  assert.deepEqual(model(parsed.lines), ["hunk null null", "del 1 null", "add null 1"]);
});

void test("a bare separator outside a conflict block stays ordinary text", () => {
  const parsed = parsePatch("@@ -1,1 +1,1 @@\n =======\n", false);
  assert.deepEqual(model(parsed.lines), ["hunk null null", "context 1 1"]);
});
