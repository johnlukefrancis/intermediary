// Path: app/src/components/layout/three_column.tsx
// Description: Three-column layout component with modular deck panels (Docs | Code | Zips)

import type React from "react";
import "../../styles/columns.css";

interface ThreeColumnProps {
  docsContent?: React.ReactNode;
  codeContent?: React.ReactNode;
  zipsContent?: React.ReactNode;
}

export function ThreeColumn({
  docsContent,
  codeContent,
  zipsContent,
}: ThreeColumnProps): React.JSX.Element {
  return (
    <div className="three-column">
      <section className="panel" data-panel="docs">
        <header className="panel-header">
          <span className="panel-title">Docs</span>
          <span className="panel-cue" aria-hidden="true" />
        </header>
        <div className="panel-content">
          {docsContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
      <section className="panel" data-panel="code">
        <header className="panel-header">
          <span className="panel-title">Code</span>
          <span className="panel-cue" aria-hidden="true" />
        </header>
        <div className="panel-content">
          {codeContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
      <section className="panel" data-panel="zips">
        <header className="panel-header">
          <span className="panel-title">Zips</span>
          <span className="panel-cue" aria-hidden="true" />
        </header>
        <div className="panel-content">
          {zipsContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
    </div>
  );
}
