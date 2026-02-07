// Path: app/src/components/options/excludes_section.tsx
// Description: Excludes configuration section for the options panel

import type React from "react";
import type { GlobalExcludes } from "../../shared/global_excludes.js";
import { AdvancedGroup } from "./excludes/advanced_group.js";
import { useExcludesState } from "./excludes/use_excludes_state.js";

interface ExcludesSectionProps {
  title?: string;
  hint?: string;
  recommendedLabel?: string;
  excludes: GlobalExcludes;
  setExcludes: (excludes: GlobalExcludes) => void;
}

export function ExcludesSection({
  title = "Excludes",
  hint,
  recommendedLabel = "Recommended excludes",
  excludes,
  setExcludes,
}: ExcludesSectionProps): React.JSX.Element {
  const {
    advancedOpen,
    advancedSectionsOpen,
    toggleAdvanced,
    toggleAdvancedSection,
    recommendedEnabled,
    handleRecommendedToggle,
    normalizedValues,
    toggleHandlers,
    optionsSections,
  } = useExcludesState({ excludes, setExcludes });

  return (
    <div className="options-section">
      <div className="options-section-title" title={hint}>{title}</div>
      <label className="options-checkbox-row">
        <input
          type="checkbox"
          checked={recommendedEnabled}
          onChange={handleRecommendedToggle}
        />
        <span>{recommendedLabel}</span>
      </label>

      <button
        type="button"
        className="options-section-toggle"
        onClick={toggleAdvanced}
        aria-expanded={advancedOpen}
      >
        <span className="options-section-title">Advanced Excludes</span>
        <span className={`options-chevron ${advancedOpen ? "open" : ""}`}>
          ▸
        </span>
      </button>

      {advancedOpen && (
        <div className="options-section-content">
          {optionsSections.map((section) => {
            const handleToggle = toggleHandlers[section.onToggle];
            return (
              <AdvancedGroup
                key={section.key}
                title={section.title}
                isOpen={advancedSectionsOpen[section.key]}
                onToggle={() => {
                  toggleAdvancedSection(section.key);
                }}
                options={section.options}
                checkedValues={section.select(normalizedValues)}
                onItemToggle={handleToggle}
              />
            );
          })}
        </div>
      )}
    </div>
  );
}
