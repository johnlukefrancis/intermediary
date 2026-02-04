// Path: app/src/components/options_overlay.tsx
// Description: Full-screen transparent overlay with options panel for app settings

import type React from "react";
import { useCallback, useMemo } from "react";
import { createPortal } from "react-dom";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import type { GlobalExcludes } from "../shared/global_excludes.js";
import type { RepoConfig, TabTheme, ThemeMode } from "../shared/config.js";
import type { AppPaths } from "../types/app_paths.js";
import { AgentSection } from "./options/agent_section.js";
import { ExcludesSection } from "./options/excludes_section.js";
import { GeneralSection } from "./options/general_section.js";
import { OutputFolderSection } from "./options/output_folder_section.js";
import { ResetSection } from "./options/reset_section.js";
import { ThemeSection } from "./options/theme_section.js";
import "../styles/options_overlay.css";

interface OptionsOverlayProps {
  autoStageOnChange: boolean;
  setAutoStageOnChange: (value: boolean) => void;
  agentAutoStart: boolean;
  setAgentAutoStart: (value: boolean) => void;
  agentDistro: string | null;
  setAgentDistro: (value: string | null) => void;
  restartAgent: () => void;
  appPaths: AppPaths | null;
  globalExcludes: GlobalExcludes;
  setGlobalExcludes: (excludes: GlobalExcludes) => void;
  setOutputWindowsRoot: (path: string | null) => void;
  recentFilesLimit: number;
  setRecentFilesLimit: (value: number) => void;
  repos: RepoConfig[];
  tabThemes: Record<string, TabTheme>;
  themeMode: ThemeMode;
  setThemeMode: (mode: ThemeMode) => void;
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
  agentDistro,
  setAgentDistro,
  restartAgent,
  appPaths,
  globalExcludes,
  setGlobalExcludes,
  setOutputWindowsRoot,
  recentFilesLimit,
  setRecentFilesLimit,
  repos,
  tabThemes,
  themeMode,
  setThemeMode,
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
      <div className="options-panel">
        <button
          type="button"
          className="options-close-button"
          onClick={handleClose}
          aria-label="Close options"
        >
          ×
        </button>

        <GeneralSection
          autoStageOnChange={autoStageOnChange}
          setAutoStageOnChange={setAutoStageOnChange}
          recentFilesLimit={recentFilesLimit}
          setRecentFilesLimit={setRecentFilesLimit}
        />

        <AgentSection
          agentAutoStart={agentAutoStart}
          setAgentAutoStart={setAgentAutoStart}
          agentDistro={agentDistro}
          setAgentDistro={setAgentDistro}
          restartAgent={restartAgent}
        />

        {/* Excludes Section */}
        <ExcludesSection
          globalExcludes={globalExcludes}
          setGlobalExcludes={setGlobalExcludes}
        />

        <OutputFolderSection
          appPaths={appPaths}
          onChooseOutputFolder={() => void handleChooseOutputFolder()}
          onOpenOutputFolder={() => void handleOpenOutputFolder()}
        />

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

        <ResetSection resetConfig={resetConfig} />
      </div>
    </div>,
    document.body
  );
}
