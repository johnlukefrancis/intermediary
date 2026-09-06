// Path: app/src/lib/stream/testing/stream_fixtures.ts
// Description: Event and card builders shared by the Stream unit tests; never imported by app code

import type { FileChangedEvent, FileDeltaEvent } from "../../../shared/protocol.js";
import type { StreamFileCard } from "../stream_types.js";

let seq = 0;

export function resetSeq(): void {
  seq = 0;
}

export function patchOf(added: number, removed: number): string {
  const lines = ["@@ -1,1 +1,1 @@"];
  for (let index = 0; index < removed; index += 1) lines.push(`-old ${String(index)}`);
  for (let index = 0; index < added; index += 1) lines.push(`+new ${String(index)}`);
  return `${lines.join("\n")}\n`;
}

export function textDelta(path: string, overrides: Partial<FileDeltaEvent> = {}, added = 2, removed = 1): FileDeltaEvent {
  seq += 1;
  return {
    type: "fileDelta",
    repoId: "r",
    seq,
    path,
    kind: "code",
    op: "modify",
    mtime: "2026-09-06T00:00:00Z",
    folded: 0,
    withheld: 0,
    dropped: 0,
    payload: {
      kind: "text",
      patch: patchOf(added, removed),
      stats: { added, removed, hunks: 1, newLines: 10 },
      baseline: "previousSighting",
      truncated: false,
    },
    ...overrides,
  };
}

/** An image delta: metadata only, a live png by default; pass `{ op: "remove", payload: { kind: "gone" } }` for a delete */
export function imageDelta(path: string, overrides: Partial<FileDeltaEvent> = {}, bytes = 1024): FileDeltaEvent {
  seq += 1;
  return {
    type: "fileDelta",
    repoId: "r",
    seq,
    path,
    kind: "image",
    op: "add",
    mtime: "2026-09-06T00:00:00Z",
    folded: 0,
    withheld: 0,
    dropped: 0,
    payload: { kind: "image", bytes, mimeType: "image/png" },
    ...overrides,
  };
}

export function changed(path: string, changeType: FileChangedEvent["changeType"] = "change"): FileChangedEvent {
  return { type: "fileChanged", repoId: "r", path, kind: "code", changeType, mtime: "2026-09-06T00:00:00Z" };
}

export function fileCard(id: number, path: string, now: number, overrides: Partial<StreamFileCard> = {}): StreamFileCard {
  return {
    kind: "file",
    id,
    path,
    fromPath: null,
    fileKind: "code",
    op: "modify",
    tracked: true,
    outsideSelection: false,
    clock: "00:00:00",
    arrivedAtMs: now,
    updatedAtMs: now,
    admittedAtMs: now,
    edits: 1,
    expanded: false,
    exiting: false,
    static: false,
    body: { status: "gone" },
    ...overrides,
  };
}
