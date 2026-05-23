// Path: app/src/components/layout/workspace_layout.tsx
// Description: Layout that replaces Auto files with a shared workspace

import type React from "react";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

interface WorkspaceLayoutProps {
  title: string;
  subtitle: string | undefined;
  onClose: () => void;
  content: React.ReactNode;
  zipsContent: React.ReactNode;
  isHandset: boolean;
}

interface WorkspacePanelProps {
  title: string;
  subtitle: string | undefined;
  onClose: () => void;
  content: React.ReactNode;
}

function WorkspacePanel({
  title,
  subtitle,
  onClose,
  content,
}: WorkspacePanelProps): React.JSX.Element {
  return (
    <section className="panel text-workspace-panel">
      <header className="panel-header text-workspace-header">
        <div className="text-workspace-heading">
          <h2 className="text-workspace-title">{title}</h2>
          {subtitle && <span className="text-workspace-subtitle">{subtitle}</span>}
        </div>
        <button
          type="button"
          className="panel-header-icon text-workspace-close"
          onClick={onClose}
          title="Close workspace"
          aria-label="Close workspace"
        >
          ×
        </button>
      </header>
      <div className="panel-content text-workspace-content">{content}</div>
    </section>
  );
}

function ZipsPanel({ zipsContent }: { zipsContent: React.ReactNode }): React.JSX.Element {
  return (
    <section className="panel" data-panel="zips">
      <header className="panel-header">
        <h2 className="panel-title">Zips</h2>
        <span className="panel-cue" aria-hidden="true" />
      </header>
      <div className="panel-content">{zipsContent}</div>
    </section>
  );
}

export function WorkspaceLayout({
  title,
  subtitle,
  onClose,
  content,
  zipsContent,
  isHandset,
}: WorkspaceLayoutProps): React.JSX.Element {
  const panel = (
    <WorkspacePanel title={title} subtitle={subtitle} onClose={onClose} content={content} />
  );

  if (isHandset) {
    return (
      <div className="handset-deck text-workspace-handset">
        <div className="handset-chassis">{panel}</div>
      </div>
    );
  }

  return (
    <div className="text-workspace-layout">
      {panel}
      <ZipsPanel zipsContent={zipsContent} />
    </div>
  );
}
