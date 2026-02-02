// Path: app/src/components/options/excludes/advanced_group.tsx
// Description: Collapsible checkbox group for advanced excludes options

import type React from "react";

interface AdvancedGroupProps {
  title: string;
  isOpen: boolean;
  onToggle: () => void;
  options: Array<{ value: string; label: string }>;
  checkedValues: string[];
  onItemToggle: (value: string, enabled: boolean) => void;
}

export function AdvancedGroup({
  title,
  isOpen,
  onToggle,
  options,
  checkedValues,
  onItemToggle,
}: AdvancedGroupProps): React.JSX.Element {
  return (
    <div className="options-advanced-group">
      <button
        type="button"
        className="options-advanced-toggle"
        onClick={onToggle}
        aria-expanded={isOpen}
      >
        <span className="options-advanced-title">{title}</span>
        <span className={`options-chevron ${isOpen ? "open" : ""}`}>▸</span>
      </button>
      {isOpen && (
        <div className="options-advanced-grid">
          {options.map((option) => {
            const checked = checkedValues.includes(option.value);
            return (
              <label key={option.value} className="options-checkbox-row">
                <input
                  type="checkbox"
                  checked={checked}
                  onChange={(event) => {
                    onItemToggle(option.value, event.target.checked);
                  }}
                />
                <span>{option.label}</span>
              </label>
            );
          })}
        </div>
      )}
    </div>
  );
}
