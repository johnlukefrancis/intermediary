// Path: app/src/hooks/overview/use_overview_git_totals.ts
// Description: Changed-file and conflict totals across shown repositories, read from the shared status registry

import { useCallback, useSyncExternalStore } from "react";
import type { RepoConfig } from "../../shared/config.js";
import type { SourceControlStatus } from "../../shared/protocol.js";
import { statusRegistry } from "../../lib/source_control/status_registry.js";
import { totalConflictCount } from "../../lib/source_control/conflict_count.js";
import { countChangedPaths } from "../source_control/source_control_counts.js";

function useRegistrySum(repoIdsKey: string, measure: (status: SourceControlStatus) => number): number {
  const subscribe = useCallback((listener: () => void) => {
    const stops = repoIdsKey.split("\0").map((repoId) => statusRegistry.subscribe(repoId, listener));
    return () => { for (const stop of stops) stop(); };
  }, [repoIdsKey]);
  const getSnapshot = useCallback(() => {
    let total = 0;
    for (const repoId of repoIdsKey.split("\0")) {
      const status = statusRegistry.getSlot(repoId).status;
      if (status !== null) total += measure(status);
    }
    return total;
  }, [measure, repoIdsKey]);
  return useSyncExternalStore(subscribe, getSnapshot);
}

export function useOverviewGitTotals(shown: readonly RepoConfig[]): { changes: number; conflicts: number } {
  const key = shown.map((repo) => repo.repoId).join("\0");
  return { changes: useRegistrySum(key, countChangedPaths), conflicts: useRegistrySum(key, totalConflictCount) };
}
