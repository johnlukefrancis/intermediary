// Path: app/src/lib/overview/overview_focus_test.ts
// Description: Active-work selection rejects idle roots and file mtimes, ages out, and re-enters on work

import { test } from "node:test";
import assert from "node:assert/strict";
import { createOverviewStore } from "./overview_store.js";
import { isWorking, lastWorkAt } from "./overview_focus.js";

const NOW = Date.parse("2026-10-09T00:00:00Z");

void test("watching alone never adds idle roots; a real edit enters and expires after fifteen minutes", () => {
  const store = createOverviewStore(() => NOW);
  store.setRepositories([{ repoId: "r", root: { kind: "host", path: "/r" } }]);
  store.setTransport({ connected: true, helloOk: true, agentVersion: "0.1.24", watchedRepoIds: ["r"], wslOnline: true, error: null });
  store.flush();
  assert.equal(isWorking(store.getSnapshot().repos.get("r"), NOW), false);
  store.intake({ type: "fileChanged", repoId: "r", rootPath: "/r", path: "work.ts", kind: "code", changeType: "change", mtime: "2000-01-01T00:00:00Z" }, NOW);
  store.flush();
  const held = store.getSnapshot().repos.get("r");
  assert.equal(isWorking(held, NOW), true);
  assert.equal(isWorking(held, NOW + 899_999), true);
  assert.equal(isWorking(held, NOW + 900_000), false);
  store.intake({ type: "fileChanged", repoId: "r", rootPath: "/r", path: "next.ts", kind: "code", changeType: "change", mtime: "2000-01-01T00:00:00Z" }, NOW + 1_000_000);
  store.flush();
  assert.equal(isWorking(store.getSnapshot().repos.get("r"), NOW + 1_000_000), true);
  assert.equal(lastWorkAt(held), NOW);
  store.dispose();
});

void test("startup uses observation time, never file mtime or a future saved timestamp, without inventing live edits", () => {
  const store = createOverviewStore(() => NOW);
  store.setRepositories([{ repoId: "r", root: { kind: "host", path: "/r" } }]);
  const path = { path: "history.ts", kind: "code" as const, changeType: "change" as const, mtime: new Date(NOW).toISOString() };
  store.intake({ type: "snapshot", repoId: "r", rootPath: "/r", recent: [path] });
  store.flush();
  assert.equal(isWorking(store.getSnapshot().repos.get("r"), NOW), false);
  const activity = { firstSeenAtIso: new Date(NOW).toISOString(), lastSeenAtIso: new Date(NOW + 60_000).toISOString(), updateCount: 1, burstCount: 1, history: [] };
  store.intake({ type: "snapshot", repoId: "r", rootPath: "/r", recent: [{ ...path, activity }] });
  store.flush();
  assert.equal(isWorking(store.getSnapshot().repos.get("r"), NOW), false);
  activity.lastSeenAtIso = new Date(NOW - 120_000).toISOString();
  store.intake({ type: "snapshot", repoId: "r", rootPath: "/r", recent: [{ ...path, activity }] });
  store.flush();
  const snapshot = store.getSnapshot().repos.get("r");
  assert.equal(isWorking(snapshot, NOW), true);
  assert.equal(snapshot?.lastChangedAt, null);
  assert.equal(snapshot.edits.length, 0);
  assert.equal(snapshot.recent[0]?.source, "history");
  store.dispose();
});
