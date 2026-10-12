// Path: app/src/lib/overview/overview_lanes_test.ts
// Description: Exact lane attribution, concurrent sessions and stale turns cannot fabricate completion

import { test } from "node:test";
import assert from "node:assert/strict";
import { createLaneStore, recentlyFinished, type Turn } from "./overview_lanes.js";
import { laneGlow } from "./overview_lane_glow.js";
import type { SourceControlStatus } from "../../shared/protocol.js";

const turn = (id: number, from: string, cwd: string, text: Turn["text"], sent = 100): Turn =>
  ({ id: String(id).padStart(20, "0"), from, cwd, text, sent });

void test("turns choose the exact longest root; root replacement cannot inherit a session", () => {
  const store = createLaneStore();
  store.setRepositories([
    { repoId: "master", root: { kind: "host", path: "E:\\Blorb" } },
    { repoId: "lab", root: { kind: "host", path: "E:\\Blorb\\lab" } },
    { repoId: "wsl", root: { kind: "wsl", path: "/home/johnf/dev/Blorb" } },
  ]);
  store.receive(turn(1, "claude@lab#1", "/mnt/e/Blorb/lab/Source", "working"), 100_000);
  store.receive(turn(2, "codex@blorb#2", "/home/johnf/dev/Blorb/src", "working"), 100_000);
  store.receive(turn(3, "claude@blorb#3", "E:\\BlorbOther", "idle"), 100_000);
  assert.deepEqual([...store.getSnapshot().keys()], ["lab", "wsl"]);
  store.setRepositories([{ repoId: "lab", root: { kind: "host", path: "E:\\Elsewhere" } }]);
  assert.equal(store.getSnapshot().size, 0);
});

void test("a clean lane finishes only after its last session; stale and repeated idle cannot restart the fade", () => {
  const store = createLaneStore();
  store.setRepositories([{ repoId: "repo", root: { kind: "wsl", path: "/repo" } }]);
  store.receive(turn(1, "a", "/repo", "working"), 100_000);
  store.receive(turn(3, "b", "/repo", "working"), 100_000);
  store.receive(turn(4, "a", "/repo", "idle", 101), 101_000);
  store.receive(turn(2, "b", "/repo", "idle", 102), 102_000);
  assert.deepEqual(store.getSnapshot().get("repo"), { working: 1, finishedAt: null });
  store.receive(turn(5, "b", "/repo", "idle", 103), 103_000);
  store.receive(turn(6, "b", "/repo", "idle", 110), 110_000);
  assert.equal(store.getSnapshot().get("repo")?.finishedAt, 103_000);
  assert.equal(recentlyFinished(store.getSnapshot().get("repo"), 126_999), true);
  assert.equal(recentlyFinished(store.getSnapshot().get("repo"), 127_000), false);
});

void test("finished glow requires settled complete clean Git status; known dirty lanes retain their soft signal", () => {
  const status: SourceControlStatus = {
    branch: "main", headSha: "h", detached: false, upstream: null, ahead: null, behind: null,
    index: [], worktree: [], conflicts: [], omitted: { stagedOutsideRoot: 0, unmergedOutsideRoot: 0, unrepresentablePath: 0 },
    indexTreeSha: "t", snapshotId: "s", mutationInProgress: false, committable: false, truncated: false,
    capturedAtIso: "2026-10-12T00:00:00Z",
  };
  const lane = { working: 0, finishedAt: 100_000 };
  const ready = { kind: "ready" } as const;
  assert.equal(laneGlow(lane, { status, phase: ready }, "watching", 101_000), "finished");
  assert.equal(laneGlow({ working: 1, finishedAt: null }, { status, phase: ready }, "watching", 101_000), null);
  assert.equal(laneGlow(lane, { status, phase: { kind: "loading" } }, "watching", 101_000), null);
  assert.equal(laneGlow(lane, { status, phase: ready }, "held", 101_000), null);
  for (const incomplete of [{ ...status, truncated: true }, { ...status, mutationInProgress: true }]) {
    assert.equal(laneGlow(lane, { status: incomplete, phase: ready }, "watching", 101_000), null);
  }
  const dirty: SourceControlStatus = { ...status, worktree: [{ area: "worktree", path: "file.ts", change: "modified" }] };
  assert.equal(laneGlow(lane, { status: dirty, phase: { kind: "loading" } }, "watching", 999_000), "working");
});
