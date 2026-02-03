// Path: app/src/hooks/use_config.tsx
// Description: Config persistence context provider and hook

import { createContext, useContext, type ReactNode } from "react";
import {
  type PersistedConfig,
  type BundleSelection,
  type RepoConfig,
  type GlobalExcludes,
} from "../shared/config.js";
import { useConfigStorage } from "./use_config_storage.js";
import {
  useSetAutoStageGlobal,
  useSetLastActiveTabId,
  useSetBundleSelection,
  useSetGlobalExcludes,
  useAddRepo,
  useUpdateRepo,
  useRemoveRepo,
  useRemoveGroup,
} from "./use_config_actions.js";
import {
  useSetOutputWindowsRoot,
  useSetTabThemeAccent,
  useSetTabThemeTexture,
  useClearTabTheme,
  useSetRecentFilesLimit,
  useToggleStarredFile,
} from "./use_config_actions_extended.js";

interface ConfigContextValue {
  /** Current config (always available, defaults if load failed) */
  config: PersistedConfig;
  /** Whether initial load has completed */
  isLoaded: boolean;
  /** Error message if load failed */
  loadError: string | null;
  /** Error message if save failed */
  saveError: string | null;
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
  /** Remove a repo by repoId (also cleans up bundleSelections, starredFiles) */
  removeRepo: (repoId: string) => void;
  /** Remove all repos in a group (also cleans up themes and starredFiles) */
  removeGroup: (groupId: string) => void;
  /** Set custom output folder override (null to reset to default) */
  setOutputWindowsRoot: (path: string | null) => void;
  /** Set accent color for a tab */
  setTabThemeAccent: (tabKey: string, accentHex: string) => void;
  /** Set texture for a tab */
  setTabThemeTexture: (tabKey: string, textureId: string) => void;
  /** Clear theme for a tab */
  clearTabTheme: (tabKey: string) => void;
  /** Set maximum recent files to track (clamped to 25-2000) */
  setRecentFilesLimit: (value: number) => void;
  /** Toggle a file's starred status for a repo */
  toggleStarredFile: (
    repoId: string,
    kind: "docs" | "code",
    path: string
  ) => void;
  /** Reset all settings to defaults */
  resetConfig: () => void;
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
    saveError,
    saveConfig,
    saveConfigNow,
    resetConfig,
  } =
    useConfigStorage();

  const setAutoStageGlobal = useSetAutoStageGlobal(setConfig, saveConfig);
  const setLastActiveTabId = useSetLastActiveTabId(setConfig, saveConfig);
  const setBundleSelection = useSetBundleSelection(setConfig, saveConfigNow);
  const setGlobalExcludes = useSetGlobalExcludes(setConfig, saveConfig);
  const addRepo = useAddRepo(setConfig, saveConfig);
  const updateRepo = useUpdateRepo(setConfig, saveConfig);
  const removeRepo = useRemoveRepo(setConfig, saveConfig);
  const removeGroup = useRemoveGroup(setConfig, saveConfig);
  const setOutputWindowsRoot = useSetOutputWindowsRoot(setConfig, saveConfig);
  const setTabThemeAccent = useSetTabThemeAccent(setConfig, saveConfig);
  const setTabThemeTexture = useSetTabThemeTexture(setConfig, saveConfig);
  const clearTabTheme = useClearTabTheme(setConfig, saveConfig);
  const setRecentFilesLimit = useSetRecentFilesLimit(setConfig, saveConfig);
  const toggleStarredFile = useToggleStarredFile(setConfig, saveConfig);

  const value: ConfigContextValue = {
    config,
    isLoaded,
    loadError,
    saveError,
    setAutoStageGlobal,
    setLastActiveTabId,
    setBundleSelection,
    setGlobalExcludes,
    addRepo,
    updateRepo,
    removeRepo,
    removeGroup,
    setOutputWindowsRoot,
    setTabThemeAccent,
    setTabThemeTexture,
    clearTabTheme,
    setRecentFilesLimit,
    toggleStarredFile,
    resetConfig,
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
