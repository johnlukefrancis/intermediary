// Path: app/src/components/repo_pane_headers.tsx
// Description: Header controls for ranked repo file feeds

import type React from "react";
import type { FileTypeFilter } from "../lib/files/file_feed.js";

interface FileFeedHeaderProps {
  title: string;
  filter: FileTypeFilter;
  onFilterChange: (filter: FileTypeFilter) => void;
  onOpenNote?: () => void;
  showTitle?: boolean;
}

interface FilterButtonDef {
  value: FileTypeFilter;
  label: string;
  icon: React.JSX.Element;
}

const FILTERS: readonly FilterButtonDef[] = [
  { value: "all", label: "Show all file types", icon: <AllIcon /> },
  { value: "docs", label: "Show documents", icon: <DocumentIcon /> },
  { value: "code", label: "Show code", icon: <CodeIcon /> },
  { value: "image", label: "Show images", icon: <ImageIcon /> },
];

export function FileFeedHeader({
  title,
  filter,
  onFilterChange,
  onOpenNote,
  showTitle = true,
}: FileFeedHeaderProps): React.JSX.Element {
  return (
    <div className="file-feed-header">
      {showTitle && <span className="file-feed-title">{title}</span>}
      <div className="file-feed-filter" role="toolbar" aria-label={`${title} file filters`}>
        {FILTERS.map((entry) => (
          <button
            key={entry.value}
            type="button"
            className={`file-feed-filter__button${
              filter === entry.value ? " file-feed-filter__button--active" : ""
            }`}
            onClick={() => { onFilterChange(entry.value); }}
            title={entry.label}
            aria-label={entry.label}
            aria-pressed={filter === entry.value}
          >
            {entry.icon}
          </button>
        ))}
      </div>
      {onOpenNote && (
        <button
          type="button"
          className="panel-header-icon"
          onClick={onOpenNote}
          title="Open notes"
          aria-label="Open notes"
        >
          ✎
        </button>
      )}
    </div>
  );
}

function AllIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <rect x="3" y="3" width="5" height="5" />
      <rect x="12" y="3" width="5" height="5" />
      <rect x="3" y="12" width="5" height="5" />
      <rect x="12" y="12" width="5" height="5" />
    </svg>
  );
}

function DocumentIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="M5 2h7l4 4v12H5V2zm7 1.8V7h3.2M7 10h7M7 13h7M7 16h5" />
    </svg>
  );
}

function CodeIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <path d="M7.5 5L3 10l4.5 5M12.5 5L17 10l-4.5 5" />
    </svg>
  );
}

function ImageIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 20 20" aria-hidden="true">
      <rect x="3" y="4" width="14" height="12" />
      <circle cx="13.5" cy="7.5" r="1.5" />
      <path d="M5 14l4-4 3 3 2-2 3 3" />
    </svg>
  );
}
