// Path: app/src/crosstalk/identity_test.ts
// Description: Repository colour attribution protects nested roots and ambiguous folder names

import { test } from "node:test";
import assert from "node:assert/strict";
import { repoStyle, type Message } from "./identity.js";
import { DEFAULT_ACCENT_HEX } from "../lib/theme/accent_utils.js";

void test("sender roots select the most specific configured colour without guessing duplicate folders", () => {
  const message: Message = { id: "1", from: "claude@child#1234", to: "codex@home#4321", text: "hello", sent: 1, cwd: "E:\\Code\\parent\\child\\Source" };
  const repos = [
    { path: "E:\\Code\\parent", label: "Parent", accent: "#111111" },
    { path: "E:\\Code\\parent\\child", label: "Child", accent: "#222222" },
  ];
  assert.equal(repoStyle(message, repos).accent, "#222222");
  assert.equal(repoStyle({ ...message, cwd: "/mnt/e/Code/parent/child/Source" }, repos).label, "Child");
  assert.equal(repoStyle({ ...message, cwd: null }, repos).accent, "#222222");
  assert.equal(repoStyle({ ...message, cwd: null }, [...repos, { path: "E:\\Other\\child", label: "Other", accent: "#333333" }]).accent, DEFAULT_ACCENT_HEX);
});
