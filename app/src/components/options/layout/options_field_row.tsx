// Path: app/src/components/options/layout/options_field_row.tsx
// Description: Shared label/control row primitive for responsive options fields

import type React from "react";

type ControlAlign = "start" | "end" | "stretch";

interface OptionsFieldRowProps {
  label: React.ReactNode;
  control: React.ReactNode;
  title?: string;
  hint?: React.ReactNode;
  controlAlign?: ControlAlign;
  className?: string;
}

export function OptionsFieldRow({
  label,
  control,
  title,
  hint,
  controlAlign = "end",
  className,
}: OptionsFieldRowProps): React.JSX.Element {
  const rootClassName = className
    ? `options-field-row ${className}`
    : "options-field-row";
  const controlClassName = `options-field-row__control options-field-row__control--${controlAlign}`;

  return (
    <div className={rootClassName} title={title}>
      <div className="options-field-row__meta">
        <span className="options-field-row__label">{label}</span>
        {hint ? <span className="options-field-row__hint">{hint}</span> : null}
      </div>
      <div className={controlClassName}>{control}</div>
    </div>
  );
}
