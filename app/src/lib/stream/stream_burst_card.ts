// Path: app/src/lib/stream/stream_burst_card.ts
// Description: Pure burst card arithmetic: counts per op and kind, top directories, elapsed time

import type { VisibleFileKind } from "../files/file_feed.js";
import type { DeltaOp, FileChangeType } from "../../shared/protocol.js";
import { BURST_TOP_DIRS, BURST_TOP_DIRS_TRACKED } from "./stream_bounds.js";
import type { StreamBurstCard } from "./stream_types.js";

export type DirCounts = ReadonlyMap<string, number>;

/** `arrivedAtMs` may predate `now`: a collapse seeds it from the oldest card it folded in */
export function newBurstCard(id: number, now: number, arrivedAtMs = now): StreamBurstCard {
  return {
    kind: "burst",
    id,
    arrivedAtMs,
    updatedAtMs: now,
    admittedAtMs: 0,
    files: 0,
    byOp: { add: 0, modify: 0, remove: 0, rename: 0 },
    byKind: { docs: 0, code: 0, image: 0 },
    topDirs: [],
    elapsedMs: 0,
    resolved: 0,
    exiting: false,
    static: false,
  };
}

export function opForChangeType(changeType: FileChangeType): DeltaOp {
  switch (changeType) {
    case "add":
      return "add";
    case "change":
      return "modify";
    case "unlink":
      return "remove";
  }
}

/** The top-level directory, or `/` for a root file */
export function burstDir(path: string): string {
  const slash = path.indexOf("/");
  return slash === -1 ? "/" : path.slice(0, slash);
}

/** The bucket every directory past BURST_TOP_DIRS_TRACKED lands in */
export const BURST_OTHER_DIR = "other";

/** One count per top-level directory; once BURST_TOP_DIRS_TRACKED are tallied, a new directory counts as `other` */
export function countDir(dirCounts: DirCounts, path: string): Map<string, number> {
  const next = new Map(dirCounts);
  const real = burstDir(path);
  const tracked = next.size - (next.has(BURST_OTHER_DIR) ? 1 : 0);
  const dir = next.has(real) || tracked < BURST_TOP_DIRS_TRACKED ? real : BURST_OTHER_DIR;
  next.set(dir, (next.get(dir) ?? 0) + 1);
  return next;
}

export function topDirsOf(dirCounts: DirCounts): ReadonlyArray<{ dir: string; count: number }> {
  return [...dirCounts.entries()]
    .map(([dir, count]) => ({ dir, count }))
    .sort((a, b) => b.count - a.count || a.dir.localeCompare(b.dir))
    .slice(0, BURST_TOP_DIRS);
}

export interface BurstAbsorb {
  op: DeltaOp;
  fileKind: VisibleFileKind;
  /** True when this path had not been absorbed before */
  newPath: boolean;
  dirCounts: DirCounts;
  now: number;
}

export function absorbIntoBurstCard(card: StreamBurstCard, absorb: BurstAbsorb): StreamBurstCard {
  return {
    ...card,
    updatedAtMs: absorb.now,
    files: absorb.newPath ? card.files + 1 : card.files,
    byOp: { ...card.byOp, [absorb.op]: card.byOp[absorb.op] + 1 },
    byKind: { ...card.byKind, [absorb.fileKind]: card.byKind[absorb.fileKind] + 1 },
    topDirs: topDirsOf(absorb.dirCounts),
    elapsedMs: absorb.now - card.arrivedAtMs,
  };
}
