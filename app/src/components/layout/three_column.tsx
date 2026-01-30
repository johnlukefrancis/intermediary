// Path: app/src/components/layout/three_column.tsx
// Description: Three-column layout component (Docs | Code | Zips)

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
      <section className="panel">
        <h2 className="panel-header">Docs</h2>
        <div className="panel-content">
          {docsContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
      <section className="panel">
        <h2 className="panel-header">Code</h2>
        <div className="panel-content">
          {codeContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
      <section className="panel">
        <h2 className="panel-header">Zips</h2>
        <div className="panel-content">
          {zipsContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
    </div>
  );
}
