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
      <section className="column">
        <h2 className="column-header">Docs</h2>
        <div className="column-content">
          {docsContent ?? <p className="placeholder">Waiting for agent...</p>}
        </div>
      </section>
      <section className="column">
        <h2 className="column-header">Code</h2>
        <div className="column-content">
          {codeContent ?? <p className="placeholder">Waiting for agent...</p>}
        </div>
      </section>
      <section className="column">
        <h2 className="column-header">Zips</h2>
        <div className="column-content">
          {zipsContent ?? <p className="placeholder">Waiting for agent...</p>}
        </div>
      </section>
    </div>
  );
}
