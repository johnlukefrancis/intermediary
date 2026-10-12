// Path: app/src/components/system_load/process_list.tsx
// Description: TOP PROGRAMS list: grouped label, ×N badge, value and a micro-bar scaled to the largest row

import type React from "react";
import type { ProcessGroupLoad } from "../../shared/protocol_system_load.js";
import { MicroBar } from "./panel_parts.js";

const TOP_COUNT = 6;

export interface ProcessColumn {
  measure: (process: ProcessGroupLoad) => number;
  format: (value: number) => string;
  secondary?: (process: ProcessGroupLoad) => string | null;
}

export function ProcessList({ processes, column }: {
  processes: readonly ProcessGroupLoad[];
  column: ProcessColumn;
}): React.JSX.Element {
  const rows = processes
    .map((process) => ({ process, value: column.measure(process) }))
    .filter((row) => row.value > 0)
    .sort((a, b) => b.value - a.value)
    .slice(0, TOP_COUNT);
  const largest = rows[0]?.value ?? 0;
  return (
    <section className="sysload-top">
      <h3 className="sysload-top__title">Top programs</h3>
      {rows.length === 0 ? <p className="sysload-empty">No measurable activity</p> : (
        <ol className="sysload-procs">
          {rows.map(({ process, value }) => {
            const secondary = column.secondary?.(process) ?? null;
            return (
              <li key={process.name} className="sysload-proc" title={process.name}>
                <span className="sysload-proc__name">
                  <span className="sysload-proc__label">{process.label || process.name}</span>
                  {process.count > 1 ? <span className="sysload-proc__count">×{process.count}</span> : null}
                </span>
                <MicroBar percent={largest > 0 ? (value / largest) * 100 : 0} />
                <span className="sysload-proc__value">
                  {column.format(value)}
                  {secondary ? <span className="sysload-proc__secondary">{secondary}</span> : null}
                </span>
              </li>
            );
          })}
        </ol>
      )}
    </section>
  );
}
