// Path: app/src/components/system_load/load_meter.tsx
// Description: One strip meter: wide-tracked label and a segmented LED bar tinted by the meter's severity

import type React from "react";
import { litSegments, SEGMENT_COUNT, type MeterKey, type MeterReading } from "../../lib/system_load/meters.js";
import { LOAD_PANEL_ID } from "./load_panel.js";

const SEGMENT_INDEXES = Array.from({ length: SEGMENT_COUNT }, (_, index) => index);

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
  const lit = litSegments(reading.level);
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
      <span className="sysload-led" aria-hidden="true">
        {SEGMENT_INDEXES.map((index) => (
          <span key={index} className="sysload-led__seg" data-lit={index < lit ? "true" : undefined} />
        ))}
      </span>
    </button>
  );
}
