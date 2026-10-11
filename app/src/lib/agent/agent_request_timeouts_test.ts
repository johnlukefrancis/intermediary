// Path: app/src/lib/agent/agent_request_timeouts_test.ts
// Description: Every UI request timeout exceeds its host->WSL forward budget by a fixed margin

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";
import {
  COMMAND_TIMEOUT_MS,
  SOURCE_CONTROL_ACTION_TIMEOUT_MS,
  WORKTREE_ACTION_TIMEOUT_MS,
} from "./agent_request_timeouts.js";

const HOST_TIMEOUTS_PATH = "crates/im_host_agent/src/wsl/wsl_backend_client/timeouts.rs";
const MIN_MARGIN_MS = 15_000;

// Independent oracle of host `timeout_for_command`, keyed by the same UI tables.
const HOST_TIER_FOR_COMMAND: Readonly<Record<keyof typeof COMMAND_TIMEOUT_MS, string>> = {
  watchRepo: "DEFAULT",
  refresh: "DEFAULT",
  stageFile: "DEFAULT",
  readTextFile: "DEFAULT",
  readImageFile: "DEFAULT",
  buildBundle: "BUILD_BUNDLE",
  inspectBundleEntry: "DEFAULT",
  cancelBundleBuild: "DEFAULT",
  clientHello: "CLIENT_HELLO",
  setOptions: "DEFAULT",
  getRepoTopLevel: "DEFAULT",
  listRepoDirectory: "DEFAULT",
  listBundles: "DEFAULT",
  getTrFleetStatus: "DEFAULT",
  trFleetAction: "DEFAULT",
  sourceControlStatus: "SOURCE_CONTROL_READ",
  sourceControlDiff: "SOURCE_CONTROL_READ",
  sourceControlImageDiff: "SOURCE_CONTROL_READ",
  importFiles: "IMPORT_FILES",
};
const HOST_TIER_FOR_SOURCE_CONTROL_ACTION: Readonly<
  Record<keyof typeof SOURCE_CONTROL_ACTION_TIMEOUT_MS, string>
> = {
  stage: "SOURCE_CONTROL_INDEX",
  unstage: "SOURCE_CONTROL_INDEX",
  discard: "SOURCE_CONTROL_DISCARD",
  commit: "SOURCE_CONTROL_COMMIT",
  push: "SOURCE_CONTROL_REMOTE",
  pull: "SOURCE_CONTROL_REMOTE",
};
const HOST_TIER_FOR_WORKTREE_ACTION: Readonly<
  Record<keyof typeof WORKTREE_ACTION_TIMEOUT_MS, string>
> = {
  delete: "WORKTREE_ENTRY",
  move: "WORKTREE_ENTRY",
  rename: "WORKTREE_ENTRY",
  copy: "IMPORT_FILES",
};

function readHostBudgetsMs(): Map<string, number> {
  const source = readFileSync(path.join(process.cwd(), HOST_TIMEOUTS_PATH), "utf8");
  const pattern = /FORWARD_REQUEST_TIMEOUT_(\w+): Duration =\s*Duration::from_secs\(([\d\s*]+)\)/g;
  const budgets = new Map<string, number>();
  for (const [, tier, expression] of source.matchAll(pattern)) {
    if (tier === undefined || expression === undefined) continue;
    const seconds = expression.split("*").reduce((product, factor) => product * Number(factor.trim()), 1);
    budgets.set(tier, seconds * 1000);
  }
  return budgets;
}

function assertLadder(
  uiTable: Readonly<Record<string, number>>,
  hostTiers: Readonly<Record<string, string>>,
  budgets: Map<string, number>,
): void {
  for (const [key, uiMs] of Object.entries(uiTable)) {
    const tier = hostTiers[key];
    assert.ok(tier !== undefined, `no host tier mapped for ${key}`);
    const hostMs = budgets.get(tier);
    assert.ok(hostMs !== undefined, `host tier ${tier} for ${key} not found in ${HOST_TIMEOUTS_PATH}`);
    assert.ok(
      uiMs >= hostMs + MIN_MARGIN_MS,
      `${key}: UI ${uiMs} ms must exceed host ${tier} ${hostMs} ms by ${MIN_MARGIN_MS} ms`,
    );
  }
}

void test("every UI request timeout exceeds its host forward budget plus margin", () => {
  const budgets = readHostBudgetsMs();
  assert.equal(budgets.get("DEFAULT"), 60_000, "host default tier parsed");
  assertLadder(COMMAND_TIMEOUT_MS, HOST_TIER_FOR_COMMAND, budgets);
  assertLadder(SOURCE_CONTROL_ACTION_TIMEOUT_MS, HOST_TIER_FOR_SOURCE_CONTROL_ACTION, budgets);
  assertLadder(WORKTREE_ACTION_TIMEOUT_MS, HOST_TIER_FOR_WORKTREE_ACTION, budgets);
});
