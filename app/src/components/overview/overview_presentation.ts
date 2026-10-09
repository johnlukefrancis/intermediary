// Path: app/src/components/overview/overview_presentation.ts
// Description: Overview watch-health summaries and relative edit ages

import type { RepoConfig } from "../../shared/config.js";
import type { OverviewAvailability, OverviewSnapshot } from "../../lib/overview/overview_types.js";

export const AVAILABILITY_LABELS: Readonly<Record<OverviewAvailability, string>> = {
  connecting: "Connecting", watching: "Watching", offline: "Agent disconnected", held: "WSL held",
  unwatched: "Watch unavailable", error: "Watch error", "update-required": "Agent update required",
};

export function monitoringSummary(repos: readonly RepoConfig[], snapshot: OverviewSnapshot): {
  issues: number; detail: string; emptyHeading: string; emptyDetail: string;
} {
  let watching = 0;
  let connecting = 0;
  const problems: string[] = [];
  const unavailable = new Set<OverviewAvailability>();
  for (const repo of repos) {
    const state = snapshot.repos.get(repo.repoId);
    const availability = state?.availability ?? "connecting";
    if (availability === "watching") watching += 1;
    else if (availability === "connecting") connecting += 1;
    else unavailable.add(availability);
    if ((availability !== "watching" && availability !== "connecting") || state?.issue) {
      problems.push(`${repo.label}: ${state?.issue ?? AVAILABILITY_LABELS[availability]}`);
    }
  }
  const live = watching > 0;
  const waiting = live || repos.length === 0;
  const onlyConnecting = !live && unavailable.size === 0 && connecting > 0;
  const emptyHeading = waiting ? "Waiting for activity" : onlyConnecting ? "Connecting to repositories"
    : unavailable.size === 1 && unavailable.has("offline") ? "Agent disconnected"
      : unavailable.size === 1 && unavailable.has("held") ? "WSL monitoring is held" : "Repository watches unavailable";
  return {
    issues: problems.length, detail: problems.join("\n"), emptyHeading,
    emptyDetail: waiting ? "Repositories appear as files change and leave after 15 minutes of quiet."
      : onlyConnecting ? "Waiting for repository watches to connect." : "Activity appears when repository watches reconnect.",
  };
}

export function relativeAge(at: number, now: number): string {
  const seconds = Math.max(0, Math.floor((now - at) / 1000));
  if (seconds < 5) return "just now";
  if (seconds < 60) return `${String(seconds)}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${String(minutes)}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${String(hours)}h ago`;
  return `${String(Math.floor(hours / 24))}d ago`;
}
