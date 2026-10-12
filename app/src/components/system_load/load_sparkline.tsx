// Path: app/src/components/system_load/load_sparkline.tsx
// Description: Slim 60 s trend line for a detail panel; samples enter at the right edge and age leftwards

import type React from "react";
import { HISTORY_LENGTH } from "../../lib/system_load/load_state.js";

const WIDTH = 240;
const HEIGHT = 28;

export interface SparkSeries {
  values: readonly number[];
  tone: "primary" | "secondary";
}

function pointsOf(values: readonly number[], max: number): string {
  const offset = HISTORY_LENGTH - values.length;
  return values
    .map((value, index) => {
      const x = ((offset + index) / (HISTORY_LENGTH - 1)) * WIDTH;
      const y = HEIGHT - (Math.min(max, Math.max(0, value)) / max) * (HEIGHT - 2) - 1;
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

export function LoadSparkline({ series, max }: { series: readonly SparkSeries[]; max: number }): React.JSX.Element {
  const safeMax = max > 0 ? max : 1;
  return (
    <svg className="sysload-spark" viewBox={`0 0 ${WIDTH} ${HEIGHT}`} preserveAspectRatio="none" aria-hidden="true">
      <line className="sysload-spark__track" x1={0} y1={HEIGHT - 0.5} x2={WIDTH} y2={HEIGHT - 0.5} />
      {series.map(({ values, tone }) => {
        if (values.length < 2) return null;
        const line = pointsOf(values, safeMax);
        const firstX = (((HISTORY_LENGTH - values.length) / (HISTORY_LENGTH - 1)) * WIDTH).toFixed(2);
        return (
          <g key={tone} data-tone={tone}>
            <polygon className="sysload-spark__area" points={`${firstX},${HEIGHT} ${line} ${WIDTH},${HEIGHT}`} />
            <polyline className="sysload-spark__line" points={line} />
          </g>
        );
      })}
    </svg>
  );
}
