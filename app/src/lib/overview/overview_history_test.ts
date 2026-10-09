// Path: app/src/lib/overview/overview_history_test.ts
// Description: Reconnect history orders recent paths by observed time without inventing live activity

import { test } from "node:test";
import assert from "node:assert/strict";
import { createOverviewStore } from "./overview_store.js";

const NOW = Date.parse("2026-10-09T12:00:00Z");

void test("newer reconnect history displaces old live paths while settled content remains independent", () => {
  const store = createOverviewStore(() => NOW);
  store.setRepositories([{ repoId: "r", root: { kind: "host", path: "/r" } }]);
  store.setTransport({
    connected: true, helloOk: true, agentVersion: "0.1.24", watchedRepoIds: ["r"], wslOnline: true, error: null,
  });
  for (let index = 0; index < 6; index += 1) {
    store.intake({
      type: "fileChanged", repoId: "r", rootPath: "/r", path: `old${String(index)}.ts`,
      kind: "code", changeType: "change", mtime: new Date(NOW - 120_000 + index).toISOString(),
    }, NOW - 120_000 + index);
  }
  store.intake({
    type: "fileDelta", repoId: "r", rootPath: "/r", seq: 1, path: "settled.ts", kind: "code", op: "modify",
    mtime: new Date(NOW - 60_000).toISOString(), folded: 0, withheld: 0, dropped: 0,
    payload: {
      kind: "text", patch: "@@ -1 +1 @@\n-old\n+new\n", baseline: "previousSighting", truncated: false,
      stats: { added: 1, removed: 1, hunks: 1, newLines: 1 },
    },
  }, NOW - 60_000);
  store.intake({
    type: "snapshot", repoId: "r", rootPath: "/r", recent: [
      { path: "old5.ts", kind: "code", changeType: "change", mtime: new Date(NOW - 180_000).toISOString() },
      {
        path: "reconnected.ts", kind: "code", changeType: "add", mtime: new Date(NOW - 30_000).toISOString(),
        activity: {
          firstSeenAtIso: new Date(NOW - 30_000).toISOString(), lastSeenAtIso: new Date(NOW - 30_000).toISOString(),
          updateCount: 1, burstCount: 1, history: [],
        },
      },
      { path: "old0.ts", kind: "code", changeType: "unlink", mtime: new Date(NOW - 15_000).toISOString() },
      { path: "invalid.ts", kind: "code", changeType: "change", mtime: "invalid" },
      { path: "future.ts", kind: "code", changeType: "change", mtime: new Date(NOW + 1).toISOString() },
    ],
  }, NOW);
  store.flush();
  const snapshot = store.getSnapshot().repos.get("r");
  assert.ok(snapshot);
  assert.equal(snapshot.recent.length, 6);
  assert.deepEqual(snapshot.recent.slice(0, 2).map((signal) => [signal.path, signal.source, signal.count]), [
    ["old0.ts", "history", 0], ["reconnected.ts", "history", 0],
  ]);
  assert.equal(snapshot.recent[2]?.path, "old5.ts");
  assert.equal(snapshot.recent[2].source, "live");
  assert.equal(snapshot.recent.some((signal) => signal.path === "invalid.ts" || signal.path === "future.ts"), false);
  assert.equal(snapshot.edits.length, 1);
  assert.equal(snapshot.lastChangedAt, NOW - 120_000 + 5);
  assert.equal(snapshot.lastWorkedAt, NOW - 30_000);
  const edit = snapshot.edits.at(-1);
  assert.ok(edit);
  assert.equal(edit.path, "settled.ts");
  assert.equal(edit.at, NOW - 60_000);
  store.dispose();
});
