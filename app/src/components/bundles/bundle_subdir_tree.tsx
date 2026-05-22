// Path: app/src/components/bundles/bundle_subdir_tree.tsx
// Description: Nested subdirectory checkbox rows for bundle selection

import React from "react";

interface BundleSubdirTreeProps {
  parentDir: string;
  subdirs: string[];
  isParentSelected: boolean;
  excludedSubdirs: Set<string>;
  onSubdirToggle: (subdirPath: string) => void;
}

function sanitizedCheckboxId(path: string): string {
  return `subdir-checkbox-${path.replace(/[^a-zA-Z0-9]/g, "-")}`;
}

function pathDepth(path: string): number {
  return Math.min(path.split("/").filter(Boolean).length, 3);
}

function hasExcludedAncestor(path: string, excludedSubdirs: Set<string>): boolean {
  const parts = path.split("/").filter(Boolean);
  for (let index = 1; index < parts.length; index += 1) {
    const ancestor = parts.slice(0, index).join("/");
    if (excludedSubdirs.has(ancestor)) {
      return true;
    }
  }
  return false;
}

export function BundleSubdirTree({
  parentDir,
  subdirs,
  isParentSelected,
  excludedSubdirs,
  onSubdirToggle,
}: BundleSubdirTreeProps): React.JSX.Element {
  return (
    <div className="subdir-list">
      {subdirs.map((subdir) => {
        const subdirPath = `${parentDir}/${subdir}`;
        const subdirCheckboxId = sanitizedCheckboxId(subdirPath);
        const isExcluded = excludedSubdirs.has(subdirPath);
        const isInheritedExcluded = hasExcludedAncestor(subdirPath, excludedSubdirs);
        const isEnabled = isParentSelected && !isInheritedExcluded;
        const isIncluded = isEnabled && !isExcluded;
        const depthClass = `subdir-row--depth-${pathDepth(subdir)}`;

        return (
          <div
            key={subdirPath}
            className={`subdir-row ${depthClass}${!isEnabled ? " subdir-row--disabled" : ""}`}
          >
            <label className="vintage-toggle">
              <input
                id={subdirCheckboxId}
                type="checkbox"
                checked={isIncluded}
                disabled={!isEnabled}
                onChange={() => { onSubdirToggle(subdirPath); }}
              />
              <span className="vintage-toggle-track" />
            </label>
            <label
              className={`subdir-label${!isEnabled ? " disabled" : ""}`}
              htmlFor={subdirCheckboxId}
              title={subdirPath}
            >
              {subdir}
            </label>
          </div>
        );
      })}
    </div>
  );
}
