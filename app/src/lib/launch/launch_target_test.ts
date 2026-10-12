// Path: app/src/lib/launch/launch_target_test.ts
// Description: Effective launch target honours a persisted choice only while it is still a candidate

import { test } from "node:test";
import assert from "node:assert/strict";
import type { LaunchCandidate } from "../../shared/protocol_launch.js";
import { effectiveLaunchTarget, launchFileName } from "./launch_target.js";

const film: LaunchCandidate = { relativePath: "Film.html", kind: "html", lastUsedMs: null };
const main: LaunchCandidate = { relativePath: "Main.html", kind: "html", lastUsedMs: 5 };

void test("persisted choice wins while it is a candidate", () => {
  assert.equal(effectiveLaunchTarget([main, film], "Film.html"), film);
});

void test("stale or absent choice falls back to the top-ranked candidate", () => {
  assert.equal(effectiveLaunchTarget([main, film], "Gone.html"), main);
  assert.equal(effectiveLaunchTarget([main, film], undefined), main);
  assert.equal(effectiveLaunchTarget([], "Film.html"), null);
});

void test("file name strips directories of either separator", () => {
  assert.equal(launchFileName("sub\\Game.uproject"), "Game.uproject");
  assert.equal(launchFileName("Game.uproject"), "Game.uproject");
});
