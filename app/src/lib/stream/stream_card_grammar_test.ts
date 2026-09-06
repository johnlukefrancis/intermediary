// Path: app/src/lib/stream/stream_card_grammar_test.ts
// Description: Line selection: head of a single segment, tail across several, one hunk per segment, newest offset

import { test } from "node:test";
import assert from "node:assert/strict";
import type { DiffLine } from "../diff/diff_lines.js";
import { selectLines } from "./stream_card_grammar.js";

function segment(label: string, rows: number, hunks = 1): DiffLine[] {
  const lines: DiffLine[] = [{ kind: "meta", text: "diff --git", oldNo: null, newNo: null }];
  for (let index = 0; index < hunks; index += 1) lines.push({ kind: "hunk", text: `@@ ${label}${String(index)}`, oldNo: null, newNo: null });
  for (let index = 0; index < rows; index += 1) lines.push({ kind: "add", text: `+${label}${String(index)}`, oldNo: null, newNo: index });
  return lines;
}

const CAP = 4;

void test("a single segment prints its head: the hunk header then the first rows", () => {
  const selected = selectLines([segment("a", 10, 2)], false, CAP);
  assert.deepEqual(selected.lines.map((line) => line.text), ["@@ a0", "+a0", "+a1", "+a2"]);
  // 11 printable rows (meta dropped, second hunk dropped), 4 printed
  assert.equal(selected.hiddenLines, 11 - CAP);
  assert.equal(selected.newestFrom, 0);
});

void test("several segments print their tail: the newest rows show and older rows fill the cap above", () => {
  const selected = selectLines([segment("a", 6), segment("b", 2)], false, CAP);
  assert.deepEqual(selected.lines.map((line) => line.text), ["+a5", "@@ b0", "+b0", "+b1"]);
  assert.equal(selected.hiddenLines, 7 + 3 - CAP);
  assert.equal(selected.newestFrom, 1);
});

void test("a newest segment larger than the cap fills the whole collapsed view", () => {
  const selected = selectLines([segment("a", 3), segment("b", 9)], false, CAP);
  assert.deepEqual(selected.lines.map((line) => line.text), ["+b5", "+b6", "+b7", "+b8"]);
  assert.equal(selected.newestFrom, 0);
});

void test("expanded prints every segment oldest first with one hunk header each", () => {
  const selected = selectLines([segment("a", 2, 3), segment("b", 1)], true, CAP);
  assert.deepEqual(selected.lines.map((line) => line.text), ["@@ a0", "+a0", "+a1", "@@ b0", "+b0"]);
  assert.equal(selected.hiddenLines, 0);
  assert.equal(selected.newestFrom, 3);
});
