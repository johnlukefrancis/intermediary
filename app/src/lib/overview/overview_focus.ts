// Path: app/src/lib/overview/overview_focus.ts
// Description: Recent-work membership from observed edits, independent of watch health and open tabs

import { OVERVIEW_WORKING_MS, type OverviewRepoSnapshot } from "./overview_types.js";

export function lastWorkAt(repo: OverviewRepoSnapshot | undefined): number | null {
  return repo?.lastWorkedAt ?? null;
}

export function isWorking(repo: OverviewRepoSnapshot | undefined, now: number): boolean {
  const at = lastWorkAt(repo);
  return at !== null && at <= now && now - at < OVERVIEW_WORKING_MS;
}
