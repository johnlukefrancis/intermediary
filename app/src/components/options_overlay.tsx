// Path: app/src/components/options_overlay.tsx
// Description: Full-screen transparent overlay with options panel for app settings

import type React from "react";
import { useCallback, useEffect, useMemo } from "react";
import { createPortal } from "react-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { GlobalExcludes } from "../shared/global_excludes.js";
import type { RepoConfig, TabTheme, ThemeMode, UiMode } from "../shared/config.js";
import type { AppPaths } from "../types/app_paths.js";
import { AgentSection } from "./options/agent_section.js";
import { ExcludesSection } from "./options/excludes_section.js";
import { GeneralSection } from "./options/general_section.js";
import { OutputFolderSection } from "./options/output_folder_section.js";
import { ResetSection } from "./options/reset_section.js";
import { ThemeSection } from "./options/theme_section.js";
import "../styles/options_overlay.css";
import "../styles/options_layout.css";
import "../styles/options_controls.css";
import "../styles/options_theme.css";
import "../styles/options_excludes.css";

interface OptionsOverlayProps {
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  agentAutoStart: boolean;
  setAgentAutoStart: (value: boolean) => void;
  supportsWsl: boolean;
  agentDistro: string | null;
  setAgentDistro: (value: string | null) => void;
  restartAgent: () => void;
  appPaths: AppPaths | null;
  globalExcludes: GlobalExcludes;
  setGlobalExcludes: (excludes: GlobalExcludes) => void;
  classificationExcludes: GlobalExcludes;
  setClassificationExcludes: (excludes: GlobalExcludes) => void;
  setOutputWindowsRoot: (path: string | null) => void;
  recentFilesLimit: number;
  setRecentFilesLimit: (value: number) => void;
  repos: RepoConfig[];
  tabThemes: Record<string, TabTheme>;
  themeMode: ThemeMode;
  setThemeMode: (mode: ThemeMode) => void;
  uiMode: UiMode;
  setUiMode: (mode: UiMode) => void;
  setTabThemeAccent: (tabKey: string, accentHex: string) => void;
  setTabThemeTexture: (tabKey: string, textureId: string) => void;
  clearTabTheme: (tabKey: string) => void;
  renameRepoLabel: (repoId: string, label: string) => void;
  renameGroupLabel: (groupId: string, label: string) => void;
  resetConfig: () => void;
  onClose: () => void;
}

interface ThemeEntry {
  tabKey: string;
  id: string;
  label: string;
  kind: "repo" | "group";
}

export function OptionsOverlay({
  autoStageOnChange,
  setAutoStageOnChange,
  agentAutoStart,
  setAgentAutoStart,
  supportsWsl,
  agentDistro,
  setAgentDistro,
  restartAgent,
  appPaths,
  globalExcludes,
  setGlobalExcludes,
  classificationExcludes,
  setClassificationExcludes,
  setOutputWindowsRoot,
  recentFilesLimit,
  setRecentFilesLimit,
  repos,
  tabThemes,
  themeMode,
  setThemeMode,
  uiMode,
  setUiMode,
  setTabThemeAccent,
  setTabThemeTexture,
  clearTabTheme,
  renameRepoLabel,
  renameGroupLabel,
  resetConfig,
  onClose,
}: OptionsOverlayProps): React.JSX.Element {
  const handleClose = useCallback(() => {
    onClose();
  }, [onClose]);

  // Close on Escape key
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        handleClose();
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => {
      document.removeEventListener("keydown", handleKeyDown);
    };
  }, [handleClose]);

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
    if (!appPaths?.stagingHostRoot) return;
    try {
      await invoke("open_in_file_manager", {
        folderPath: appPaths.stagingHostRoot,
      });
    } catch (err) {
      console.error("[OptionsOverlay] Failed to open output folder:", err);
    }
  }, [appPaths?.stagingHostRoot]);

  // Derive theme entries from repos (grouped: one per groupId, ungrouped: one per repoId)
  const themeEntries = useMemo((): ThemeEntry[] => {
    const seenGroups = new Set<string>();
    const groupIndex = new Map<string, number>();
    const entries: ThemeEntry[] = [];

    for (const repo of repos) {
      if (repo.groupId) {
        // Grouped repo: one entry per groupId
        if (!seenGroups.has(repo.groupId)) {
          seenGroups.add(repo.groupId);
          const nextIndex = entries.length;
          entries.push({
            tabKey: repo.groupId,
            id: repo.groupId,
            label: repo.groupLabel ?? repo.groupId,
            kind: "group",
          });
          groupIndex.set(repo.groupId, nextIndex);
        } else if (repo.groupLabel) {
          const index = groupIndex.get(repo.groupId);
          if (index !== undefined && entries[index]?.label === repo.groupId) {
            entries[index] = {
              ...entries[index],
              label: repo.groupLabel,
            };
          }
        }
      } else {
        // Ungrouped repo: one entry per repoId
        entries.push({
          tabKey: repo.repoId,
          id: repo.repoId,
          label: repo.label,
          kind: "repo",
        });
      }
    }

    return entries;
  }, [repos]);

  return createPortal(
    <div className="options-overlay" onClick={handleBackdropClick}>
      <div className="options-panel" id="options-overlay-panel" role="dialog" aria-modal="false">
        <div className="options-body">
          <div className="options-column-primary">
            <ThemeSection
              entries={themeEntries}
              tabThemes={tabThemes}
              themeMode={themeMode}
              setThemeMode={setThemeMode}
              setTabThemeAccent={setTabThemeAccent}
              setTabThemeTexture={setTabThemeTexture}
              clearTabTheme={clearTabTheme}
              renameRepoLabel={renameRepoLabel}
              renameGroupLabel={renameGroupLabel}
            />

            <GeneralSection
              uiMode={uiMode}
              setUiMode={setUiMode}
              autoStageOnChange={autoStageOnChange}
              setAutoStageOnChange={setAutoStageOnChange}
              recentFilesLimit={recentFilesLimit}
              setRecentFilesLimit={setRecentFilesLimit}
            />

            <AgentSection
              agentAutoStart={agentAutoStart}
              setAgentAutoStart={setAgentAutoStart}
              supportsWsl={supportsWsl}
              agentDistro={agentDistro}
              setAgentDistro={setAgentDistro}
              restartAgent={restartAgent}
            />
          </div>

          <div className="options-column-secondary">
            <OutputFolderSection
              appPaths={appPaths}
              onChooseOutputFolder={() => void handleChooseOutputFolder()}
              onOpenOutputFolder={() => void handleOpenOutputFolder()}
            />

            {/* Bundle Excludes */}
            <ExcludesSection
              title="Bundle Excludes"
              hint="Files and folders excluded from ZIP bundles"
              recommendedLabel="Recommended bundle excludes (always applied)"
              excludes={globalExcludes}
              setExcludes={setGlobalExcludes}
            />

            {/* Classification Excludes */}
            <ExcludesSection
              title="Classification Excludes"
              hint="Hide noisy or generated files from the Docs/Code panes without affecting bundles"
              recommendedLabel="Recommended classification excludes"
              excludes={classificationExcludes}
              setExcludes={setClassificationExcludes}
            />
          </div>

          <div className="options-footer">
            <ResetSection resetConfig={resetConfig} />
          </div>
        </div>
      </div>
    </div>,
    document.body
  );
}
