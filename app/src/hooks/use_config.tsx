// Path: app/src/hooks/use_config.tsx
// Description: Config persistence context provider and hook

import {
  createContext,
  useContext,
  useCallback,
  type ReactNode,
} from "react";
import {
  type PersistedConfig,
  type BundleSelection,
  type RepoConfig,
  type GlobalExcludes,
} from "../shared/config.js";
import { DEFAULT_ACCENT_HEX } from "../lib/theme/accent_utils.js";
import { useConfigStorage } from "./use_config_storage.js";

interface ConfigContextValue {
  /** Current config (always available, defaults if load failed) */
  config: PersistedConfig;
  /** Whether initial load has completed */
  isLoaded: boolean;
  /** Error message if load failed */
  loadError: string | null;
  /** Update auto-stage global setting */
  setAutoStageGlobal: (value: boolean) => void;
  /** Update last active tab (by repoId) */
  setLastActiveTabId: (repoId: string | null) => void;
  /** Update bundle selection for a repo/preset */
  setBundleSelection: (
    repoId: string,
    presetId: string,
    selection: BundleSelection
  ) => void;
  /** Update global excludes (extensions and patterns) */
  setGlobalExcludes: (excludes: GlobalExcludes) => void;
  /** Add a new repo to config */
  addRepo: (repo: RepoConfig) => void;
  /** Update an existing repo's fields (e.g., to add groupId/groupLabel) */
  updateRepo: (
    repoId: string,
    updates: Partial<Omit<RepoConfig, "repoId">>
  ) => void;
  /** Remove a repo by repoId (also cleans up bundleSelections) */
  removeRepo: (repoId: string) => void;
  /** Set custom output folder override (null to reset to default) */
  setOutputWindowsRoot: (path: string | null) => void;
  /** Set accent color for a tab */
  setTabThemeAccent: (tabKey: string, accentHex: string) => void;
  /** Set texture for a tab */
  setTabThemeTexture: (tabKey: string, textureId: string) => void;
  /** Clear theme for a tab */
  clearTabTheme: (tabKey: string) => void;
}

const ConfigContext = createContext<ConfigContextValue | null>(null);

interface ConfigProviderProps {
  children: ReactNode;
}

export function ConfigProvider({
  children,
}: ConfigProviderProps): React.JSX.Element {
  const {
    config,
    setConfig,
    isLoaded,
    loadError,
    saveConfig,
    saveConfigNow,
  } = useConfigStorage();

  const setAutoStageGlobal = useCallback(
    (value: boolean) => {
      setConfig((prev) => {
        const next = { ...prev, autoStageGlobal: value };
        saveConfig(next);
        return next;
      });
    },
    [saveConfig]
  );

  const setLastActiveTabId = useCallback(
    (repoId: string | null) => {
      setConfig((prev) => {
        const next = {
          ...prev,
          uiState: { ...prev.uiState, lastActiveTabId: repoId },
        };
        saveConfig(next);
        return next;
      });
    },
    [saveConfig]
  );

  const setBundleSelection = useCallback(
    (repoId: string, presetId: string, selection: BundleSelection) => {
      setConfig((prev) => {
        const repoSelections = prev.bundleSelections[repoId] ?? {};
        const next: PersistedConfig = {
          ...prev,
          bundleSelections: {
            ...prev.bundleSelections,
            [repoId]: {
              ...repoSelections,
              [presetId]: selection,
            },
          },
        };
        // Persist bundle selections immediately and cancel pending saves.
        saveConfigNow(next);
        return next;
      });
    },
    [saveConfigNow]
  );

  const setGlobalExcludes = useCallback(
    (excludes: GlobalExcludes) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          globalExcludes: excludes,
        };
        saveConfig(next);
        return next;
      });
    },
    [saveConfig]
  );

  const addRepo = useCallback(
    (repo: RepoConfig) => {
      setConfig((prev) => {
        const next: PersistedConfig = {
          ...prev,
          repos: [...prev.repos, repo],
        };
        saveConfig(next);
        return next;
      });
    },
    [saveConfig]
  );

  const updateRepo = useCallback(
    (repoId: string, updates: Partial<Omit<RepoConfig, "repoId">>) => {
      setConfig((prev) => {
        const newRepos = prev.repos.map((r) =>
          r.repoId === repoId ? { ...r, ...updates } : r
        );
        const next: PersistedConfig = { ...prev, repos: newRepos };
        saveConfig(next);
        return next;
      });
    },
    [saveConfig]
  );

  const removeRepo = useCallback(
    (repoId: string) => {
      setConfig((prev) => {
        // Remove from repos array
        const newRepos = prev.repos.filter((r) => r.repoId !== repoId);

        // Clean up bundleSelections for this repo
        const { [repoId]: _removed, ...newBundleSelections } =
          prev.bundleSelections;

        // Clear lastActiveTabId if it was this repo
        const newUiState =
          prev.uiState.lastActiveTabId === repoId
            ? { ...prev.uiState, lastActiveTabId: null }
            : prev.uiState;

        // Clean up tabThemes entry for this repoId (if present)
        const { [repoId]: _removedTheme, ...newTabThemes } = prev.tabThemes;

        const next: PersistedConfig = {
          ...prev,
          repos: newRepos,
          bundleSelections: newBundleSelections,
          uiState: newUiState,
          tabThemes: newTabThemes,
        };
        saveConfig(next);
        return next;
      });
    },
    [saveConfig]
  );

  const setOutputWindowsRoot = useCallback(
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
    [saveConfig]
  );

  const setTabThemeAccent = useCallback(
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
    [saveConfig]
  );

  const setTabThemeTexture = useCallback(
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
    [saveConfig]
  );

  const clearTabTheme = useCallback(
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
    [saveConfig]
  );

  const value: ConfigContextValue = {
    config,
    isLoaded,
    loadError,
    setAutoStageGlobal,
    setLastActiveTabId,
    setBundleSelection,
    setGlobalExcludes,
    addRepo,
    updateRepo,
    removeRepo,
    setOutputWindowsRoot,
    setTabThemeAccent,
    setTabThemeTexture,
    clearTabTheme,
  };

  return (
    <ConfigContext.Provider value={value}>{children}</ConfigContext.Provider>
  );
}

export function useConfig(): ConfigContextValue {
  const context = useContext(ConfigContext);
  if (!context) {
    throw new Error("useConfig must be used within ConfigProvider");
  }
  return context;
}
