// Path: app/src/components/overview/overview_feed_entries_test.ts
// Description: Loss notices sit where the loss happened and age out with the working window

import { test } from "node:test";
import assert from "node:assert/strict";
import { RepoConfigSchema } from "../../shared/config/repo_config.js";
import { OVERVIEW_WORKING_MS, type OverviewRepoSnapshot, type OverviewSnapshot } from "../../lib/overview/overview_types.js";
import { overviewFeedEntries } from "./overview_feed_entries.js";

const TIME = Date.parse("2026-10-11T12:00:00Z");
const REPO = RepoConfigSchema.parse({ repoId: "r", label: "r", root: { kind: "wsl", path: "/r" } });

function snapshot(losses: OverviewRepoSnapshot["losses"]): OverviewSnapshot {
  const repo: OverviewRepoSnapshot = {
    repoId: "r", root: { kind: "wsl", path: "/r" }, availability: "watching", issue: null,
    recent: [], edits: [], lastChangedAt: null, lastWorkedAt: TIME + 60_000, losses,
  };
  return { repos: new Map([["r", repo]]), revision: 1 };
}

void test("a loss notice keeps its own time and leaves with the working window", () => {
  const view = snapshot([{ id: 3, at: TIME, count: 2 }]);
  const [notice] = overviewFeedEntries([REPO], view, TIME + 60_000);
  if (notice?.kind !== "notice") throw new Error("expected a loss notice");
  assert.equal(notice.at, TIME);
  assert.equal(notice.message, "2 edits not shown");
  assert.equal(overviewFeedEntries([REPO], view, TIME + OVERVIEW_WORKING_MS).length, 0);
});
