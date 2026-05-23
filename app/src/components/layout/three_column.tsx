// Path: app/src/components/layout/three_column.tsx
// Description: Standard layout component with Auto files and zip bundles

import type React from "react";
import "../../styles/columns.css";

interface ThreeColumnProps {
  fileContent?: React.ReactNode;
  zipsContent?: React.ReactNode;
}

export function ThreeColumn({
  fileContent,
  zipsContent,
}: ThreeColumnProps): React.JSX.Element {
  return (
    <div className="three-column">
      {fileContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
      <section className="panel" data-panel="zips">
        <div className="panel-content">
          {zipsContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
        </div>
      </section>
    </div>
  );
}
