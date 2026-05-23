// Path: app/src/lib/files/file_feed.ts
// Description: Auto file feed filtering, ranking, and row metric helpers

import type { FileActivity, FileActivityBucket, FileEntry, FileKind } from "../../shared/protocol.js";

export type VisibleFileKind = "docs" | "code" | "image";
export type FileTypeFilter = "all" | VisibleFileKind;
export type FileSortMode = "auto" | "latest" | "active";
export type FileActivityBadge = "hot" | "rising";
export type FileActivityTrend = "up" | "flat" | "down";

export interface FeedFileEntry extends FileEntry {
  activity: FileActivity;
  activityScore: number;
  recencyScore: number;
  autoScore: number;
  activityBadge: FileActivityBadge | null;
  trend: FileActivityTrend;
  activityBlocks: number;
  pulse: number[];
}

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;
const RISING_WINDOW_MS = DAY_MS;
const HOT_WINDOW_MS = 2 * HOUR_MS;
const ACTIVITY_BLOCKS = 10;
const PULSE_SEGMENTS = 12;
const PULSE_SEGMENT_MS = 2 * HOUR_MS;

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

export function buildAutoFileFeed(
  files: readonly FileEntry[],
  filter: FileTypeFilter,
  sortMode: FileSortMode,
  nowMs = Date.now()
): FeedFileEntry[] {
  return sortAutoFileFeed(filterFeedFiles(files, filter), sortMode, nowMs);
}

export function sortAutoFileFeed(
  files: readonly FileEntry[],
  sortMode: FileSortMode,
  nowMs = Date.now()
): FeedFileEntry[] {
  const decorated = decorateFiles(files, nowMs);
  return decorated.sort((a, b) => compareByMode(a, b, sortMode));
}

function decorateFiles(files: readonly FileEntry[], nowMs: number): FeedFileEntry[] {
  return files.map((file) => {
    const activity = normalizeActivity(file);
    const activityScore = scoreActivity(activity, nowMs);
    return {
      ...file,
      activity,
      activityScore,
      recencyScore: scoreRecency(activity, nowMs),
      autoScore: scoreAuto(activity, nowMs),
      activityBadge: badgeActivity(activity, nowMs),
      trend: trendActivity(activity, nowMs),
      activityBlocks: activityBlockCount(activity, nowMs),
      pulse: pulseSegments(activity, nowMs),
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
    history: [{ bucketStartIso: fallbackIso, count: 1 }],
  };
}

function compareByMode(a: FeedFileEntry, b: FeedFileEntry, sortMode: FileSortMode): number {
  if (sortMode === "latest") return compareNewest(a, b);
  if (sortMode === "active") {
    const activityDiff = b.activityScore - a.activityScore;
    if (activityDiff !== 0) return activityDiff;
    return compareNewest(a, b);
  }

  const autoDiff = b.autoScore - a.autoScore;
  if (autoDiff !== 0) return autoDiff;
  return compareNewest(a, b);
}

function scoreAuto(activity: FileActivity, nowMs: number): number {
  return scoreActivity(activity, nowMs) + scoreRecency(activity, nowMs) * 0.85;
}

function scoreActivity(activity: FileActivity, nowMs: number): number {
  const firstSeenMs = parseIsoMs(activity.firstSeenAtIso);
  const frequencyScore = Math.min(activity.updateCount, 25) * 4;
  const burstScore = Math.min(Math.max(0, activity.burstCount - 1), 8) * 8;
  const risingScore =
    nowMs - firstSeenMs <= RISING_WINDOW_MS && activity.updateCount >= 3 ? 35 : 0;

  return frequencyScore + burstScore + risingScore + recentHistoryCount(activity, nowMs) * 5;
}

function scoreRecency(activity: FileActivity, nowMs: number): number {
  const lastSeenMs = parseIsoMs(activity.lastSeenAtIso);
  const ageHours = Math.max(0, (nowMs - lastSeenMs) / HOUR_MS);
  return Math.exp(-ageHours / 72) * 50;
}

function badgeActivity(activity: FileActivity, nowMs: number): FileActivityBadge | null {
  const lastSeenMs = parseIsoMs(activity.lastSeenAtIso);
  const firstSeenMs = parseIsoMs(activity.firstSeenAtIso);
  const isFresh = nowMs - lastSeenMs <= HOT_WINDOW_MS;
  if (isFresh && activity.burstCount >= 3) return "hot";
  if (nowMs - firstSeenMs <= RISING_WINDOW_MS && activity.updateCount >= 3) return "rising";
  return null;
}

function trendActivity(activity: FileActivity, nowMs: number): FileActivityTrend {
  const recent = bucketCountBetween(activity.history, nowMs - 6 * HOUR_MS, nowMs);
  const previous = bucketCountBetween(activity.history, nowMs - 12 * HOUR_MS, nowMs - 6 * HOUR_MS);
  if (recent > previous) return "up";
  if (previous > recent) return "down";
  return activity.burstCount >= 3 ? "up" : "flat";
}

function activityBlockCount(activity: FileActivity, nowMs: number): number {
  const score = scoreActivity(activity, nowMs);
  return Math.max(0, Math.min(ACTIVITY_BLOCKS, Math.ceil(score / 12)));
}

function pulseSegments(activity: FileActivity, nowMs: number): number[] {
  return Array.from({ length: PULSE_SEGMENTS }, (_, index) => {
    const start = nowMs - (PULSE_SEGMENTS - index) * PULSE_SEGMENT_MS;
    const end = start + PULSE_SEGMENT_MS;
    return bucketCountBetween(activity.history, start, end);
  });
}

function recentHistoryCount(activity: FileActivity, nowMs: number): number {
  return bucketCountBetween(activity.history, nowMs - DAY_MS, nowMs);
}

function bucketCountBetween(
  history: readonly FileActivityBucket[],
  startMs: number,
  endMs: number
): number {
  return history.reduce((total, bucket) => {
    const bucketMs = parseIsoMs(bucket.bucketStartIso);
    if (bucketMs < startMs || bucketMs >= endMs) return total;
    return total + bucket.count;
  }, 0);
}

function compareNewest(a: FileEntry, b: FileEntry): number {
  return parseIsoMs(b.mtime) - parseIsoMs(a.mtime);
}

function parseIsoMs(value: string): number {
  const parsed = new Date(value).getTime();
  return Number.isNaN(parsed) ? 0 : parsed;
}
