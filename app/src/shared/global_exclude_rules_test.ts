// Path: app/src/shared/global_exclude_rules_test.ts
// Description: The frontend global-exclude matcher agrees with the Rust scanner on suffixes, names, and path segments

import { test } from "node:test";
import assert from "node:assert/strict";
import {
  isGloballyExcludedDirName,
  isGloballyExcludedFileName,
  isGloballyExcludedPath,
  normalizeGlobalExcludes,
} from "./global_exclude_rules.js";

const rules = normalizeGlobalExcludes({
  dirNames: [" Node_Modules ", ""],
  dirSuffixes: ["egg-info"],
  fileNames: ["Thumbs.db"],
  extensions: [".blend", "BLEND1", "~"],
  patterns: ["/wandb/", ""],
});

void test("extensions match file suffixes case-insensitively, with or without the leading dot", () => {
  assert.equal(isGloballyExcludedFileName("blorb_world.blend", rules), true);
  assert.equal(isGloballyExcludedFileName("Blorb_World.BLEND1", rules), true);
  assert.equal(isGloballyExcludedFileName("notes.md~", rules), true);
  assert.equal(isGloballyExcludedFileName("blorb_world.blend.png", rules), false);
});

void test("file names match exactly after lowercasing", () => {
  assert.equal(isGloballyExcludedFileName("thumbs.db", rules), true);
  assert.equal(isGloballyExcludedFileName("thumbs.db.bak", rules), false);
});

void test("directory names and suffixes match the scanner rules", () => {
  assert.equal(isGloballyExcludedDirName("node_modules", rules), true);
  assert.equal(isGloballyExcludedDirName("pkg.egg-info", rules), true);
  assert.equal(isGloballyExcludedDirName("src", rules), false);
});

void test("path segment patterns match whole, leading, trailing, and interior segments only", () => {
  assert.equal(isGloballyExcludedPath("wandb", rules), true);
  assert.equal(isGloballyExcludedPath("wandb/run.log", rules), true);
  assert.equal(isGloballyExcludedPath("witness/wandb", rules), true);
  assert.equal(isGloballyExcludedPath("witness/wandb/run.log", rules), true);
  assert.equal(isGloballyExcludedPath("witness/wandb_export/run.log", rules), false);
});
