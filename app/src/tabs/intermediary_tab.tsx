// Path: app/src/tabs/intermediary_tab.tsx
// Description: Intermediary project tab

import type React from "react";
import { ThreeColumn } from "../components/layout/three_column";

export function IntermediaryTab(): React.JSX.Element {
  return (
    <div className="tab intermediary-tab">
      <ThreeColumn />
    </div>
  );
}
