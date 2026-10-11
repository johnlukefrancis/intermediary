// Path: app/src/components/source_control/source_control_section_head.tsx
// Description: Shared collapsible group header: chevron, alert mark, LABEL [n], and an optional bulk-action slot

import type React from "react";
import { ChevronIcon } from "./source_control_icons.js";

interface SourceControlSectionHeadProps {
  title: string;
  /** `alert` adds the error mark; the enclosing section's data-tone colours the label */
  tone: "alert" | undefined;
  count: number;
  collapsed: boolean;
  bodyId: string;
  onToggle: () => void;
  children?: React.ReactNode;
}

export function SourceControlSectionHead({
  title,
  tone,
  count,
  collapsed,
  bodyId,
  onToggle,
  children,
}: SourceControlSectionHeadProps): React.JSX.Element {
  return (
    <div className="source-control-section__header">
      <button
        type="button"
        className="source-control-section__toggle"
        aria-expanded={!collapsed}
        aria-controls={bodyId}
        onClick={onToggle}
      >
        <span className="source-control-section__chevron" aria-hidden="true">
          <ChevronIcon />
        </span>
        {tone === "alert" && (
          <span className="source-control-section__alert" aria-hidden="true">!</span>
        )}
        <span className="source-control-section__title">{title}</span>
        <span className="source-control-section__count">[{count}]</span>
      </button>
      {children}
    </div>
  );
}
