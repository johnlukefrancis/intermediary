// Path: app/src/lib/system_load/meters.ts
// Description: Strip meter readings (bar level, 5 s peak, value, severity) and LED segment tones

import { formatBitsCompact, formatPercent } from "./format.js";
import type { LoadSample, LoadState } from "./load_state.js";
import { memorySeverity, percentSeverity, type LoadSeverity } from "./severity.js";

export type MeterKey = "cpu" | "mem" | "gpu" | "dsk" | "net";
export type SegmentTone = "low" | "mid" | "high" | "hot";

export const METER_KEYS: readonly MeterKey[] = ["cpu", "mem", "gpu", "dsk", "net"];
export const SEGMENT_COUNT = 12;
const PEAK_HOLD_SAMPLES = 5;
const NET_FLOOR_BYTES = 1024;
const NET_DECADES = Math.log10((1024 * 1024 * 1024) / NET_FLOOR_BYTES);

export const METER_LABELS: Record<MeterKey, string> = {
  cpu: "CPU", mem: "MEM", gpu: "GPU", dsk: "DSK", net: "NET",
};

export const PANEL_TITLES: Record<MeterKey, string> = {
  cpu: "CPU", mem: "MEMORY", gpu: "GPU", dsk: "DISK", net: "NETWORK",
};

export interface MeterReading {
  key: MeterKey;
  label: string;
  level: number;
  peak: number;
  value: string;
  severity: LoadSeverity;
  spoken: string;
}

/** NET bar: log10 throughput from 1 KB/s (empty) to 1 GB/s (full). */
export function netLevel(bytesPerSec: number): number {
  if (bytesPerSec <= NET_FLOOR_BYTES) return 0;
  return Math.min(100, (Math.log10(bytesPerSec / NET_FLOOR_BYTES) / NET_DECADES) * 100);
}

export function levelOf(key: MeterKey, sample: LoadSample): number {
  return key === "net" ? netLevel(sample.netDown + sample.netUp) : sample[key];
}

/** Lit segments ramp by position, so a full bar always ends hot. */
export function segmentTone(index: number): SegmentTone {
  if (index >= 10) return "hot";
  if (index >= 7) return "high";
  if (index >= 5) return "mid";
  return "low";
}

export function litSegments(level: number): number {
  if (level <= 0) return 0;
  return Math.max(1, Math.min(SEGMENT_COUNT, Math.round((level / 100) * SEGMENT_COUNT)));
}

export function buildMeters(state: LoadState): MeterReading[] {
  const latest = state.history[state.history.length - 1];
  if (!latest) return [];
  const recent = state.history.slice(-PEAK_HOLD_SAMPLES);
  return METER_KEYS.map((key) => {
    const level = levelOf(key, latest);
    const peak = Math.max(level, ...recent.map((sample) => levelOf(key, sample)));
    const severity = key === "mem"
      ? memorySeverity(state.snapshot.memory, state.pagingStreak)
      : percentSeverity(key === "net" ? latest.netUtil : level);
    const value = key === "net"
      ? `↓${formatBitsCompact(latest.netDown * 8)} ↑${formatBitsCompact(latest.netUp * 8)}`
      : formatPercent(level);
    const label = severity === "paging" ? "PAGING" : METER_LABELS[key];
    return { key, label, level, peak, value, severity, spoken: `${PANEL_TITLES[key]} ${value}, ${severity}` };
  });
}
