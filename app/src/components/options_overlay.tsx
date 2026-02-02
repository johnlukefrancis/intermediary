// Path: app/src/components/options_overlay.tsx
// Description: Full-screen transparent overlay with options panel for app settings

import type React from "react";
import { useCallback, useMemo } from "react";
import { createPortal } from "react-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { GlobalExcludes } from "../shared/global_excludes.js";
import type { RepoConfig, TabTheme } from "../shared/config.js";
import type { AppPaths } from "../types/app_paths.js";
import { ExcludesSection } from "./options/excludes_section.js";
import { DEFAULT_ACCENT_HEX } from "../lib/theme/accent_utils.js";
import "../styles/options_overlay.css";

interface OptionsOverlayProps {
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  appPaths: AppPaths | null;
  globalExcludes: GlobalExcludes;
  setGlobalExcludes: (excludes: GlobalExcludes) => void;
  setOutputWindowsRoot: (path: string | null) => void;
  repos: RepoConfig[];
  tabThemes: Record<string, TabTheme>;
  setTabThemeAccent: (tabKey: string, accentHex: string) => void;
  clearTabTheme: (tabKey: string) => void;
  onClose: () => void;
}

interface ThemeEntry {
  tabKey: string;
  label: string;
}

export function OptionsOverlay({
  autoStageOnChange,
  setAutoStageOnChange,
  appPaths,
  globalExcludes,
  setGlobalExcludes,
  setOutputWindowsRoot,
  repos,
  tabThemes,
  setTabThemeAccent,
  clearTabTheme,
  onClose,
}: OptionsOverlayProps): React.JSX.Element {
  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  const handleBackdropClick = useCallback(
    (event: React.MouseEvent<HTMLDivElement>) => {
      if (event.target === event.currentTarget) {
        handleClose();
      }
    },
    [handleClose]
  );

  const handleChooseOutputFolder = useCallback(async () => {
    try {
      const selected = await open({ directory: true, multiple: false });
      if (selected && typeof selected === "string") {
        setOutputWindowsRoot(selected);
      }
    } catch (err) {
      console.error("[OptionsOverlay] Failed to choose output folder:", err);
    }
  }, [setOutputWindowsRoot]);

  const handleOpenOutputFolder = useCallback(async () => {
    if (!appPaths?.stagingWindowsRoot) return;
    try {
      await invoke("open_in_file_manager", {
        folderPath: appPaths.stagingWindowsRoot,
      });
    } catch (err) {
      console.error("[OptionsOverlay] Failed to open output folder:", err);
    }
  }, [appPaths?.stagingWindowsRoot]);

  // Derive theme entries from repos (grouped: one per groupId, ungrouped: one per repoId)
  const themeEntries = useMemo((): ThemeEntry[] => {
    const seenGroups = new Set<string>();
    const entries: ThemeEntry[] = [];

    for (const repo of repos) {
      if (repo.groupId) {
        // Grouped repo: one entry per groupId
        if (!seenGroups.has(repo.groupId)) {
          seenGroups.add(repo.groupId);
          entries.push({
            tabKey: repo.groupId,
            label: repo.groupLabel ?? repo.groupId,
          });
        }
      } else {
        // Ungrouped repo: one entry per repoId
        entries.push({
          tabKey: repo.repoId,
          label: repo.label,
        });
      }
    }

    return entries;
  }, [repos]);

  return createPortal(
    <div className="options-overlay" onClick={handleBackdropClick}>
      <div className="options-panel">
        <button
          type="button"
          className="options-close-button"
          onClick={handleClose}
          aria-label="Close options"
        >
          ×
        </button>

        {/* General Section */}
        <div className="options-section">
          <div className="options-section-title">General</div>
          <div className="options-row">
            <span className="options-row-label">Auto-stage</span>
            <label className="vintage-toggle">
              <input
                type="checkbox"
                checked={autoStageOnChange}
                onChange={(event) => {
                  setAutoStageOnChange(event.target.checked);
                }}
              />
              <span className="vintage-toggle-track" aria-hidden="true" />
            </label>
          </div>
        </div>

        {/* Excludes Section */}
        <ExcludesSection
          globalExcludes={globalExcludes}
          setGlobalExcludes={setGlobalExcludes}
        />

        {/* Output Folder Section */}
        <div className="options-section">
          <div className="options-section-title">Output Folder</div>
          <div className="options-row stacked">
            <span
              className={`options-path-display ${!appPaths ? "muted" : ""}`}
              title={appPaths?.stagingWindowsRoot ?? "Loading..."}
            >
              {appPaths?.stagingWindowsRoot ?? "Loading..."}
            </span>
            <div className="options-button-row">
              <button
                type="button"
                className="options-button"
                onClick={() => void handleChooseOutputFolder()}
                disabled={!appPaths}
              >
                Choose output folder
              </button>
              <button
                type="button"
                className="options-button"
                onClick={() => void handleOpenOutputFolder()}
                disabled={!appPaths}
              >
                Open output folder
              </button>
            </div>
          </div>
        </div>

        {/* Tab Colors Section */}
        {themeEntries.length > 0 && (
          <div className="options-section">
            <div className="options-section-title">Theme Colors</div>
            <div className="options-theme-list">
              {themeEntries.map((entry) => {
                const currentHex =
                  tabThemes[entry.tabKey]?.accentHex ?? DEFAULT_ACCENT_HEX;
                const hasCustomTheme = entry.tabKey in tabThemes;

                return (
                  <div key={entry.tabKey} className="options-theme-row">
                    <span
                      className="options-theme-label"
                      title={entry.label}
                    >
                      {entry.label}
                    </span>
                    <div className="options-theme-controls">
                      <input
                        type="color"
                        value={currentHex}
                        onChange={(e) => {
                          setTabThemeAccent(entry.tabKey, e.target.value);
                        }}
                        className="options-color-input"
                        title="Choose accent color"
                      />
                      {hasCustomTheme && (
                        <button
                          type="button"
                          className="options-reset-button"
                          onClick={() => {
                            clearTabTheme(entry.tabKey);
                          }}
                          title="Reset to default"
                        >
                          ×
                        </button>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}
      </div>
    </div>,
    document.body
  );
}
