// Path: app/src/lib/overview/overview_edits_test.ts
// Description: Independent feed-entry retention, global identity and immutable publication oracles

import { test } from "node:test";
import assert from "node:assert/strict";
import type { FileDeltaEvent } from "../../shared/protocol.js";
import { createOverviewStore } from "./overview_store.js";

const NOW = Date.parse("2026-10-09T12:00:00Z");

function delta(repoId: string, seq: number): FileDeltaEvent {
  return {
    type: "fileDelta", repoId, rootPath: `/${repoId}`, seq, path: "same.ts", kind: "code", op: "modify",
    mtime: new Date(NOW).toISOString(), folded: 0, withheld: 0, dropped: 0,
    payload: {
      kind: "text", patch: `@@ -1 +1 @@\n-before\n+edit ${String(seq)}\n`, baseline: "previousSighting", truncated: false,
      stats: { added: 1, removed: 1, hunks: 1, newLines: 1 },
    },
  };
}

void test("each settled delta has a stable global identity in a twelve-entry per-root feed", () => {
  const store = createOverviewStore(() => NOW);
  store.setRepositories([
    { repoId: "r", root: { kind: "host", path: "/r" } },
    { repoId: "s", root: { kind: "host", path: "/s" } },
  ]);
  for (let seq = 1; seq <= 14; seq += 1) {
    store.intake(delta("r", seq), NOW + seq);
    if (seq === 2) store.intake(delta("s", 1), NOW + seq);
  }
  store.flush();
  const before = store.getSnapshot();
  const first = before.repos.get("r");
  assert.ok(first);
  assert.equal(first.edits.length, 12);
  assert.deepEqual(first.edits.map((edit) => edit.id), Array.from({ length: 12 }, (_, index) => index + 4));
  assert.equal(before.repos.get("s")?.edits[0]?.id, 3);
  assert.deepEqual(first.edits.map((edit) => edit.at), Array.from({ length: 12 }, (_, index) => NOW + index + 3));
  assert.equal(first.recent.length, 0);
  assert.equal(first.lastChangedAt, null);
  store.intake(delta("r", 14), NOW + 15);
  store.intake({
    type: "snapshot", repoId: "r", rootPath: "/r", recent: [
      { path: "history.ts", kind: "code", changeType: "change", mtime: new Date(NOW - 1).toISOString() },
    ],
  }, NOW + 15);
  store.flush();
  assert.deepEqual(store.getSnapshot().repos.get("r")?.edits.map((edit) => edit.id), first.edits.map((edit) => edit.id));
  const event = delta("r", 15);
  store.intake(event, NOW + 15);
  if (event.payload.kind === "text") event.payload.patch = "mutated after intake";
  store.flush();
  const after = store.getSnapshot().repos.get("r");
  assert.ok(after);
  assert.deepEqual(after.edits.map((edit) => edit.id), Array.from({ length: 12 }, (_, index) => index + 5));
  assert.equal(after.edits.at(-1)?.lines.at(-1)?.text, "edit 15");
  assert.equal(first.edits[0]?.id, 4);
  assert.equal(first.edits.at(-1)?.id, 15);
  store.setRepositories([{ repoId: "r", root: { kind: "host", path: "/replacement" } }]);
  store.intake(delta("r", 16), NOW + 16);
  store.flush();
  assert.equal(store.getSnapshot().repos.get("r")?.edits.length, 0);
  store.intake({ ...delta("r", 1), rootPath: "/replacement" }, NOW + 17);
  store.flush();
  assert.equal(store.getSnapshot().repos.get("r")?.edits[0]?.id, 17);
  assert.equal(store.getSnapshot().repos.has("s"), false);
  store.dispose();
});
