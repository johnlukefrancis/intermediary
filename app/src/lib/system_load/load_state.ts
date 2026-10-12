// Path: app/src/lib/system_load/load_state.ts
// Description: Folds each system-load snapshot into the latest reading, a 60-sample history and the paging streak

import type { NetworkLoad, SystemLoadSnapshot } from "../../shared/protocol_system_load.js";
import { clampPercent } from "./format.js";
import { memoryInUsePercent, nextPagingStreak } from "./severity.js";

export const HISTORY_LENGTH = 60;

/** Per-sample headline values; percentages are 0–100 and network rates are bytes per second. */
export interface LoadSample {
  cpu: number;
  mem: number;
  gpu: number;
  dsk: number;
  netDown: number;
  netUp: number;
  netUtil: number;
}

export interface LoadState {
  snapshot: SystemLoadSnapshot;
  history: readonly LoadSample[];
  pagingStreak: number;
}

/** Virtual adapters relay the same traffic as the physical NIC, so summing interfaces double-counts. */
export function busiestInterface(networks: readonly NetworkLoad[]): NetworkLoad | null {
  let best: NetworkLoad | null = null;
  for (const net of networks) {
    const total = net.receivedBytesPerSec + net.sentBytesPerSec;
    if (!best || total > best.receivedBytesPerSec + best.sentBytesPerSec) best = net;
  }
  return best;
}

export function networkUtilPercent(net: NetworkLoad): number {
  if (net.linkBitsPerSec <= 0) return 0;
  return clampPercent(((net.receivedBytesPerSec + net.sentBytesPerSec) * 8 * 100) / net.linkBitsPerSec);
}

export function sampleOf(snapshot: SystemLoadSnapshot): LoadSample {
  const net = busiestInterface(snapshot.networks);
  return {
    cpu: clampPercent(snapshot.cpu.usagePercent),
    mem: clampPercent(memoryInUsePercent(snapshot.memory)),
    gpu: clampPercent(Math.max(0, ...snapshot.gpus.map((gpu) => gpu.usagePercent))),
    dsk: clampPercent(Math.max(0, ...snapshot.disks.map((disk) => disk.activePercent))),
    netDown: net?.receivedBytesPerSec ?? 0,
    netUp: net?.sentBytesPerSec ?? 0,
    netUtil: net ? networkUtilPercent(net) : 0,
  };
}

export function advanceLoadState(previous: LoadState | null, snapshot: SystemLoadSnapshot): LoadState {
  const history = [...(previous?.history ?? []), sampleOf(snapshot)].slice(-HISTORY_LENGTH);
  return {
    snapshot,
    history,
    pagingStreak: nextPagingStreak(previous?.pagingStreak ?? 0, snapshot.memory),
  };
}
