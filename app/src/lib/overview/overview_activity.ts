// Path: app/src/lib/overview/overview_activity.ts
// Description: Bounded live observations, historical paths and settled-detail sequence accounting

import type { RepoRoot } from "../../shared/config.js";
import type { AgentEvent, FileDeltaCountersEvent, FileDeltaEvent } from "../../shared/protocol.js";
import { isVisibleFileKind } from "../files/file_feed.js";
import {
  OVERVIEW_EDITS_MAX, OVERVIEW_RECENT_MAX, type OverviewEdit, type OverviewLoss, type OverviewSignal,
} from "./overview_types.js";

export interface OverviewActivity {
  repoId: string;
  root: RepoRoot;
  recent: OverviewSignal[];
  edits: OverviewEdit[];
  lastChangedAt: number | null;
  lastWorkedAt: number | null;
  losses: OverviewLoss[];
  lastSeq: number | null;
  confirmed: boolean;
  watcherIssue: string | null;
  refreshIssue: string | null;
  refreshUnavailable: boolean;
  rehydrating: boolean;
  recoveryPending: boolean;
}

export function createActivity(repoId: string, root: RepoRoot): OverviewActivity {
  return {
    repoId, root: { ...root }, recent: [], edits: [],
    lastChangedAt: null, lastWorkedAt: null, losses: [], lastSeq: null, confirmed: false,
    watcherIssue: null, refreshIssue: null, refreshUnavailable: false, rehydrating: false, recoveryPending: false,
  };
}

export function observeChange(
  activity: OverviewActivity, event: Extract<AgentEvent, { type: "fileChanged" }>, now: number,
): void {
  if (!isVisibleFileKind(event.kind)) return;
  const previous = activity.recent.find((signal) => signal.path === event.path);
  const previousIndex = activity.recent.findIndex((signal) => signal.path === event.path);
  if (previousIndex >= 0) activity.recent.splice(previousIndex, 1);
  activity.recent.unshift({
    path: event.path, kind: event.kind, changeType: event.changeType, at: now,
    source: "live", count: previous?.source === "live" ? previous.count + 1 : 1,
  });
  activity.recent.length = Math.min(activity.recent.length, OVERVIEW_RECENT_MAX);
  activity.lastChangedAt = now;
  activity.lastWorkedAt = now;
}

export function seedHistory(activity: OverviewActivity, event: Extract<AgentEvent, { type: "snapshot" }>, now: number): void {
  for (const file of event.recent) {
    if (!isVisibleFileKind(file.kind)) continue;
    const observedAt = Date.parse(file.activity?.lastSeenAtIso ?? "");
    if (Number.isFinite(observedAt) && observedAt <= now) {
      activity.lastWorkedAt = Math.max(activity.lastWorkedAt ?? observedAt, observedAt);
    }
    const at = Date.parse(file.activity?.lastSeenAtIso ?? file.mtime);
    if (!Number.isFinite(at) || at > now) continue;
    const existing = activity.recent.find((signal) => signal.path === file.path);
    if (existing !== undefined && existing.at >= at) continue;
    activity.recent = activity.recent.filter((signal) => signal.path !== file.path);
    activity.recent.push({
      path: file.path, kind: file.kind, changeType: file.changeType,
      at, source: "history", count: 0,
    });
    activity.recent.sort((a, b) => b.at - a.at);
    activity.recent.length = Math.min(activity.recent.length, OVERVIEW_RECENT_MAX);
  }
}

/** Stream's rule: a restart is a new stream; a forward gap plus the agent's counters are edits not shown.
 * Returns that count, or null for a repeated sequence number. */
export function acceptDetails(activity: OverviewActivity, event: FileDeltaEvent | FileDeltaCountersEvent): number | null {
  const last = activity.lastSeq;
  if (last === event.seq) return null;
  activity.lastSeq = event.seq;
  const gap = last !== null && event.seq > last ? event.seq - last - 1 : 0;
  return gap + event.withheld + event.dropped;
}

/** Losses with no edit between them share one mark, which moves to the newest loss.
 * Returns whether `id` (from the edit counter) was spent on a new mark. */
export function noteLoss(activity: OverviewActivity, count: number, at: number, id: number): boolean {
  if (count <= 0) return false;
  const latest = activity.losses.at(-1);
  const lastEditId = activity.edits.at(-1)?.id ?? 0;
  if (latest !== undefined && latest.id > lastEditId) {
    latest.count += count;
    latest.at = Math.max(latest.at, at);
    return false;
  }
  activity.losses.push({ id, at, count });
  if (activity.losses.length > OVERVIEW_EDITS_MAX) activity.losses.shift();
  return true;
}

/** Stream's rule: a modify whose settled text gained and lost nothing is no news, so its raw signal goes too */
export function isUnchangedEdit(event: FileDeltaEvent): boolean {
  return event.op === "modify" && event.payload.kind === "text"
    && event.payload.stats.added + event.payload.stats.removed === 0;
}

export function retractLiveSignal(activity: OverviewActivity, path: string): void {
  activity.recent = activity.recent.filter((signal) => signal.source !== "live" || signal.path !== path);
}
