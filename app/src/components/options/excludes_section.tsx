// Path: app/src/components/options/excludes_section.tsx
// Description: Excludes configuration section for the options panel

import type React from "react";
import type { GlobalExcludes } from "../../shared/global_excludes.js";
import { AdvancedGroup } from "./excludes/advanced_group.js";
import { useExcludesState } from "./excludes/use_excludes_state.js";

interface ExcludesSectionProps {
  globalExcludes: GlobalExcludes;
  setGlobalExcludes: (excludes: GlobalExcludes) => void;
}

export function ExcludesSection({
  globalExcludes,
  setGlobalExcludes,
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
  } = useExcludesState({ globalExcludes, setGlobalExcludes });

  return (
    <div className="options-section">
      <div className="options-section-title">Excludes</div>
      <label className="options-checkbox-row">
        <input
          type="checkbox"
          checked={recommendedEnabled}
          onChange={handleRecommendedToggle}
        />
        <span>Recommended excludes</span>
      </label>

      <button
        type="button"
        className="options-section-toggle"
        onClick={toggleAdvanced}
        aria-expanded={advancedOpen}
      >
        <span className="options-section-title">Exclude Presets</span>
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
