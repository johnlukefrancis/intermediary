// Path: app/src/hooks/overview/use_overview_git_states.ts
// Description: One shared-status subscription projects visible Git lanes and their turn glow

import { useCallback, useMemo, useSyncExternalStore } from "react";
import type { RepoConfig } from "../../shared/config.js";
import { statusRegistry } from "../../lib/source_control/status_registry.js";
import { isCleanStatus } from "../../lib/source_control/status_groups.js";
import { laneGlow, type LaneGlow } from "../../lib/overview/overview_lane_glow.js";
import type { Lane } from "../../lib/overview/overview_lanes.js";
import type { OverviewSnapshot } from "../../lib/overview/overview_types.js";

export interface GitLaneState { dirty: boolean; glow: LaneGlow }

export function useOverviewGitStates(repos: readonly RepoConfig[], lanes: ReadonlyMap<string, Lane>, overview: OverviewSnapshot, now: number): ReadonlyMap<string, GitLaneState> {
  const key = repos.map(repo => repo.repoId).join("\0");
  const ids = useMemo(() => key.split("\0").filter(Boolean), [key]);
  const subscribe = useCallback((listener: () => void) => {
    const stops = ids.map(repoId => statusRegistry.subscribe(repoId, listener));
    return () => { for (const stop of stops) stop(); };
  }, [ids]);
  const getSnapshot = useCallback(() => ids.map(repoId => {
    const slot = statusRegistry.getSlot(repoId);
    const glow = laneGlow(lanes.get(repoId), slot, overview.repos.get(repoId)?.availability ?? "connecting", now);
    return `${!isCleanStatus(slot.status) ? "1" : "0"}:${glow ?? ""}`;
  }).join("\0"), [ids, lanes, overview, now]);
  const states = useSyncExternalStore(subscribe, getSnapshot);
  return useMemo(() => {
    const encoded = states.split("\0");
    return new Map(ids.map((repoId, index) => {
      const [dirty, glow] = encoded[index]?.split(":") ?? [];
      return [repoId, { dirty: dirty === "1", glow: glow === "working" || glow === "finished" ? glow : null }];
    }));
  }, [ids, states]);
}
