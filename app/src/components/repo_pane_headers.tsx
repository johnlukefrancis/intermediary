// Path: app/src/components/repo_pane_headers.tsx
// Description: Header controls for repo Docs and Code file panes

import type React from "react";

export type FilePaneView = "recent" | "starred";

interface DocsHeaderRightProps {
  view: FilePaneView;
  onViewChange: (view: FilePaneView) => void;
  onOpenNote: () => void;
}

interface CodeHeaderRightProps {
  view: FilePaneView;
  onViewChange: (view: FilePaneView) => void;
}

export function DocsHeaderLeft({
  onRecent,
}: {
  onRecent: () => void;
}): React.JSX.Element {
  return (
    <button
      type="button"
      className="panel-title-button"
      onClick={onRecent}
      title="Show recent docs"
    >
      Docs
    </button>
  );
}

export function DocsHeaderRight({
  view,
  onViewChange,
  onOpenNote,
}: DocsHeaderRightProps): React.JSX.Element {
  const isStarred = view === "starred";

  return (
    <div className="panel-header-icons">
      <button
        type="button"
        className={`panel-header-icon${isStarred ? " panel-header-icon--active" : ""}`}
        onClick={() => {
          onViewChange(isStarred ? "recent" : "starred");
        }}
        title={isStarred ? "Show recent docs" : "Show favourited docs"}
        aria-label={isStarred ? "Show recent docs" : "Show favourited docs"}
        aria-pressed={isStarred}
      >
        ★
      </button>
      <button
        type="button"
        className="panel-header-icon"
        onClick={onOpenNote}
        title="Open notes"
        aria-label="Open notes"
      >
        ✎
      </button>
    </div>
  );
}

export function CodeHeaderLeft({
  view,
  onRecent,
}: {
  view: FilePaneView;
  onRecent: () => void;
}): React.JSX.Element {
  return (
    <button
      type="button"
      className={`panel-title-button${view === "starred" ? " panel-title-button--dimmed" : ""}`}
      onClick={onRecent}
      title="Show recent files"
    >
      Code
    </button>
  );
}

export function CodeHeaderRight({
  view,
  onViewChange,
}: CodeHeaderRightProps): React.JSX.Element {
  const isStarred = view === "starred";

  return (
    <button
      type="button"
      className={`panel-header-icon${isStarred ? " panel-header-icon--active" : ""}`}
      onClick={() => {
        onViewChange(isStarred ? "recent" : "starred");
      }}
      title={isStarred ? "Show recent files" : "Show favourited files"}
      aria-label={isStarred ? "Show recent files" : "Show favourited files"}
      aria-pressed={isStarred}
    >
      ★
    </button>
  );
}
