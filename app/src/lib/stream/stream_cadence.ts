// Path: app/src/lib/stream/stream_cadence.ts
// Description: Pure cadence and pressure arithmetic for the Stream conductor

import {
  BURST_THRESHOLD,
  CADENCE_BASE_MS,
  CADENCE_MIN_MS,
  LAG_BUDGET_MS,
  PRESSURE_BUSY_AT,
  PRESSURE_FLOOD_AT,
} from "./stream_bounds.js";
import type { StreamPressure } from "./stream_types.js";

/** Milliseconds between admissions: the lag budget spread over the backlog, clamped to the band */
export function cadenceMs(backlog: number): number {
  const spread = LAG_BUDGET_MS / Math.max(1, backlog);
  return Math.min(CADENCE_BASE_MS, Math.max(CADENCE_MIN_MS, spread));
}

/** calm below PRESSURE_BUSY_AT · busy below PRESSURE_FLOOD_AT · flood from there */
export function pressureBand(backlog: number): StreamPressure {
  if (backlog >= PRESSURE_FLOOD_AT) return "flood";
  if (backlog >= PRESSURE_BUSY_AT) return "busy";
  return "calm";
}

/** A backlog this deep is a DOM bound, not a cadence problem: it collapses into one burst card */
export function shouldCollapse(backlog: number): boolean {
  return backlog >= BURST_THRESHOLD;
}
