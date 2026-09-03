// Path: app/src/lib/source_control/change_badges.ts
// Description: Single badge map (letter, variant, label) for every source-control change kind

import type { SourceControlChange } from "../../shared/protocol.js";

export interface ChangeBadge {
  letter: string;
  /** Suffix of the `.badge--*` class; never `staged`, which means drag-handoff staging */
  variant: string;
  label: string;
}

export const CHANGE_BADGES: Record<SourceControlChange, ChangeBadge> = {
  added: { letter: "A", variant: "add", label: "Added" },
  modified: { letter: "M", variant: "modify", label: "Modified" },
  deleted: { letter: "D", variant: "delete", label: "Deleted" },
  renamed: { letter: "R", variant: "warning", label: "Renamed" },
  copied: { letter: "C", variant: "warning", label: "Copied" },
  typeChanged: { letter: "T", variant: "typechange", label: "Type changed" },
  untracked: { letter: "U", variant: "untracked", label: "Untracked" },
  unmerged: { letter: "!", variant: "error", label: "Conflict" },
};
