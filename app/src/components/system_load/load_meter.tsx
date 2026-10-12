// Path: app/src/components/system_load/load_meter.tsx
// Description: One strip meter: wide-tracked label, segmented LED bar with 5 s peak hold, tabular value

import type React from "react";
import { litSegments, SEGMENT_COUNT, segmentTone, type MeterKey, type MeterReading } from "../../lib/system_load/meters.js";
import { LOAD_PANEL_ID } from "./load_panel.js";

const SEGMENT_INDEXES = Array.from({ length: SEGMENT_COUNT }, (_, index) => index);

function LedBar({ level, peak }: { level: number; peak: number }): React.JSX.Element {
  const lit = litSegments(level);
  const peakIndex = litSegments(peak) - 1;
  return (
    <span className="sysload-led" aria-hidden="true">
      {SEGMENT_INDEXES.map((index) => (
        <span
          key={index}
          className="sysload-led__seg"
          data-tone={segmentTone(index)}
          data-state={index < lit ? "lit" : index === peakIndex ? "peak" : undefined}
        />
      ))}
    </span>
  );
}

interface LoadMeterProps {
  reading: MeterReading;
  open: boolean;
  registerAnchor: (key: MeterKey, node: HTMLButtonElement | null) => void;
  onEnter: (key: MeterKey) => void;
  onFocus: (key: MeterKey) => void;
  onClick: (key: MeterKey) => void;
  onLeave: () => void;
  onBlur: () => void;
}

export function LoadMeter({
  reading, open, registerAnchor, onEnter, onFocus, onClick, onLeave, onBlur,
}: LoadMeterProps): React.JSX.Element {
  const { key } = reading;
  return (
    <button
      type="button"
      ref={(node) => { registerAnchor(key, node); }}
      className="sysload-meter"
      data-meter={key}
      data-severity={reading.severity}
      data-open={open ? "true" : undefined}
      aria-label={reading.spoken}
      aria-expanded={open}
      aria-controls={open ? LOAD_PANEL_ID : undefined}
      onPointerEnter={() => { onEnter(key); }}
      onPointerLeave={onLeave}
      onFocus={() => { onFocus(key); }}
      onBlur={onBlur}
      onClick={() => { onClick(key); }}
    >
      <span className="sysload-meter__label">{reading.label}</span>
      <LedBar level={reading.level} peak={reading.peak} />
      <span className="sysload-meter__value">{reading.value}</span>
    </button>
  );
}
