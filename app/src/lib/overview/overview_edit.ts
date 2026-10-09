// Path: app/src/lib/overview/overview_edit.ts
// Description: Metadata-only settled edits with bounded excerpts from the shared patch parser

import type { FileDeltaEvent } from "../../shared/protocol.js";
import { parsePatch } from "../diff/diff_lines.js";
import type { OverviewEdit } from "./overview_types.js";

export function projectEdit(event: FileDeltaEvent, now: number, id: number): OverviewEdit {
  const payload = event.payload;
  const edit: OverviewEdit = {
    id,
    path: event.path, fromPath: event.fromPath ?? null, kind: event.kind,
    op: event.op, at: now, lines: [], image: null, added: null, removed: null,
    baseline: null, detail: "", truncated: false,
  };
  if (payload.kind === "text") {
    const changed = parsePatch(payload.patch, false).lines.filter((line) => line.kind === "add" || line.kind === "del");
    edit.lines = changed.slice(0, 6).map((line) => ({
      kind: line.kind === "add" ? "add" : "del", text: line.text.slice(1, 181),
    }));
    edit.added = payload.stats.added;
    edit.removed = payload.stats.removed;
    edit.baseline = payload.baseline;
    edit.truncated = payload.truncated || changed.length > 6 || changed.some((line) => line.text.length > 181);
    edit.detail = payload.stats.added + payload.stats.removed === 0 ? "No textual changes" : "Settled text edit";
  } else if (payload.kind === "image") {
    edit.image = { bytes: payload.bytes, mimeType: payload.mimeType, mtimeMs: payload.mtimeMs };
    edit.detail = `Image · ${payload.mimeType ?? "unknown format"} · ${String(payload.bytes)} bytes`;
  } else if (payload.kind === "opaque") {
    const reason = { binary: "binary content", tooLarge: "content too large", unreadable: "content unreadable" }[payload.reason];
    edit.detail = `No text preview · ${reason} · ${String(payload.bytes)} bytes`;
  } else {
    edit.detail = "File removed · content unavailable";
  }
  return edit;
}
