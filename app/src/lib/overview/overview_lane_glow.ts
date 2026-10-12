// Path: app/src/lib/overview/overview_lane_glow.ts
// Description: Git status and observed turns jointly determine a lane's visual signal

import type { StatusSlotView } from "../source_control/status_registry_types.js";
import { isCleanStatus } from "../source_control/status_groups.js";
import { recentlyFinished, type Lane } from "./overview_lanes.js";
import type { OverviewAvailability } from "./overview_types.js";

export type LaneGlow = "working" | "finished" | null;

export function laneGlow(lane: Lane | undefined, slot: StatusSlotView, availability: OverviewAvailability, now: number): LaneGlow {
  if (!lane || availability !== "watching" || !slot.status || slot.phase.kind === "error" || slot.phase.kind === "waiting") return null;
  if (!isCleanStatus(slot.status)) return "working";
  if (slot.phase.kind !== "ready" || slot.status.truncated || slot.status.mutationInProgress) return null;
  return recentlyFinished(lane, now) ? "finished" : null;
}
