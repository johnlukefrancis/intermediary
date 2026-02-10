// Path: app/src/hooks/use_config_actions_extended.ts
// Description: Extended config actions for theme, starred files, and recent files limit

import { useCallback, type Dispatch, type SetStateAction } from "react";
import {
  type PersistedConfig,
  type StarredFilesEntry,
  type ThemeMode,
  type UiMode,
} from "../shared/config.js";
import { DEFAULT_ACCENT_HEX } from "../lib/theme/accent_utils.js";

type SetConfig = Dispatch<SetStateAction<PersistedConfig>>;
type SaveConfig = (config: PersistedConfig) => void;

export function useSetThemeMode(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (mode: ThemeMode) => void {
  return useCallback(
    (mode: ThemeMode) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          themeMode: mode,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetUiMode(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (mode: UiMode) => void {
  return useCallback(
    (mode: UiMode) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          uiMode: mode,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetOutputWindowsRoot(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (path: string | null) => void {
  return useCallback(
    (path: string | null) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          outputWindowsRoot: path,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetAgentAutoStart(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (value: boolean) => void {
  return useCallback(
    (value: boolean) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          agentAutoStart: value,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetAgentDistro(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (value: string | null) => void {
  return useCallback(
    (value: string | null) => {
      const trimmed = value?.trim() ?? "";
      const nextValue = trimmed.length > 0 ? trimmed : null;
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          agentDistro: nextValue,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetTabThemeAccent(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (tabKey: string, accentHex: string) => void {
  return useCallback(
    (tabKey: string, accentHex: string) => {
      setConfig((prev) => {
        const existing = prev.tabThemes[tabKey];
        const next: PersistedConfig = {
          ...prev,
          tabThemes: {
            ...prev.tabThemes,
            [tabKey]: {
              accentHex,
              textureId: existing?.textureId,
            },
          },
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetTabThemeTexture(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (tabKey: string, textureId: string) => void {
  return useCallback(
    (tabKey: string, textureId: string) => {
      setConfig((prev) => {
        const existing = prev.tabThemes[tabKey];
        const next: PersistedConfig = {
          ...prev,
          tabThemes: {
            ...prev.tabThemes,
            [tabKey]: {
              accentHex: existing?.accentHex ?? DEFAULT_ACCENT_HEX,
              textureId,
            },
          },
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useClearTabTheme(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (tabKey: string) => void {
  return useCallback(
    (tabKey: string) => {
      setConfig((prev) => {
        const { [tabKey]: _removed, ...remaining } = prev.tabThemes;
        const next: PersistedConfig = {
          ...prev,
          tabThemes: remaining,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetRecentFilesLimit(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (value: number) => void {
  return useCallback(
    (value: number) => {
      if (!Number.isFinite(value)) {
        return;
      }
      const clamped = Math.max(25, Math.min(2000, Math.round(value)));
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          recentFilesLimit: clamped,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useToggleStarredFile(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repoId: string, kind: "docs" | "code", path: string) => void {
  return useCallback(
    (repoId: string, kind: "docs" | "code", path: string) => {
      setConfig((prev) => {
        const repoEntry: StarredFilesEntry = prev.starredFiles[repoId] ?? {
          docs: [],
          code: [],
        };
        const currentList = repoEntry[kind];
        const existingIndex = currentList.indexOf(path);

        let newList: string[];
        if (existingIndex >= 0) {
          // Remove - unstar
          newList = currentList.filter((p) => p !== path);
        } else {
          // Add at front - most recently starred first (MRU order)
          newList = [path, ...currentList];
        }

        const newEntry: StarredFilesEntry = {
          ...repoEntry,
          [kind]: newList,
        };

        // Clean up repo key if both lists are empty
        let newStarredFiles: PersistedConfig["starredFiles"];
        if (newEntry.docs.length === 0 && newEntry.code.length === 0) {
          const { [repoId]: _removed, ...rest } = prev.starredFiles;
          newStarredFiles = rest;
        } else {
          newStarredFiles = {
            ...prev.starredFiles,
            [repoId]: newEntry,
          };
        }

        const next: PersistedConfig = {
          ...prev,
          starredFiles: newStarredFiles,
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}
