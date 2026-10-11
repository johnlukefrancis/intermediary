// Path: app/src/lib/source_control/status_registry_test.ts
// Description: Status registry rules: read cap, release/dispose, stale generation, demanded routing, writer, hidden defer, WSL hold, root change, re-hello

import { test } from "node:test";
import assert from "node:assert/strict";
import type { RepoRoot } from "../../shared/config.js";
import type { AgentEvent, SourceControlOmitted, SourceControlStatus } from "../../shared/protocol.js";
import { createStatusRegistry } from "./status_registry.js";
import type { StatusHostState, StatusRegistry } from "./status_registry_types.js";

interface PendingRead {
  repoId: string;
  resolve(status: SourceControlStatus): void;
  reject(error: Error): void;
}

interface Harness {
  registry: StatusRegistry<string>;
  reads: PendingRead[];
  focus: () => void;
  focusListeners: () => number;
}

function harness(): Harness {
  const reads: PendingRead[] = [];
  const focusListeners = new Set<() => void>();
  const registry = createStatusRegistry<string>({
    read: (_client, repoId) =>
      new Promise<SourceControlStatus>((resolve, reject) => {
        reads.push({ repoId, resolve, reject });
      }),
    listenFocus(onFocus) {
      focusListeners.add(onFocus);
      return () => {
        focusListeners.delete(onFocus);
      };
    },
  });
  return {
    registry,
    reads,
    focus: () => {
      for (const listener of focusListeners) listener();
    },
    focusListeners: () => focusListeners.size,
  };
}

function host(overrides: Partial<StatusHostState<string>> = {}): StatusHostState<string> {
  return { client: "c", ready: true, documentHidden: false, wslOnline: true, rootKinds: new Map(), ...overrides };
}

const OMITTED: SourceControlOmitted = { stagedOutsideRoot: 0, unmergedOutsideRoot: 0, unrepresentablePath: 0 };

function status(headSha: string): SourceControlStatus {
  return {
    branch: "main", headSha, detached: false, upstream: null, ahead: null, behind: null,
    index: [], worktree: [], conflicts: [], omitted: OMITTED, indexTreeSha: "t", snapshotId: "s",
    mutationInProgress: false, committable: false, truncated: false, capturedAtIso: "2026-01-01T00:00:00Z",
  };
}

async function settle(): Promise<void> {
  for (let i = 0; i < 5; i += 1) await Promise.resolve();
}

const changed = (repoId: string): AgentEvent => ({ type: "sourceControlChanged", repoId });

void test("at most two reads run at once and the rest wait their turn", async () => {
  const { registry, reads } = harness();
  registry.setHostState(host());
  for (const id of ["a", "b", "c", "d"]) registry.retain(id);
  assert.deepEqual(reads.map((read) => read.repoId), ["a", "b"]);
  assert.equal(registry.readsInFlight(), 2);
  reads[0]?.resolve(status("1"));
  await settle();
  assert.deepEqual(reads.map((read) => read.repoId), ["a", "b", "c"]);
  assert.equal(registry.getSlot("a").status?.headSha, "1");
});

void test("the last release cancels a queued read and keeps the status", async () => {
  const { registry, reads, focusListeners } = harness();
  registry.setHostState(host());
  registry.retain("a");
  registry.retain("b");
  const releaseC = registry.retain("c");
  assert.equal(focusListeners(), 1);
  releaseC();
  reads[0]?.resolve(status("1"));
  reads[1]?.resolve(status("2"));
  await settle();
  assert.deepEqual(reads.map((read) => read.repoId), ["a", "b"]);
  assert.equal(registry.getSlot("b").phase.kind, "ready");
});

void test("a read superseded by an applied status is dropped", async () => {
  const { registry, reads } = harness();
  registry.setHostState(host());
  registry.retain("a");
  registry.applyStatus("a", status("fresh"));
  reads[0]?.resolve(status("stale"));
  await settle();
  assert.equal(registry.getSlot("a").status?.headSha, "fresh");
});

void test("change events reach only demanded slots and focus skips a pending action", async () => {
  const { registry, reads, focus } = harness();
  registry.setHostState(host());
  const release = registry.retain("a");
  reads[0]?.resolve(status("1"));
  await settle();
  release();
  registry.routeAgentEvent(changed("a"));
  registry.retain("b");
  reads[1]?.resolve(status("2"));
  await settle();
  registry.actionStarted("b");
  focus();
  assert.deepEqual(reads.map((read) => read.repoId), ["a", "b"]);
  registry.actionFinished("b", null);
  focus();
  assert.deepEqual(reads.map((read) => read.repoId), ["a", "b", "b"]);
});

void test("an attached writer sees every read and may absorb a failure", async () => {
  const { registry, reads } = harness();
  const seen: string[] = [];
  registry.setHostState(host());
  registry.retain("a");
  registry.attachWriter("a", {
    acceptStatus: (next) => { seen.push(next.headSha ?? "none"); },
    acceptFailure: () => true,
  });
  reads[0]?.resolve(status("1"));
  await settle();
  registry.refresh("a");
  reads[1]?.reject(new Error("boom"));
  await settle();
  assert.deepEqual(seen, ["1"]);
  assert.equal(registry.getSlot("a").status?.headSha, "1");
});

void test("a hidden document defers the read until it is visible again", async () => {
  const { registry, reads } = harness();
  registry.setHostState(host({ documentHidden: true }));
  registry.retain("a");
  await settle();
  assert.equal(reads.length, 0);
  registry.setHostState(host());
  assert.equal(reads.length, 1);
});

void test("a WSL root does not read while the backend is offline", async () => {
  const { registry, reads } = harness();
  const rootKinds = new Map<string, RepoRoot["kind"]>([["w", "wsl"]]);
  registry.setHostState(host({ wslOnline: false, rootKinds }));
  registry.retain("w");
  registry.retain("h");
  await settle();
  assert.deepEqual(reads.map((read) => read.repoId), ["h"]);
  assert.equal(registry.getSlot("w").phase.kind, "waiting");
  registry.setHostState(host({ rootKinds }));
  registry.markRehydrated();
  assert.deepEqual(reads.map((read) => read.repoId), ["h", "w"]);
});

void test("a root change drops the old status and rereads a demanded slot", async () => {
  const { registry, reads } = harness();
  const root = (path: string): RepoRoot => ({ kind: "host", path });
  registry.setRepositories([{ repoId: "a", root: root("/one") }]);
  registry.setHostState(host());
  registry.retain("a");
  reads[0]?.resolve(status("1"));
  await settle();
  registry.setRepositories([{ repoId: "a", root: root("/two") }]);
  assert.equal(registry.getSlot("a").status, null);
  assert.equal(reads.length, 2);
});

void test("a re-hello rereads each demanded slot exactly once", async () => {
  const { registry, reads } = harness();
  registry.setHostState(host());
  registry.retain("a");
  registry.retain("b");
  reads[0]?.resolve(status("1"));
  reads[1]?.resolve(status("1"));
  await settle();
  registry.setHostState(host({ ready: false }));
  registry.setHostState(host());
  await settle();
  assert.deepEqual(reads.slice(2).map((read) => read.repoId).sort(), ["a", "b"]);
});
