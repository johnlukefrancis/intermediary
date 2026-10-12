// Path: app/src/lib/system_load/severity.ts
// Description: Calm/busy/hot/paging classification for system-load meters (docs/design/system_load_design.md)

import type { MemoryLoad } from "../../shared/protocol_system_load.js";

export type LoadSeverity = "calm" | "busy" | "hot" | "paging";

export const PAGING_STREAK_SAMPLES = 3;
const PAGING_IN_USE_PERCENT = 85;
const PAGING_HARD_FAULTS_PER_SEC = 100;

export function percentSeverity(percent: number): LoadSeverity {
  if (percent >= 85) return "hot";
  if (percent >= 60) return "busy";
  return "calm";
}

export function memoryInUsePercent(memory: MemoryLoad): number {
  if (memory.totalBytes <= 0) return 0;
  return ((memory.totalBytes - memory.availableBytes) / memory.totalBytes) * 100;
}

export function commitPercent(memory: MemoryLoad): number {
  return memory.commitLimitBytes > 0 ? (memory.commitBytes / memory.commitLimitBytes) * 100 : 0;
}

/** One sample's vote towards paging; the streak must reach PAGING_STREAK_SAMPLES. */
export function isPagingSample(memory: MemoryLoad): boolean {
  return (
    memoryInUsePercent(memory) >= PAGING_IN_USE_PERCENT &&
    memory.hardFaultsPerSec >= PAGING_HARD_FAULTS_PER_SEC
  );
}

export function nextPagingStreak(previous: number, memory: MemoryLoad): number {
  return isPagingSample(memory) ? previous + 1 : 0;
}

export function memorySeverity(memory: MemoryLoad, pagingStreak: number): LoadSeverity {
  if (pagingStreak >= PAGING_STREAK_SAMPLES) return "paging";
  const inUse = memoryInUsePercent(memory);
  if (inUse >= 90 || commitPercent(memory) >= 90) return "hot";
  if (inUse >= 75) return "busy";
  return "calm";
}
