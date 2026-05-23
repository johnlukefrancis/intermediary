// Path: app/src/lib/files/file_feed.ts
// Description: File feed filtering and activity ranking helpers

import type { FileActivity, FileEntry, FileKind } from "../../shared/protocol.js";

export type VisibleFileKind = "docs" | "code" | "image";
export type FileTypeFilter = "all" | VisibleFileKind;
export type FileActivityBadge = "hot" | "rising";

export interface FeedFileEntry extends FileEntry {
  activityScore: number;
  activityBadge: FileActivityBadge | null;
}

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;
const RISING_WINDOW_MS = DAY_MS;
const HOT_WINDOW_MS = 2 * HOUR_MS;

export function isVisibleFileKind(kind: FileKind): kind is VisibleFileKind {
  return kind === "docs" || kind === "code" || kind === "image";
}

export function filterFeedFiles(
  files: readonly FileEntry[],
  filter: FileTypeFilter
): FileEntry[] {
  return files.filter((file) => isVisibleFileKind(file.kind) && (
    filter === "all" || file.kind === filter
  ));
}

export function sortLatestFeed(files: readonly FileEntry[], nowMs = Date.now()): FeedFileEntry[] {
  return decorateFiles(files, nowMs).sort((a, b) => compareNewest(a, b));
}

export function sortActiveFeed(files: readonly FileEntry[], nowMs = Date.now()): FeedFileEntry[] {
  return decorateFiles(files, nowMs).sort((a, b) => {
    const scoreDiff = b.activityScore - a.activityScore;
    if (scoreDiff !== 0) return scoreDiff;
    return compareNewest(a, b);
  });
}

function decorateFiles(files: readonly FileEntry[], nowMs: number): FeedFileEntry[] {
  return files.map((file) => {
    const activity = normalizeActivity(file);
    const activityScore = scoreActivity(activity, nowMs);
    return {
      ...file,
      activityScore,
      activityBadge: badgeActivity(activity, nowMs),
    };
  });
}

function normalizeActivity(file: FileEntry): FileActivity {
  if (file.activity) return file.activity;
  const fallbackIso = file.mtime || new Date(0).toISOString();
  return {
    firstSeenAtIso: fallbackIso,
    lastSeenAtIso: fallbackIso,
    updateCount: 1,
    burstCount: 1,
  };
}

function scoreActivity(activity: FileActivity, nowMs: number): number {
  const lastSeenMs = parseIsoMs(activity.lastSeenAtIso);
  const firstSeenMs = parseIsoMs(activity.firstSeenAtIso);
  const ageHours = Math.max(0, (nowMs - lastSeenMs) / HOUR_MS);
  const recencyScore = Math.exp(-ageHours / 72) * 50;
  const frequencyScore = Math.min(activity.updateCount, 25) * 4;
  const burstScore = Math.min(Math.max(0, activity.burstCount - 1), 8) * 8;
  const risingScore =
    nowMs - firstSeenMs <= RISING_WINDOW_MS && activity.updateCount >= 3 ? 35 : 0;

  return recencyScore + frequencyScore + burstScore + risingScore;
}

function badgeActivity(activity: FileActivity, nowMs: number): FileActivityBadge | null {
  const lastSeenMs = parseIsoMs(activity.lastSeenAtIso);
  const firstSeenMs = parseIsoMs(activity.firstSeenAtIso);
  const isFresh = nowMs - lastSeenMs <= HOT_WINDOW_MS;
  if (isFresh && activity.burstCount >= 3) return "hot";
  if (nowMs - firstSeenMs <= RISING_WINDOW_MS && activity.updateCount >= 3) return "rising";
  return null;
}

function compareNewest(a: FileEntry, b: FileEntry): number {
  return parseIsoMs(b.mtime) - parseIsoMs(a.mtime);
}

function parseIsoMs(value: string): number {
  const parsed = new Date(value).getTime();
  return Number.isNaN(parsed) ? 0 : parsed;
}
