// Path: app/src/lib/overview/overview_store_test.ts
// Description: Independent oracles for all-root retention, activity identity, bounds and watcher recovery

import { test } from "node:test";
import assert from "node:assert/strict";
import type { AgentEvent, FileChangedEvent, FileDeltaEvent } from "../../shared/protocol.js";
import { createOverviewStore } from "./overview_store.js";
import type { OverviewStore, OverviewTransport } from "./overview_types.js";

const TIME = Date.parse("2026-10-09T12:00:00Z");
const ONLINE: OverviewTransport = {
  connected: true, helloOk: true, agentVersion: "0.1.24", watchedRepoIds: [], wslOnline: true, error: null,
};

function repository(repoId = "r", path = "/r", kind: "host" | "wsl" = "host") {
  return { repoId, root: { kind, path } };
}

function changed(path: string, repoId = "r", rootPath = "/r"): FileChangedEvent {
  return { type: "fileChanged", repoId, rootPath, path, kind: "code", changeType: "change", mtime: "2000-01-01T00:00:00Z" };
}

function delta(seq: number, path = "settled.ts"): FileDeltaEvent {
  return {
    type: "fileDelta", repoId: "r", rootPath: "/r", seq, path, kind: "code", op: "modify",
    mtime: "2000-01-01T00:00:00Z", folded: 0, withheld: 0, dropped: 0,
    payload: {
      kind: "text", patch: "@@ -1 +1 @@\n-old\n+new\n", baseline: "index", truncated: false,
      stats: { added: 1, removed: 1, hunks: 1, newLines: 1 },
    },
  };
}

function repo(store: OverviewStore, repoId = "r") {
  store.flush();
  const snapshot = store.getSnapshot().repos.get(repoId);
  assert.ok(snapshot);
  return snapshot;
}

function history(path: string, mtime = "2026-01-01T00:00:00Z"): AgentEvent {
  return { type: "snapshot", repoId: "r", rootPath: "/r", recent: [{ path, kind: "code", changeType: "change", mtime }] };
}

void test("every configured root captures without visiting a detailed Stream store", () => {
  const store = createOverviewStore(() => TIME);
  const repositories = Array.from({ length: 9 }, (_, index) => repository(`r${String(index)}`, `/r${String(index)}`));
  store.setRepositories(repositories);
  store.setTransport(ONLINE);
  for (const entry of repositories) store.intake(changed("live.ts", entry.repoId, entry.root.path), TIME);
  store.flush();
  assert.equal(store.getSnapshot().repos.size, 9);
  assert.deepEqual([...store.getSnapshot().repos.keys()], repositories.map((entry) => entry.repoId));
  for (const entry of repositories) {
    const snapshot = repo(store, entry.repoId);
    assert.equal(snapshot.recent[0]?.count, 1);
    assert.equal(snapshot.availability, "watching");
  }
  store.dispose();
});

void test("live notification counts, historical orientation and settled content remain separate", () => {
  const store = createOverviewStore(() => TIME);
  store.setRepositories([repository()]);
  store.setTransport(ONLINE);
  store.intake(history("history.ts"));
  assert.equal(repo(store).edits.length, 0);
  assert.equal(repo(store).lastChangedAt, null);
  assert.equal(repo(store).recent[0]?.source, "history");
  store.intake(changed("live.ts"), TIME);
  store.intake(changed("live.ts"), TIME + 1);
  store.intake(history("live.ts", "2099-01-01T00:00:00Z"));
  store.intake(delta(1), TIME + 2);
  const snapshot = repo(store);
  assert.equal(snapshot.edits.length, 1);
  assert.equal(snapshot.lastChangedAt, TIME + 1);
  assert.equal(snapshot.recent[0]?.path, "live.ts");
  assert.equal(snapshot.recent[0].source, "live");
  assert.equal(snapshot.recent[0].count, 2);
  const edit = snapshot.edits.at(-1);
  assert.ok(edit);
  assert.equal(edit.path, "settled.ts");
  assert.equal(edit.at, TIME + 2);
  assert.equal(edit.baseline, "index");
  assert.equal(edit.image, null);
  assert.deepEqual(edit.lines, [{ kind: "del", text: "old" }, { kind: "add", text: "new" }]);
  store.dispose();
});

void test("removed and replaced roots reject every delayed activity event", () => {
  const store = createOverviewStore(() => TIME);
  store.setRepositories([repository()]);
  store.intake(changed("old.ts"), TIME);
  assert.equal(repo(store).recent[0]?.count, 1);
  store.setRepositories([repository("r", "/replacement")]);
  store.intake(changed("stale.ts"), TIME);
  store.intake(delta(1), TIME);
  store.intake(history("stale-history.ts"));
  store.intake({ type: "fileDeltaCounters", repoId: "r", rootPath: "/r", seq: 2, withheld: 99, dropped: 99 });
  const replaced = repo(store);
  assert.equal(replaced.lastChangedAt, null);
  assert.equal(replaced.losses.length, 0);
  assert.equal(replaced.recent.length, 0);
  assert.equal(replaced.edits.length, 0);
  store.intake(changed("current.ts", "r", "/replacement"), TIME);
  assert.equal(repo(store).recent[0]?.path, "current.ts");
  store.setRepositories([]);
  store.intake(changed("removed.ts", "r", "/replacement"), TIME);
  store.flush();
  assert.equal(store.getSnapshot().repos.size, 0);
  store.setRepositories([repository("r", "/replacement", "wsl")]);
  assert.equal(repo(store).recent.length, 0);
  store.dispose();
});

void test("floods keep six folded paths and a short immutable settled preview", () => {
  let now = TIME;
  const store = createOverviewStore(() => now);
  store.setRepositories([repository()]);
  store.flush();
  const before = store.getSnapshot();
  let notifications = 0;
  store.subscribe(() => { notifications += 1; });
  for (let index = 0; index < 2000; index += 1) store.intake(changed(`f${String(index % 10)}.ts`), TIME);
  const large = delta(1);
  if (large.payload.kind !== "text") throw new Error("text fixture");
  large.payload.patch = `@@ -0,0 +1,10 @@\n${Array.from({ length: 10 }, () => `+${"x".repeat(500)}`).join("\n")}\n`;
  large.payload.stats.added = 10;
  store.intake(large, TIME);
  assert.equal(store.getSnapshot(), before);
  const snapshot = repo(store);
  assert.equal(notifications, 1);
  assert.equal(snapshot.lastChangedAt, TIME);
  assert.equal(snapshot.recent.length, 6);
  const edit = snapshot.edits.at(-1);
  assert.ok(edit);
  assert.equal(edit.lines.length, 6);
  assert.ok(edit.lines.every((line) => line.text.length === 180));
  assert.equal(edit.truncated, true);
  assert.equal(edit.added, 10);
  now += 300_000;
  store.intake(changed("new.ts"), now);
  assert.equal(repo(store).recent[0]?.path, "new.ts");
  assert.equal(snapshot.lastChangedAt, TIME);
  assert.equal(snapshot.recent[0]?.path, "f9.ts");
  store.flush();
  assert.equal(notifications, 2);
  store.dispose();
});

void test("losses mark where they happened, merge until the next edit, and a restart is a new stream", () => {
  const store = createOverviewStore(() => TIME);
  store.setRepositories([repository()]);
  store.intake({ ...delta(4), folded: 50, withheld: 2 }, TIME);
  store.intake({ ...delta(4), path: "duplicate.ts", dropped: 99 }, TIME);
  store.intake({ type: "fileDeltaCounters", repoId: "r", rootPath: "/r", seq: 7, withheld: 3, dropped: 1 }, TIME);
  store.intake({ type: "fileDeltaCounters", repoId: "r", rootPath: "/r", seq: 8, withheld: 1, dropped: 0 }, TIME + 5);
  assert.deepEqual(repo(store).losses.map((loss) => [loss.count, loss.at]), [[2, TIME], [7, TIME + 5]]);
  assert.equal(repo(store).edits.at(-1)?.path, "settled.ts");
  assert.ok((repo(store).losses[0]?.id ?? 0) < (repo(store).edits.at(-1)?.id ?? 0));
  store.intake(delta(1, "restart.ts"), TIME);
  assert.equal(repo(store).edits.at(-1)?.path, "restart.ts");
  assert.equal(repo(store).losses.length, 2);
  assert.equal(repo(store).lastChangedAt, null);
  store.dispose();
});

void test("watcher failures stay sticky through traffic and clear only after backend recovery", () => {
  const store = createOverviewStore(() => TIME);
  store.setRepositories([repository(), repository("w", "/w", "wsl")]);
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r", "w"] });
  store.intake({ type: "error", scope: "watcher", message: "watch failed", details: { repoId: "r", rootPath: "/r" } });
  store.intake({ type: "error", scope: "bundles", message: "unrelated", details: { repoId: "r" } });
  store.intake(changed("still.ts"), TIME);
  store.intake(history("still.ts"));
  assert.equal(repo(store).availability, "error");
  assert.equal(repo(store).issue, "watch failed");
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r"], wslOnline: false });
  assert.equal(repo(store, "w").availability, "held");
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r"], wslOnline: true });
  store.markRehydrated();
  assert.equal(repo(store).availability, "error");
  store.setTransport({ ...ONLINE, connected: false, helloOk: false });
  assert.equal(repo(store).availability, "offline");
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r"] });
  store.markRehydrated();
  assert.equal(repo(store).availability, "watching");
  assert.equal(repo(store).issue, null);
  assert.equal(repo(store).losses.length, 0);
  store.setRepoIssue("r", { kind: "host", path: "/r" }, "history refresh failed");
  assert.equal(repo(store).issue, "history refresh failed");
  assert.equal(repo(store).availability, "watching");
  store.intake(changed("new.ts"), TIME);
  assert.equal(repo(store).issue, "history refresh failed");
  store.intake(history("new.ts"));
  assert.equal(repo(store).issue, null);
  store.setRepoIssue("r", { kind: "host", path: "/r" }, "WATCHER_STOPPED", true);
  store.intake(changed("raw.ts"), TIME);
  assert.equal(repo(store).availability, "unwatched");
  assert.equal(repo(store).issue, "WATCHER_STOPPED");
  store.intake(history("restarted.ts"));
  assert.equal(repo(store).availability, "watching");
  assert.equal(repo(store).issue, null);
  store.setTransport({ ...ONLINE, agentVersion: "0.1.23" });
  assert.equal(repo(store).availability, "update-required");
  store.dispose();
});

void test("initial hello has no gap and a WSL flap marks only the real host sequence gap", () => {
  const store = createOverviewStore(() => TIME);
  store.setRepositories([repository(), repository("w", "/w", "wsl")]);
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r", "w"] });
  store.markRehydrated();
  store.intake(delta(4), TIME);
  store.intake({ ...delta(4), repoId: "w", rootPath: "/w" }, TIME);
  assert.equal(repo(store).losses.length, 0);
  assert.equal(repo(store, "w").losses.length, 0);
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r", "w"], helloOk: false });
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r", "w"] });
  store.markRehydrated();
  store.intake(delta(5), TIME);
  assert.equal(repo(store).losses.length, 0);
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r", "w"], wslOnline: false });
  store.setTransport({ ...ONLINE, watchedRepoIds: ["r", "w"] });
  store.markRehydrated();
  store.intake(delta(7), TIME);
  store.intake({ ...delta(10), repoId: "w", rootPath: "/w" }, TIME);
  assert.deepEqual(repo(store).losses.map((loss) => loss.count), [1]);
  assert.equal(repo(store, "w").losses.length, 0);
  store.dispose();
});

void test("a root-matched snapshot confirms WSL watching even when cached host hello omits it", () => {
  const store = createOverviewStore(() => TIME);
  store.setRepositories([repository("r", "/r", "wsl")]);
  store.setTransport(ONLINE);
  assert.equal(repo(store).availability, "unwatched");
  store.intake(history("before.ts"));
  assert.equal(repo(store).availability, "watching");
  store.intake({ type: "error", scope: "watcher", message: "WSL watch failed", details: { repoId: "r", rootPath: "/r" } });
  store.setTransport({ ...ONLINE, wslOnline: false });
  store.setTransport(ONLINE);
  store.markRehydrated();
  assert.equal(repo(store).availability, "error");
  store.intake(changed("traffic.ts"), TIME);
  assert.equal(repo(store).availability, "error");
  store.intake(history("after.ts"));
  assert.equal(repo(store).availability, "watching");
  store.dispose();
});

void test("image, opaque and gone edits expose metadata without retaining pixel or opaque content", () => {
  const store = createOverviewStore(() => TIME);
  store.setRepositories([repository()]);
  store.intake({ ...delta(1), kind: "image", payload: { kind: "image", bytes: 123, mimeType: "image/png", mtimeMs: TIME - 60_000 } });
  assert.deepEqual(repo(store).edits.at(-1)?.image, { bytes: 123, mimeType: "image/png", mtimeMs: TIME - 60_000 });
  assert.equal(repo(store).edits.at(-1)?.detail, "Image · image/png · 123 bytes");
  assert.deepEqual(repo(store).edits.at(-1)?.lines, []);
  store.intake({ ...delta(2), payload: { kind: "opaque", bytes: 456, reason: "tooLarge" } });
  assert.equal(repo(store).edits.at(-1)?.detail, "No text preview · content too large · 456 bytes");
  assert.equal(repo(store).edits.at(-1)?.baseline, null);
  assert.equal(repo(store).edits.at(-1)?.image, null);
  store.intake({ ...delta(3), op: "remove", payload: { kind: "gone" } });
  assert.equal(repo(store).edits.at(-1)?.detail, "File removed · content unavailable");
  assert.equal(repo(store).lastChangedAt, null);
  store.dispose();
});
