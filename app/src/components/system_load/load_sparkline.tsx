// Path: app/src/components/system_load/load_sparkline.tsx
// Description: 60 s area chart for a detail panel; samples enter at the right edge and age leftwards

import type React from "react";
import { HISTORY_LENGTH } from "../../lib/system_load/load_state.js";

const WIDTH = 240;
const HEIGHT = 48;

export interface SparkSeries {
  values: readonly number[];
  tone: "primary" | "secondary";
}

function pointsOf(values: readonly number[], max: number): string {
  const offset = HISTORY_LENGTH - values.length;
  return values
    .map((value, index) => {
      const x = ((offset + index) / (HISTORY_LENGTH - 1)) * WIDTH;
      const y = HEIGHT - (Math.min(max, Math.max(0, value)) / max) * HEIGHT;
      return `${x.toFixed(2)},${y.toFixed(2)}`;
    })
    .join(" ");
}

export function LoadSparkline({ series, max, scaleLabel }: {
  series: readonly SparkSeries[];
  max: number;
  scaleLabel: string;
}): React.JSX.Element {
  const safeMax = max > 0 ? max : 1;
  return (
    <figure className="sysload-spark">
      <span className="sysload-spark__scale">{scaleLabel}</span>
      <svg
        className="sysload-spark__chart"
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        preserveAspectRatio="none"
        aria-hidden="true"
      >
        {[0.25, 0.5, 0.75].map((fraction) => (
          <line key={fraction} className="sysload-spark__grid" x1="0" x2={WIDTH} y1={HEIGHT * fraction} y2={HEIGHT * fraction} />
        ))}
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
      <figcaption className="sysload-spark__axis">
        <span>60 s</span>
        <span>now</span>
      </figcaption>
    </figure>
  );
}
