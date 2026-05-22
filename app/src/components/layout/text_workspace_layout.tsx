// Path: app/src/components/layout/text_workspace_layout.tsx
// Description: Layout that replaces Docs and Code panes with a full text workspace

import type React from "react";
import "../../styles/handset_deck.css";
import "../../styles/handset_chassis.css";

interface TextWorkspaceLayoutProps {
  title: string;
  subtitle?: string;
  onClose: () => void;
  editorContent: React.ReactNode;
  zipsContent: React.ReactNode;
  isHandset: boolean;
}

interface TextWorkspacePanelProps {
  title: string;
  subtitle: string | undefined;
  onClose: () => void;
  editorContent: React.ReactNode;
}

function TextWorkspacePanel({
  title,
  subtitle,
  onClose,
  editorContent,
}: TextWorkspacePanelProps): React.JSX.Element {
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
          title="Close text editor"
          aria-label="Close text editor"
        >
          ×
        </button>
      </header>
      <div className="panel-content text-workspace-content">{editorContent}</div>
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

export function TextWorkspaceLayout({
  title,
  subtitle,
  onClose,
  editorContent,
  zipsContent,
  isHandset,
}: TextWorkspaceLayoutProps): React.JSX.Element {
  const panel = (
    <TextWorkspacePanel
      title={title}
      subtitle={subtitle}
      onClose={onClose}
      editorContent={editorContent}
    />
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
