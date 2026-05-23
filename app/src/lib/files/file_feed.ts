// Path: app/src/lib/files/file_feed.ts
// Description: Auto file feed filtering, ranking, and row metric helpers

import type { FileActivity, FileActivityBucket, FileEntry, FileKind } from "../../shared/protocol.js";

export type VisibleFileKind = "docs" | "code" | "image";
export type FileTypeFilter = "all" | VisibleFileKind;
export type FileSortMode = "auto" | "latest" | "active";
export type FileActivityBadge = "hot" | "rising";
export type FileActivityTrend = "up" | "flat" | "down";
export type FileActivityGraphBand = "low" | "mid" | "high" | "hot";

export interface FileActivityGraphColumn {
  value: number;
  band: FileActivityGraphBand;
  roughness: number;
}

export interface FeedFileEntry extends FileEntry {
  activity: FileActivity;
  activityScore: number;
  recencyScore: number;
  autoScore: number;
  activityBadge: FileActivityBadge | null;
  trend: FileActivityTrend;
  activityGraph: FileActivityGraphColumn[];
  pulse: number[];
}

const HOUR_MS = 60 * 60 * 1000;
const DAY_MS = 24 * HOUR_MS;
const RISING_WINDOW_MS = DAY_MS;
const HOT_WINDOW_MS = 2 * HOUR_MS;
const ACTIVITY_GRAPH_COLUMNS = 20;
const ACTIVITY_GRAPH_SOFT_CAP = 180;
const ACTIVITY_DECAY_HOURS = 8;
const PULSE_SEGMENTS = 12;
const PULSE_SEGMENT_MS = 2 * HOUR_MS;
const WAVEFORM_VALUE_CEILING = 0.94;

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
    const activityLevel = activityIntensity(activity, nowMs);
    return {
      ...file,
      activity,
      activityScore,
      recencyScore: scoreRecency(activity, nowMs),
      autoScore: scoreAuto(activity, nowMs),
      activityBadge: badgeActivity(activity, nowMs),
      trend: trendActivity(activity, nowMs),
      activityGraph: activityGraphColumns(file.path, activityLevel),
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
  return scoreActivity(activity, nowMs) + scoreRecency(activity, nowMs) * 0.45;
}

function scoreActivity(activity: FileActivity, nowMs: number): number {
  const firstSeenMs = parseIsoMs(activity.firstSeenAtIso);
  const recentScore = Math.log1p(recentWeightedActivity(activity, nowMs)) * 24;
  const burstScore = Math.log1p(Math.min(activity.burstCount, 12)) * 8;
  const risingScore =
    nowMs - firstSeenMs <= RISING_WINDOW_MS && recentHistoryCount(activity, nowMs) >= 3 ? 8 : 0;

  return recentScore + burstScore + risingScore;
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

function activityIntensity(activity: FileActivity, nowMs: number): number {
  const weighted = recentWeightedActivity(activity, nowMs);
  return clamp(Math.log1p(weighted) / Math.log1p(ACTIVITY_GRAPH_SOFT_CAP));
}

function recentWeightedActivity(activity: FileActivity, nowMs: number): number {
  return activity.history.reduce((total, bucket) => {
    const bucketMs = parseIsoMs(bucket.bucketStartIso);
    if (bucketMs < nowMs - DAY_MS || bucketMs > nowMs) return total;
    const ageHours = Math.max(0, (nowMs - bucketMs) / HOUR_MS);
    return total + bucket.count * Math.exp(-ageHours / ACTIVITY_DECAY_HOURS);
  }, 0);
}

function activityGraphColumns(path: string, activityLevel: number): FileActivityGraphColumn[] {
  const phase = deterministicUnit(`${path}:wave-phase`) * Math.PI * 2;
  const reveal = clamp(0.06 + activityLevel * 0.94);
  return Array.from({ length: ACTIVITY_GRAPH_COLUMNS }, (_, index) => {
    const progress = (index + 0.5) / ACTIVITY_GRAPH_COLUMNS;
    const curve = 0.16 + Math.pow(progress, 1.28) * 0.74;
    const wave = Math.sin(progress * Math.PI * 4.2 + phase) * 0.08;
    const grain = (deterministicUnit(`${path}:grain:${index}`) - 0.5) * 0.22;
    const roughness = deterministicUnit(`${path}:rough:${index}`);
    const edge = (roughness - 0.5) * 0.16;
    const revealEdge = reveal + (roughness - 0.5) * 0.07;
    const isRevealed = progress <= revealEdge;
    const noise = (wave + grain + edge) * 0.58;
    const crest = clamp(Math.min(WAVEFORM_VALUE_CEILING, curve + noise));
    const value = isRevealed ? Math.max(0.12, crest) : 0;
    return { value, band: graphBand(index), roughness };
  });
}

function graphBand(index: number): FileActivityGraphBand {
  const progress = (index + 1) / ACTIVITY_GRAPH_COLUMNS;
  if (progress <= 0.24) return "low";
  if (progress <= 0.52) return "mid";
  if (progress <= 0.82) return "high";
  return "hot";
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

function deterministicUnit(value: string): number {
  let hash = 2166136261;
  for (let index = 0; index < value.length; index += 1) {
    hash ^= value.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0) / 0xffffffff;
}

function clamp(value: number): number {
  return Math.max(0, Math.min(1, value));
}
