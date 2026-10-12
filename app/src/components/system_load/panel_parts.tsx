// Path: app/src/components/system_load/panel_parts.tsx
// Description: Detail-panel headline ("how much"), severity verdict line and shared micro-bar

import type React from "react";
import { clampPercent } from "../../lib/system_load/format.js";
import { verdict, type MeterKey } from "../../lib/system_load/meters.js";
import type { LoadSeverity } from "../../lib/system_load/severity.js";

export function PanelHeadline({ meterKey, severity, value, aside, warning }: {
  meterKey: MeterKey;
  severity: LoadSeverity;
  value: string;
  aside?: string;
  warning?: string | null;
}): React.JSX.Element {
  return (
    <div className="sysload-headline">
      <div className="sysload-headline__line">
        <span className="sysload-headline__value">{value}</span>
        {aside ? <span className="sysload-headline__aside" title={aside}>{aside}</span> : null}
      </div>
      <div className="sysload-verdict">{verdict(meterKey, severity)}</div>
      {warning ? <div className="sysload-warning">{warning}</div> : null}
    </div>
  );
}

export function MicroBar({ percent }: { percent: number }): React.JSX.Element {
  return (
    <span className="sysload-micro-bar" aria-hidden="true">
      <span className="sysload-micro-bar__fill" style={{ width: `${clampPercent(percent)}%` }} />
    </span>
  );
}
