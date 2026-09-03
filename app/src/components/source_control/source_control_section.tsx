// Path: app/src/components/source_control/source_control_section.tsx
// Description: Collapsible MERGE CONFLICTS / STAGED CHANGES / CHANGES section with capped rows and a bulk action

import type React from "react";
import { useId, useState } from "react";
import type { SourceControlEntry } from "../../shared/protocol.js";
import { ChevronIcon, MinusIcon, PlusIcon } from "./source_control_icons.js";
import { SourceControlRow, type RowActionKind } from "./source_control_row.js";

const MAX_ROWS = 500;

/** Section-wide stage/unstage rendered as the same + / − glyph the row hover action uses */
export interface SectionBulkAction {
  kind: RowActionKind;
  title: string;
  disabled: boolean;
  onClick: () => void;
}

interface SourceControlSectionProps {
  title: string;
  /** `alert` renders the section in the error tone (MERGE CONFLICTS outranks the other sections) */
  tone?: "alert";
  entries: SourceControlEntry[];
  rowAction: RowActionKind;
  bulk?: SectionBulkAction;
  /** Every per-row action is disabled (an action is pending or status is not ready) */
  disabled: boolean;
  onRowAction: (entry: SourceControlEntry) => void;
  onOpenDiff: (entry: SourceControlEntry) => void;
  onContextMenu: (event: React.MouseEvent, entry: SourceControlEntry) => void;
}

export function SourceControlSection({
  title,
  tone,
  entries,
  rowAction,
  bulk,
  disabled,
  onRowAction,
  onOpenDiff,
  onContextMenu,
}: SourceControlSectionProps): React.JSX.Element {
  const [collapsed, setCollapsed] = useState(false);
  const bodyId = useId();
  const visible = entries.length > MAX_ROWS ? entries.slice(0, MAX_ROWS) : entries;
  const hidden = entries.length - visible.length;

  return (
    <section
      className="source-control-section"
      data-tone={tone}
      data-collapsed={collapsed || undefined}
    >
      <div className="source-control-section__header">
        <button
          type="button"
          className="source-control-section__toggle"
          aria-expanded={!collapsed}
          aria-controls={bodyId}
          onClick={() => { setCollapsed((value) => !value); }}
        >
          <span className="source-control-section__chevron" aria-hidden="true">
            <ChevronIcon />
          </span>
          {tone === "alert" && (
            <span className="source-control-section__alert" aria-hidden="true">!</span>
          )}
          <span className="source-control-section__title">{title}</span>
          <span className="source-control-section__count">[{entries.length}]</span>
        </button>
        {bulk && (
          <button
            type="button"
            className="source-control-section__bulk"
            disabled={bulk.disabled}
            title={bulk.title}
            aria-label={bulk.title}
            onClick={bulk.onClick}
          >
            {bulk.kind === "stage" ? <PlusIcon /> : <MinusIcon />}
          </button>
        )}
      </div>
      {!collapsed && (
        <div id={bodyId} className="source-control-section__rows" role="list">
          {visible.map((entry) => (
            <SourceControlRow
              key={`${entry.area}:${entry.path}`}
              entry={entry}
              actionKind={rowAction}
              disabled={disabled}
              onAction={onRowAction}
              onOpenDiff={onOpenDiff}
              onContextMenu={onContextMenu}
            />
          ))}
          {hidden > 0 && (
            <p className="empty-state source-control-section__more">+{hidden} MORE</p>
          )}
        </div>
      )}
    </section>
  );
}
