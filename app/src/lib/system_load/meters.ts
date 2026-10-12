// Path: app/src/lib/system_load/meters.ts
// Description: Strip meter readings (bar level, severity, spoken value) and the plain-language panel verdicts

import { formatBits, formatPercent } from "./format.js";
import type { LoadSample, LoadState } from "./load_state.js";
import { memorySeverity, percentSeverity, type LoadSeverity } from "./severity.js";

export type MeterKey = "cpu" | "mem" | "gpu" | "dsk" | "net";

export const METER_KEYS: readonly MeterKey[] = ["cpu", "mem", "gpu", "dsk", "net"];
export const SEGMENT_COUNT = 10;
const NET_FLOOR_BYTES = 1024;
const NET_DECADES = Math.log10((1024 * 1024 * 1024) / NET_FLOOR_BYTES);

export const METER_LABELS: Record<MeterKey, string> = {
  cpu: "CPU", mem: "MEM", gpu: "GPU", dsk: "DSK", net: "NET",
};

export const PANEL_TITLES: Record<MeterKey, string> = {
  cpu: "CPU", mem: "MEMORY", gpu: "GPU", dsk: "DISK", net: "NETWORK",
};

const VERDICTS: Record<"compute" | "mem" | "io", Record<LoadSeverity, string>> = {
  compute: { calm: "Plenty of headroom", busy: "Busy", hot: "Maxed out", paging: "Maxed out" },
  mem: { calm: "Plenty free", busy: "Getting full", hot: "Nearly full", paging: "Full — paging to disk" },
  io: { calm: "Quiet", busy: "Busy", hot: "Saturated", paging: "Saturated" },
};

export interface MeterReading {
  key: MeterKey;
  label: string;
  level: number;
  severity: LoadSeverity;
  spoken: string;
}

/** NET bar: log10 throughput from 1 KB/s (empty) to 1 GB/s (full). */
export function netLevel(bytesPerSec: number): number {
  if (bytesPerSec <= NET_FLOOR_BYTES) return 0;
  return Math.min(100, (Math.log10(bytesPerSec / NET_FLOOR_BYTES) / NET_DECADES) * 100);
}

export function litSegments(level: number): number {
  if (level <= 0) return 0;
  return Math.max(1, Math.min(SEGMENT_COUNT, Math.round((level / 100) * SEGMENT_COUNT)));
}

export function verdict(key: MeterKey, severity: LoadSeverity): string {
  const family = key === "mem" ? "mem" : key === "cpu" || key === "gpu" ? "compute" : "io";
  return VERDICTS[family][severity];
}

function spokenValue(key: MeterKey, sample: LoadSample): string {
  if (key !== "net") return formatPercent(sample[key]);
  return `down ${formatBits(sample.netDown * 8)}, up ${formatBits(sample.netUp * 8)}`;
}

export function buildMeters(state: LoadState): MeterReading[] {
  const latest = state.history[state.history.length - 1];
  if (!latest) return [];
  return METER_KEYS.map((key) => {
    const level = key === "net" ? netLevel(latest.netDown + latest.netUp) : latest[key];
    const severity = key === "mem"
      ? memorySeverity(state.snapshot.memory, state.pagingStreak)
      : percentSeverity(key === "net" ? latest.netUtil : level);
    const label = severity === "paging" ? "PAGING" : METER_LABELS[key];
    const spoken = `${PANEL_TITLES[key]} ${spokenValue(key, latest)}, ${verdict(key, severity)}`;
    return { key, label, level, severity, spoken };
  });
}
