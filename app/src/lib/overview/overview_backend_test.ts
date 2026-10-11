// Path: app/src/lib/overview/overview_backend_test.ts
// Description: Root-scoped watcher failure and backend-version recovery regression oracles

import { test } from "node:test";
import assert from "node:assert/strict";
import { createOverviewStore } from "./overview_store.js";
import type { OverviewStore, OverviewTransport } from "./overview_types.js";

const ONLINE: OverviewTransport = {
  connected: true, helloOk: true, agentVersion: "0.1.24", watchedRepoIds: ["h", "w"], wslOnline: true, error: null,
};

function view(store: OverviewStore, repoId: string) {
  store.flush();
  const snapshot = store.getSnapshot().repos.get(repoId);
  assert.ok(snapshot);
  return snapshot;
}

void test("WSL flaps with a continuous sequence never invent an edit loss", () => {
  const store = createOverviewStore(() => 900_000);
  store.setRepositories([{ repoId: "w", root: { kind: "wsl", path: "/wsl" } }]);
  store.setTransport({ ...ONLINE, helloOk: false, watchedRepoIds: [] });
  store.setTransport({ ...ONLINE, helloOk: false, watchedRepoIds: [], wslOnline: false });
  assert.equal(view(store, "w").losses.length, 0);
  store.setTransport(ONLINE);
  store.markRehydrated();
  assert.equal(view(store, "w").availability, "watching");
  store.intake({ type: "fileDeltaCounters", repoId: "w", rootPath: "/wsl", seq: 9, withheld: 0, dropped: 0 });
  store.setTransport({ ...ONLINE, helloOk: false });
  store.setTransport({ ...ONLINE, helloOk: false, wslOnline: false });
  store.setTransport(ONLINE);
  store.markRehydrated();
  store.intake({ type: "fileDeltaCounters", repoId: "w", rootPath: "/wsl", seq: 10, withheld: 0, dropped: 0 });
  assert.equal(view(store, "w").losses.length, 0);
  store.dispose();
});

void test("a delayed error from a replaced root cannot make its replacement unavailable", () => {
  const store = createOverviewStore(() => 900_000);
  store.setRepositories([{ repoId: "h", root: { kind: "host", path: "/old" } }]);
  store.setTransport(ONLINE);
  store.setRepositories([{ repoId: "h", root: { kind: "host", path: "/new" } }]);
  store.setTransport(ONLINE);
  store.intake({ type: "error", scope: "watcher", message: "old failed", details: { repoId: "h", rootPath: "/old" } });
  store.intake({ type: "error", scope: "watcher", message: "unscoped failed", details: { repoId: "h" } });
  store.setRepoIssue("h", { kind: "host", path: "/old" }, "old history failed", true);
  store.setRepoIssue("h", { kind: "wsl", path: "/new" }, "other backend failed", true);
  assert.equal(view(store, "h").availability, "watching");
  assert.equal(view(store, "h").issue, null);
  store.setRepoIssue("h", { kind: "host", path: "/new" }, "current history failed");
  store.setRepoIssue("h", { kind: "host", path: "/old" }, "stale unavailable", true);
  assert.equal(view(store, "h").availability, "watching");
  assert.equal(view(store, "h").issue, "current history failed");
  store.setRepoIssue("h", { kind: "host", path: "/new" }, "WATCHER_STOPPED", true);
  assert.equal(view(store, "h").availability, "unwatched");
  assert.equal(view(store, "h").issue, "WATCHER_STOPPED");
  store.intake({ type: "error", scope: "watcher", message: "current failed", details: { repoId: "h", rootPath: "/new" } });
  store.setRepoIssue("h", { kind: "host", path: "/new" }, "UNKNOWN_REPO", true);
  store.intake({ type: "snapshot", repoId: "h", rootPath: "/new", recent: [] });
  assert.equal(view(store, "h").availability, "error");
  assert.equal(view(store, "h").issue, "current failed");
  store.dispose();
});

void test("an old external WSL backend requires an update while the host continues watching", () => {
  const store = createOverviewStore(() => 900_000);
  store.setRepositories([
    { repoId: "h", root: { kind: "host", path: "/host" } },
    { repoId: "w", root: { kind: "wsl", path: "/wsl" } },
  ]);
  store.setTransport(ONLINE);
  store.intake({
    type: "error", scope: "wslBackend", message: "WSL agent must be updated to 0.1.24",
    details: { rawCode: "WSL_BACKEND_VERSION_MISMATCH" },
  });
  store.setTransport({ ...ONLINE, wslOnline: false });
  assert.equal(view(store, "w").availability, "update-required");
  assert.equal(view(store, "w").issue, "WSL agent must be updated to 0.1.24");
  assert.equal(view(store, "h").availability, "watching");
  assert.equal(view(store, "h").issue, null);
  store.setTransport({ ...ONLINE, helloOk: false, wslOnline: false });
  store.setTransport({ ...ONLINE, wslOnline: false });
  store.markRehydrated();
  store.intake({ type: "snapshot", repoId: "w", rootPath: "/wsl", recent: [] });
  assert.equal(view(store, "w").availability, "update-required");
  store.setTransport({ ...ONLINE, helloOk: false });
  store.markRehydrated();
  assert.equal(view(store, "w").availability, "update-required");
  store.setTransport(ONLINE);
  store.markRehydrated();
  assert.equal(view(store, "w").availability, "watching");
  assert.equal(view(store, "w").issue, null);
  assert.equal(view(store, "h").losses.length, 0);
  store.dispose();
});
