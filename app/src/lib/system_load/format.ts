// Path: app/src/lib/system_load/format.ts
// Description: Task Manager style number formatting for system-load meters and detail panels

const KIB = 1024;
const MIB = KIB * 1024;
const GIB = MIB * 1024;

export function formatPercent(percent: number): string {
  return `${Math.round(clampPercent(percent))}%`;
}

export function formatPercentFine(percent: number): string {
  const value = clampPercent(percent);
  return value > 0 && value < 0.1 ? "<0.1%" : `${value.toFixed(1)}%`;
}

/** Memory sizes: binary units labelled GB/MB as Task Manager does. */
export function formatMemory(bytes: number): string {
  if (bytes >= GIB) return `${(bytes / GIB).toFixed(1)} GB`;
  if (bytes >= MIB) return `${Math.round(bytes / MIB)} MB`;
  return `${Math.round(bytes / KIB)} KB`;
}

export function formatGigabytes(bytes: number): string {
  return (bytes / GIB).toFixed(1);
}

export function formatRate(bytesPerSec: number): string {
  if (bytesPerSec >= GIB) return `${(bytesPerSec / GIB).toFixed(1)} GB/s`;
  if (bytesPerSec >= MIB) return `${(bytesPerSec / MIB).toFixed(1)} MB/s`;
  return `${Math.round(bytesPerSec / KIB)} KB/s`;
}

/** Network rates are decimal bits per second (Kbps/Mbps/Gbps). */
export function formatBits(bitsPerSec: number): string {
  if (bitsPerSec >= 1e9) return `${(bitsPerSec / 1e9).toFixed(1)} Gbps`;
  if (bitsPerSec >= 1e6) return `${(bitsPerSec / 1e6).toFixed(1)} Mbps`;
  return `${(bitsPerSec / 1e3).toFixed(bitsPerSec >= 1e5 ? 0 : 1)} Kbps`;
}

export function formatLinkSpeed(bitsPerSec: number): string {
  if (bitsPerSec >= 1e9) return `${trimZero(bitsPerSec / 1e9)} Gbps`;
  return `${trimZero(bitsPerSec / 1e6)} Mbps`;
}

/** Strip-width bits per second: at most four characters, e.g. 840K, 12M, 1.2G. */
export function formatBitsCompact(bitsPerSec: number): string {
  if (bitsPerSec >= 1e9) return `${compact(bitsPerSec / 1e9)}G`;
  if (bitsPerSec >= 1e6) return `${compact(bitsPerSec / 1e6)}M`;
  if (bitsPerSec >= 1e3) return `${compact(bitsPerSec / 1e3)}K`;
  return "0";
}

export function formatGhz(mhz: number): string {
  return `${(mhz / 1000).toFixed(2)} GHz`;
}

export function formatCount(value: number): string {
  return Math.round(value).toLocaleString();
}

function compact(value: number): string {
  return value < 10 ? value.toFixed(1) : String(Math.round(value));
}

function trimZero(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(1);
}

export function clampPercent(percent: number): number {
  return Number.isFinite(percent) ? Math.min(100, Math.max(0, percent)) : 0;
}
