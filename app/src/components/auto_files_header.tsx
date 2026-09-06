// Path: app/src/components/auto_files_header.tsx
// Description: Header controls for the unified Auto files panel

import type React from "react";
import { StreamIcon } from "./stream/stream_icons.js";
import type { FileTypeFilter } from "../lib/files/file_feed.js";
import type { FilesMode } from "../lib/files/files_mode.js";

interface AutoFilesHeaderProps {
  filter: FileTypeFilter;
  mode: FilesMode;
  prefix?: React.ReactNode;
  /** Rendered immediately right of the rocker; the Stream panel puts its LIVE indicator here */
  liveSlot?: React.ReactNode;
  onFilterChange: (filter: FileTypeFilter) => void;
  onModeChange: (mode: FilesMode) => void;
}

/** The one rocker over the panel's four modes; STREAM leads because it is the default */
const PANEL_MODES: ReadonlyArray<{ value: FilesMode; title: string; icon: React.ReactNode }> = [
  { value: "stream", title: "Stream mode", icon: <StreamIcon /> },
  { value: "auto", title: "Auto sort", icon: <GridIcon /> },
  { value: "latest", title: "Latest sort", icon: <ListIcon /> },
  { value: "active", title: "Active sort", icon: <PulseIcon /> },
];

const FILTERS: ReadonlyArray<{ value: FileTypeFilter; label: string; icon: React.ReactNode }> = [
  { value: "all", label: "All", icon: <ImageIcon /> },
  { value: "docs", label: "Docs", icon: <DocumentIcon /> },
  { value: "code", label: "Code", icon: <CodeIcon /> },
  { value: "image", label: "Images", icon: <ImageIcon /> },
];

export function AutoFilesHeader({
  filter,
  mode,
  prefix,
  liveSlot,
  onFilterChange,
  onModeChange,
}: AutoFilesHeaderProps): React.JSX.Element {
  return (
    <div className="auto-files-header">
      {prefix}
      <div className="auto-files-mode" role="toolbar" aria-label="File panel modes">
        {PANEL_MODES.map((entry) => (
          <button
            key={entry.value}
            type="button"
            className={`auto-files-icon-button${
              mode === entry.value ? " auto-files-icon-button--active" : ""
            }`}
            aria-label={entry.title}
            title={entry.title}
            aria-pressed={mode === entry.value}
            onClick={() => { onModeChange(entry.value); }}
          >
            {entry.icon}
          </button>
        ))}
      </div>
      {liveSlot}
      <div className="auto-files-filter" role="toolbar" aria-label="File type filters">
        {FILTERS.map((entry) => {
          const isIconOnly = entry.value !== "all";
          return (
            <button
              key={entry.value}
              type="button"
              className={[
                "auto-files-filter-button",
                isIconOnly ? "auto-files-filter-button--icon-only" : "auto-files-filter-button--with-label",
                filter === entry.value ? "auto-files-filter-button--active" : "",
              ].filter(Boolean).join(" ")}
              onClick={() => { onFilterChange(entry.value); }}
              aria-label={`Show ${entry.label}`}
              aria-pressed={filter === entry.value}
              title={`Show ${entry.label}`}
            >
              {entry.icon}
              {!isIconOnly && <span>{entry.label}</span>}
            </button>
          );
        })}
      </div>
    </div>
  );
}

function GridIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4" y="4" width="6" height="6" />
      <rect x="14" y="4" width="6" height="6" />
      <rect x="4" y="14" width="6" height="6" />
      <rect x="14" y="14" width="6" height="6" />
    </svg>
  );
}

function ListIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M8 6h12M8 12h12M8 18h12" />
      <path d="M4 6h.01M4 12h.01M4 18h.01" />
    </svg>
  );
}

function PulseIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 12h3l2-5 4 10 2-5h5" />
    </svg>
  );
}

function DocumentIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M7 3h7l4 4v14H7z" />
      <path d="M14 3v5h5M9 13h6M9 17h6" />
    </svg>
  );
}

function CodeIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <path d="M9 18l-6-6 6-6M15 6l6 6-6 6" />
    </svg>
  );
}

function ImageIcon(): React.JSX.Element {
  return (
    <svg viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4" y="5" width="16" height="14" />
      <path d="M7 16l4-4 3 3 2-2 3 3M15 9h.01" />
    </svg>
  );
}
