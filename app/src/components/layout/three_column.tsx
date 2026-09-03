// Path: app/src/components/layout/three_column.tsx
// Description: Standard layout component with Auto files and the right rail

import type React from "react";
import "../../styles/columns.css";

interface ThreeColumnProps {
  fileContent?: React.ReactNode;
  railContent?: React.ReactNode;
}

export function ThreeColumn({
  fileContent,
  railContent,
}: ThreeColumnProps): React.JSX.Element {
  return (
    <div className="three-column">
      {fileContent ?? <p className="empty-state empty-state--waiting">Waiting for agent</p>}
      {railContent ?? (
        <section className="panel" data-panel="rail">
          <div className="panel-content">
            <p className="empty-state empty-state--waiting">Waiting for agent</p>
          </div>
        </section>
      )}
    </div>
  );
}
