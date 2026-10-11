// Path: app/src/hooks/overview/use_dirty_repo_ids.ts
// Description: The shown repositories whose working tree is not known clean, read from the shared status registry

import { useCallback, useMemo, useSyncExternalStore } from "react";
import type { RepoConfig } from "../../shared/config.js";
import { statusRegistry } from "../../lib/source_control/status_registry.js";
import { isCleanStatus } from "../../lib/source_control/status_groups.js";

export function useDirtyRepoIds(repos: readonly RepoConfig[]): ReadonlySet<string> {
  const key = repos.map((repo) => repo.repoId).join("\0");
  const subscribe = useCallback((listener: () => void) => {
    const stops = key.split("\0").filter((repoId) => repoId.length > 0).map((repoId) => statusRegistry.subscribe(repoId, listener));
    return () => { for (const stop of stops) stop(); };
  }, [key]);
  const getSnapshot = useCallback(() => key.split("\0")
    .filter((repoId) => repoId.length > 0 && !isCleanStatus(statusRegistry.getSlot(repoId).status)).join("\0"), [key]);
  const dirtyKey = useSyncExternalStore(subscribe, getSnapshot);
  return useMemo(() => new Set(dirtyKey.split("\0").filter((repoId) => repoId.length > 0)), [dirtyKey]);
}
