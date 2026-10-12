// Path: app/src/components/system_load/panel_parts.tsx
// Description: Shared detail-panel building blocks: readout, section, stat grid and labelled micro-bar row

import type React from "react";
import { clampPercent } from "../../lib/system_load/format.js";
import { percentSeverity, type LoadSeverity } from "../../lib/system_load/severity.js";

export function PanelReadout({ value, aside, subtitle }: {
  value: string;
  aside?: string;
  subtitle: string;
}): React.JSX.Element {
  return (
    <div className="sysload-readout">
      <div className="sysload-readout__line">
        <span className="sysload-readout__value">{value}</span>
        {aside ? <span className="sysload-readout__aside">{aside}</span> : null}
      </div>
      <div className="sysload-readout__subtitle" title={subtitle}>{subtitle}</div>
    </div>
  );
}

export function PanelSection({ title, children }: { title: string; children: React.ReactNode }): React.JSX.Element {
  return (
    <section className="sysload-section">
      <h3 className="sysload-section__title">{title}</h3>
      {children}
    </section>
  );
}

export function StatGrid({ stats }: { stats: readonly (readonly [string, string])[] }): React.JSX.Element {
  return (
    <dl className="sysload-stats">
      {stats.map(([label, value]) => (
        <div key={label} className="sysload-stats__item">
          <dt>{label}</dt>
          <dd>{value}</dd>
        </div>
      ))}
    </dl>
  );
}

/** Bar tone follows the same calm/busy/hot bands as the strip unless the caller overrides it. */
export function BarRow({ label, value, percent, severity, title }: {
  label: string;
  value: string;
  percent: number;
  severity?: LoadSeverity;
  title?: string;
}): React.JSX.Element {
  const width = clampPercent(percent);
  return (
    <div className="sysload-bar-row" title={title ?? label}>
      <span className="sysload-bar-row__label">{label}</span>
      <MicroBar percent={width} severity={severity ?? percentSeverity(width)} />
      <span className="sysload-bar-row__value">{value}</span>
    </div>
  );
}

export function MicroBar({ percent, severity }: { percent: number; severity: LoadSeverity }): React.JSX.Element {
  return (
    <span className="sysload-micro-bar" data-severity={severity} aria-hidden="true">
      <span className="sysload-micro-bar__fill" style={{ width: `${clampPercent(percent)}%` }} />
    </span>
  );
}

export function PanelEmpty({ children }: { children: React.ReactNode }): React.JSX.Element {
  return <p className="sysload-empty">{children}</p>;
}
