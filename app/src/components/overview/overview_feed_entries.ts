// Path: app/src/components/overview/overview_feed_entries.ts
// Description: Bounded chronological Overview edit, observation and coverage entries

import type { RepoConfig } from "../../shared/config.js";
import { OVERVIEW_WORKING_MS, type OverviewEdit, type OverviewSignal, type OverviewSnapshot } from "../../lib/overview/overview_types.js";
import { AVAILABILITY_LABELS } from "./overview_presentation.js";

interface EntryIdentity { key: string; repo: RepoConfig; at: number; order: number }
export type OverviewFeedEntry = EntryIdentity & (
  { kind: "edit"; edit: OverviewEdit }
  | { kind: "signal"; signal: OverviewSignal }
  | { kind: "notice"; message: string; detail: string }
);

export function overviewFeedEntries(repos: readonly RepoConfig[], snapshot: OverviewSnapshot, now: number): OverviewFeedEntry[] {
  const entries: OverviewFeedEntry[] = [];
  const recent = (at: number): boolean => at <= now && now - at < OVERVIEW_WORKING_MS;
  for (const repo of repos) {
    const state = snapshot.repos.get(repo.repoId);
    if (state === undefined) continue;
    const newestPreview = new Map<string, number>();
    for (const edit of state.edits) {
      if (!recent(edit.at)) continue;
      newestPreview.set(edit.path, Math.max(newestPreview.get(edit.path) ?? edit.at, edit.at));
      entries.push({ key: `edit:${String(edit.id)}`, repo, at: edit.at, order: edit.id, kind: "edit", edit });
    }
    for (const signal of state.recent) {
      const previewAt = newestPreview.get(signal.path);
      if (!recent(signal.at) || (previewAt !== undefined && previewAt >= signal.at)) continue;
      entries.push({ key: `${repo.repoId}:${signal.source}:${signal.path}:${String(signal.at)}`, repo,
        at: signal.at, order: 0, kind: "signal", signal });
    }
    const incomplete = state.coverageInterrupted || state.missedDetails > 0;
    const unavailable = state.availability !== "watching" && state.availability !== "connecting";
    if (incomplete || state.issue !== null || unavailable) {
      const availability = unavailable ? AVAILABILITY_LABELS[state.availability] : null;
      entries.push({ key: `${repo.repoId}:notice`, repo, at: state.lastWorkedAt ?? 0, order: Number.MAX_SAFE_INTEGER,
        kind: "notice", message: state.issue !== null ? "Monitoring issue" : availability ?? "Activity detail is incomplete",
        detail: [state.issue, availability, incomplete ? `Activity detail is incomplete: ${String(state.missedDetails)} detail-loss signals${state.coverageInterrupted ? "; observation coverage interrupted" : ""}` : null]
          .filter((text) => text !== null).join(" · ") });
    }
  }
  return entries.sort((left, right) => left.at - right.at || left.order - right.order).slice(-80);
}
