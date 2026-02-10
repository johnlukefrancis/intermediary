// Path: app/src/components/options/output_folder_section.tsx
// Description: Options panel controls for staging output folder

import type React from "react";
import type { AppPaths } from "../../types/app_paths.js";
import { OptionsFieldRow } from "./layout/options_field_row.js";

interface OutputFolderSectionProps {
  appPaths: AppPaths | null;
  onChooseOutputFolder: () => void;
  onOpenOutputFolder: () => void;
}

export function OutputFolderSection({
  appPaths,
  onChooseOutputFolder,
  onOpenOutputFolder,
}: OutputFolderSectionProps): React.JSX.Element {
  return (
    <div className="options-section">
      <div
        className="options-section-title"
        title="Where bundled ZIP files are saved on disk"
      >
        Output Folder
      </div>
      <OptionsFieldRow
        label="Staging path"
        title="Where bundled ZIP files are saved on disk"
        controlAlign="stretch"
        control={(
          <div className="options-path-stack">
            <span
              className={`options-path-display ${!appPaths ? "muted" : ""}`}
              title={appPaths?.stagingHostRoot ?? "Loading..."}
            >
              {appPaths?.stagingHostRoot ?? "Loading..."}
            </span>
            <div className="options-button-row">
              <button
                type="button"
                className="options-button"
                onClick={() => {
                  onChooseOutputFolder();
                }}
                disabled={!appPaths}
              >
                Choose output folder
              </button>
              <button
                type="button"
                className="options-button"
                onClick={() => {
                  onOpenOutputFolder();
                }}
                disabled={!appPaths}
              >
                Open output folder
              </button>
            </div>
          </div>
        )}
      />
    </div>
  );
}
