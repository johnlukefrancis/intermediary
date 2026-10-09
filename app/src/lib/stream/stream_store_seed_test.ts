// Path: app/src/lib/stream/stream_store_seed_test.ts
// Description: Only the repo's own events land: cross-repo snapshots never seed, a foreign fileDelta prints no card, live rings never reseed, no seed API exists

import { test } from "node:test";
import assert from "node:assert/strict";
import type { AgentEvent, FileEntry } from "../../shared/protocol.js";
import { FLUSH_MS } from "./stream_bounds.js";
import { createStreamStore } from "./stream_store.js";
import { createStreamStoreRegistry, streamRootIdentity } from "./stream_store_registry.js";
import { resetSeq, textDelta } from "./testing/stream_fixtures.js";
import type { StreamStore, StreamStoreDeps } from "./stream_types.js";

/** Timers fire in order on `run()`; the clock advances by FLUSH_MS per flush so the reducers see time move */
function fakeTimers(): StreamStoreDeps & { run(): void } {
  let now = 10_000;
  const due: (() => void)[] = [];
  return {
    now: () => now,
    setTimer(callback) { due.push(callback); return due.length; },
    clearTimer() { /* the store only clears timers it will never need again */ },
    run() {
      while (due.length > 0) {
        now += FLUSH_MS;
        due.shift()?.();
      }
    },
  };
}

function entry(path: string, lastSeenAtIso: string): FileEntry {
  return { path, kind: "code", changeType: "change", mtime: lastSeenAtIso };
}

function snapshot(repoId: string, paths: readonly string[]): Extract<AgentEvent, { type: "snapshot" }> {
  return { type: "snapshot", repoId, rootPath: `/repo/${repoId}`, recent: paths.map((path, index) => entry(path, `2026-09-06T00:00:0${String(index)}Z`)) };
}

function historyPaths(store: StreamStore): string[] {
  return store.getSnapshot().ring.cards.flatMap((card) => (card.kind === "history" ? [card.path] : []));
}

const ROWS_A = ["a/stream_store_test.ts", "a/stream_panel_design.md"];
const ROWS_B = ["b/fish.png", "b/glitch.ts"];

void test("a store seeds history rows only from its own repo's snapshot; another repo's rows are dropped", () => {
  const timers = fakeTimers();
  const store = createStreamStore("b", timers);
  store.intake(snapshot("a", ROWS_A));
  timers.run();
  assert.deepEqual(historyPaths(store), []);
  store.intake(snapshot("b", ROWS_B));
  timers.run();
  assert.deepEqual(historyPaths(store), ["b/fish.png", "b/glitch.ts"]);
  store.dispose();
});

void test("a fileDelta for another repo never becomes a card, whichever store it reaches", () => {
  resetSeq();
  const timers = fakeTimers();
  const store = createStreamStore("b", timers);
  store.intake(textDelta("a/stream_store_test.ts", { repoId: "a" }));
  timers.run();
  assert.equal(store.getSnapshot().ring.cards.length, 0);
  assert.equal(store.getSnapshot().pending, 0);
  store.intake(textDelta("b/glitch.ts", { repoId: "b" }));
  timers.run();
  assert.equal(store.getSnapshot().ring.cards.length, 1);
  store.dispose();
});

void test("no public seed route exists beside intake, so no caller can hand a store rows without a repoId", () => {
  const store = createStreamStore("b", fakeTimers());
  assert.equal("seedHistory" in store, false);
  assert.equal(Object.keys(store).some((key) => key.toLowerCase().includes("seed")), false);
  store.dispose();
});

void test("a snapshot never reseeds a ring that already holds a card", () => {
  resetSeq();
  const timers = fakeTimers();
  const store = createStreamStore("b", timers);
  store.intake(textDelta("b/glitch.ts", { repoId: "b" }));
  timers.run();
  assert.equal(store.getSnapshot().ring.cards.length, 1);
  store.intake(snapshot("b", ROWS_B));
  timers.run();
  assert.equal(store.getSnapshot().ring.cards.length, 1);
  assert.deepEqual(historyPaths(store), []);
  store.dispose();
});

void test("the registry routes a snapshot to the store whose repoId matches and to no other", () => {
  const timers = fakeTimers();
  const registry = createStreamStoreRegistry((repoId) => createStreamStore(repoId, timers));
  const a = registry.getOrCreate("a", streamRootIdentity("a", { kind: "host", path: "/repo/a" }));
  const b = registry.getOrCreate("b", streamRootIdentity("b", { kind: "host", path: "/repo/b" }));
  registry.routeAgentEvent(snapshot("a", ROWS_A));
  timers.run();
  assert.deepEqual(historyPaths(a), ["a/stream_store_test.ts", "a/stream_panel_design.md"]);
  assert.deepEqual(historyPaths(b), []);
  registry.routeAgentEvent(snapshot("c", ["c/orphan.ts"]));
  timers.run();
  assert.equal(registry.get("c"), undefined);
  assert.deepEqual(historyPaths(b), []);
  registry.disposeAll();
});

void test("a replacement store survives later configuration reconciliation while labels preserve identity", () => {
  const timers = fakeTimers();
  const disposed = new Set<StreamStore>();
  const registry = createStreamStoreRegistry((repoId) => {
    const store = createStreamStore(repoId, timers);
    const dispose = store.dispose;
    store.dispose = () => { disposed.add(store); dispose(); };
    return store;
  });
  const original = { repoId: "a", root: { kind: "host" as const, path: "/repo/a" }, label: "Original" };
  registry.setRepositories([original]);
  const old = registry.getOrCreate("a", streamRootIdentity("a", original.root));
  const replacement = { ...original, root: { ...original.root, path: "/repo/replacement" } };
  const next = registry.getOrCreate("a", streamRootIdentity("a", replacement.root));
  assert.notEqual(next, old);
  assert.equal(disposed.has(old), true);
  registry.setRepositories([replacement]);
  assert.equal(registry.get("a"), next);
  assert.equal(disposed.has(next), false);
  const renamed = { ...replacement, label: "Renamed" };
  registry.setRepositories([renamed]);
  assert.equal(registry.getOrCreate("a", streamRootIdentity("a", renamed.root)), next);
  assert.equal(disposed.has(next), false);
  registry.routeAgentEvent({ ...snapshot("a", ROWS_A), rootPath: replacement.root.path });
  timers.run();
  assert.deepEqual(historyPaths(next), ROWS_A);
  registry.setRepositories([]);
  assert.equal(registry.get("a"), undefined);
  assert.equal(disposed.has(next), true);
  registry.disposeAll();
});
