// Path: app/src/components/options/controls/tri_state_rocker.tsx
// Description: Reusable three-state hardware-style rocker control for options

import type React from "react";

export interface TriStateOption<T extends string> {
  value: T;
  label: string;
  icon?: string;
  title?: string;
}

interface TriStateRockerProps<T extends string> {
  value: T;
  options: readonly TriStateOption<T>[];
  onChange: (value: T) => void;
  ariaLabel: string;
  className?: string;
}

function resolveIndex<T extends string>(
  options: readonly TriStateOption<T>[],
  value: T
): number {
  const index = options.findIndex((option) => option.value === value);
  return index >= 0 ? index : 0;
}

export function TriStateRocker<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
  className,
}: TriStateRockerProps<T>): React.JSX.Element {
  const rootClassName = className
    ? `tri-state-rocker ${className}`
    : "tri-state-rocker";

  return (
    <div className={rootClassName} role="radiogroup" aria-label={ariaLabel}>
      {options.map((option) => {
        const isActive = option.value === value;
        const optionClassName = isActive
          ? "tri-state-rocker__option tri-state-rocker__option--active"
          : "tri-state-rocker__option";

        return (
          <button
            key={option.value}
            type="button"
            role="radio"
            aria-checked={isActive}
            aria-label={option.title ?? option.label}
            data-rocker-value={option.value}
            tabIndex={isActive ? 0 : -1}
            className={optionClassName}
            title={option.title ?? option.label}
            onClick={() => {
              onChange(option.value);
            }}
            onKeyDown={(event) => {
              const activeIndex = resolveIndex(options, value);
              let targetIndex: number | null = null;

              if (event.key === "ArrowRight" || event.key === "ArrowDown") {
                targetIndex = (activeIndex + 1) % options.length;
              } else if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
                targetIndex = (activeIndex - 1 + options.length) % options.length;
              } else if (event.key === "Home") {
                targetIndex = 0;
              } else if (event.key === "End") {
                targetIndex = options.length - 1;
              } else if (event.key === "Enter" || event.key === " ") {
                event.preventDefault();
                onChange(option.value);
                return;
              }

              if (targetIndex === null) {
                return;
              }

              event.preventDefault();
              const nextValue = options[targetIndex]?.value;
              if (!nextValue) {
                return;
              }
              onChange(nextValue);

              const container = event.currentTarget.parentElement;
              const nextButton = container?.querySelector<HTMLButtonElement>(
                `[data-rocker-value="${nextValue}"]`
              );
              nextButton?.focus();
            }}
          >
            {option.icon ? (
              <span className="tri-state-rocker__icon" aria-hidden="true">
                {option.icon}
              </span>
            ) : null}
            <span className="tri-state-rocker__label">{option.label}</span>
          </button>
        );
      })}
    </div>
  );
}
