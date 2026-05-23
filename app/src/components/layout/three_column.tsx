// Path: app/src/components/layout/three_column.tsx
// Description: Three-column layout component with file feeds and zip bundles

import type React from "react";
import "../../styles/columns.css";

interface ThreeColumnProps {
  latestContent?: React.ReactNode;
  activeContent?: React.ReactNode;
  zipsContent?: React.ReactNode;
  latestHeaderLeft?: React.ReactNode;
  activeHeaderLeft?: React.ReactNode;
}

export function ThreeColumn({
  latestContent,
  activeContent,
  zipsContent,
  latestHeaderLeft,
  activeHeaderLeft,
}: ThreeColumnProps): React.JSX.Element {
  return (
    <div className="three-column">
      <section className="panel" data-panel="latest">
        <header className="panel-header">
          {latestHeaderLeft ?? <h2 className="panel-title">Latest</h2>}
        </header>
        <div className="panel-content">
          {latestContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
      <section className="panel" data-panel="active">
        <header className="panel-header">
          {activeHeaderLeft ?? <h2 className="panel-title">Active</h2>}
        </header>
        <div className="panel-content">
          {activeContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
      <section className="panel" data-panel="zips">
        <header className="panel-header">
          <h2 className="panel-title">Zips</h2>
          <span className="panel-cue" aria-hidden="true" />
        </header>
        <div className="panel-content">
          {zipsContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
    </div>
  );
}
