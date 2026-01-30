// Path: agent/src/bundles/ignore_rules.test.ts
// Description: Unit tests for bundle ignore rules

import assert from "node:assert/strict";
import { test } from "node:test";
import { shouldIgnoreEntry } from "./ignore_rules.js";

void test("ignores known build/cache directories", () => {
  assert.equal(shouldIgnoreEntry("node_modules", true), true);
  assert.equal(shouldIgnoreEntry(".git", true), true);
  assert.equal(shouldIgnoreEntry("dist", true), true);
  assert.equal(shouldIgnoreEntry("target", true), true);
});

void test("ignores known sensitive files", () => {
  assert.equal(shouldIgnoreEntry(".env", false), true);
  assert.equal(shouldIgnoreEntry(".env.local", false), true);
  assert.equal(shouldIgnoreEntry(".DS_Store", false), true);
});

void test("allows dotfiles and normal files not in ignore lists", () => {
  assert.equal(shouldIgnoreEntry(".github", true), false);
  assert.equal(shouldIgnoreEntry(".gitignore", false), false);
  assert.equal(shouldIgnoreEntry("README.md", false), false);
});
