// Path: app/src/lib/launch/launch_target_test.ts
// Description: Effective launch target honours a persisted choice only while it is still a candidate

import { test } from "node:test";
import assert from "node:assert/strict";
import type { LaunchCandidate } from "../../shared/protocol_launch.js";
import { demoTargetFor, effectiveLaunchTarget, launchFileName } from "./launch_target.js";
import { RepoConfigSchema } from "../../shared/config/repo_config.js";

const film: LaunchCandidate = { target: "Film.html", kind: "html", lastUsedMs: null };
const main: LaunchCandidate = { target: "Main.html", kind: "html", lastUsedMs: 5 };

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
  assert.equal(launchFileName("J:\\BlenderAuthoring\\blorb\\mainland\\index.html"), "index.html");
  assert.equal(launchFileName("http://localhost:8788/repairs/"), "repairs");
});

void test("a group's working demo is shared across its roots and independent of Unreal choice", () => {
  const repos = ["blorb", "blorb-code", "blorb-lab"].map((repoId) => RepoConfigSchema.parse({
    repoId, label: repoId, groupId: "blorb", root: { kind: "host", path: `E:/Worktrees/${repoId}` },
  }));
  const demo = "http://127.0.0.1:8777/Docs/Reference/momo.html";
  const targets = { blorb: demo };
  for (const repo of repos) assert.equal(demoTargetFor(repos, targets, repo.repoId), demo);
  assert.equal(demoTargetFor(repos, targets, "unknown"), null);
  assert.equal(demoTargetFor(repos, {}, "blorb-lab"), null);
  const project: LaunchCandidate = { target: "Blorb.uproject", kind: "unreal", lastUsedMs: null };
  assert.equal(effectiveLaunchTarget([project], undefined), project);
});
