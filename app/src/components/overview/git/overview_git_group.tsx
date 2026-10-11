// Path: app/src/components/overview/git/overview_git_group.tsx
// Description: One Git column group: shared section head, capped read-only rows that open the diff, and the SOURCE overflow link

import type React from "react";
import { useId, useState } from "react";
import type { SourceControlEntry } from "../../../shared/protocol.js";
import type { StatusGroupKind } from "../../../lib/source_control/status_groups.js";
import { SourceControlSectionHead } from "../../source_control/source_control_section_head.js";
import { MERGE_CONFLICTS_TITLE } from "../../source_control/source_control_copy.js";
import { SourceControlEntryFace, entryPathTitle, isDeletedEntry, opensDiff } from "../../source_control/source_control_entry_face.js";

const ROW_CAP = 8;

const GROUP_TITLES: Readonly<Record<StatusGroupKind, string>> = {
  conflicts: MERGE_CONFLICTS_TITLE, staged: "STAGED CHANGES", changes: "CHANGES",
};

interface OverviewGitGroupProps {
  kind: StatusGroupKind;
  entries: readonly SourceControlEntry[];
  onOpenDiff: (entry: SourceControlEntry) => void;
  onOpenSource: () => void;
}

function rowTitle(entry: SourceControlEntry): string {
  const title = entryPathTitle(entry);
  if (!isDeletedEntry(entry)) return `${title} — open diff`;
  return opensDiff(entry) ? `${title} (deleted) — open image diff` : `${title} (deleted)`;
}

export function OverviewGitGroup({ kind, entries, onOpenDiff, onOpenSource }: OverviewGitGroupProps): React.JSX.Element {
  const alert = kind === "conflicts";
  // Overview is a glance across repositories: counts first, rows on request.
  const [collapsed, setCollapsed] = useState(true);
  const bodyId = useId();
  const hidden = entries.length - ROW_CAP;
  return (
    <section className="source-control-section overview-git-group" data-group={kind}
      data-tone={alert ? "alert" : undefined} data-collapsed={collapsed || undefined}>
      <SourceControlSectionHead title={GROUP_TITLES[kind]} tone={alert ? "alert" : undefined} count={entries.length}
        collapsed={collapsed} bodyId={bodyId} onToggle={() => { setCollapsed((value) => !value); }} />
      {!collapsed && <div id={bodyId} className="overview-git-group__rows">
        {entries.slice(0, ROW_CAP).map((entry) => (
          <button key={`${entry.area}:${entry.path}`} type="button" className="source-control-row overview-git-row"
            data-change={entry.change} data-deleted={isDeletedEntry(entry) || undefined} disabled={!opensDiff(entry)}
            title={rowTitle(entry)} onClick={() => { onOpenDiff(entry); }}>
            <SourceControlEntryFace entry={entry} />
          </button>
        ))}
        {hidden > 0 && <button type="button" className="overview-git-more" onClick={onOpenSource}>
          +{hidden} MORE IN SOURCE
        </button>}
      </div>}
    </section>
  );
}
