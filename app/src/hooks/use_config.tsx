// Path: app/src/hooks/use_config.tsx
// Description: Config persistence context provider and hook

import {
  createContext,
  useContext,
  useEffect,
  useState,
  useCallback,
  useRef,
  type ReactNode,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type PersistedConfig,
  type LoadConfigResult,
  type BundleSelection,
  getDefaultPersistedConfig,
  parsePersistedConfig,
} from "../shared/config.js";
import type { TabId, WorktreeId } from "../shared/ids.js";

/** Debounce delay for saving config (ms) */
const SAVE_DEBOUNCE_MS = 500;

interface ConfigContextValue {
  /** Current config (always available, defaults if load failed) */
  config: PersistedConfig;
  /** Whether initial load has completed */
  isLoaded: boolean;
  /** Error message if load failed */
  loadError: string | null;
  /** Update auto-stage global setting */
  setAutoStageGlobal: (value: boolean) => void;
  /** Update last active tab */
  setLastActiveTabId: (tabId: TabId | null) => void;
  /** Update last Triangle Rain worktree */
  setLastTriangleRainWorktreeId: (worktreeId: WorktreeId | null) => void;
  /** Update bundle selection for a repo/preset */
  setBundleSelection: (
    repoId: string,
    presetId: string,
    selection: BundleSelection
  ) => void;
}

const ConfigContext = createContext<ConfigContextValue | null>(null);

interface ConfigProviderProps {
  children: ReactNode;
}

export function ConfigProvider({
  children,
}: ConfigProviderProps): React.JSX.Element {
  const [config, setConfig] = useState<PersistedConfig>(() =>
    getDefaultPersistedConfig()
  );
  const [isLoaded, setIsLoaded] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  // Ref to track pending save and current config for beforeunload
  const saveTimeoutRef = useRef<number | null>(null);
  const configRef = useRef(config);
  const isDirtyRef = useRef(false);

  // Keep configRef in sync
  useEffect(() => {
    configRef.current = config;
  }, [config]);

  // Save config to disk (debounced)
  const saveConfig = useCallback((newConfig: PersistedConfig) => {
    // Clear pending save
    if (saveTimeoutRef.current !== null) {
      clearTimeout(saveTimeoutRef.current);
    }

    isDirtyRef.current = true;

    // Schedule save
    saveTimeoutRef.current = window.setTimeout(() => {
      saveTimeoutRef.current = null;
      isDirtyRef.current = false;

      invoke("save_config", { config: newConfig }).catch((err: unknown) => {
        console.error("[ConfigProvider] Failed to save config:", err);
      });
    }, SAVE_DEBOUNCE_MS);
  }, []);

  // Save immediately (for beforeunload)
  const saveConfigImmediate = useCallback(() => {
    if (!isDirtyRef.current) return;

    // Clear pending debounced save
    if (saveTimeoutRef.current !== null) {
      clearTimeout(saveTimeoutRef.current);
      saveTimeoutRef.current = null;
    }

    isDirtyRef.current = false;

    // Synchronous save attempt via sendBeacon or blocking invoke
    // Note: invoke is async but we try anyway on unload
    invoke("save_config", { config: configRef.current }).catch(() => {
      // Ignore errors on unload
    });
  }, []);

  // Load config on mount
  useEffect(() => {
    let mounted = true;

    async function loadConfig(): Promise<void> {
      try {
        const result = await invoke<LoadConfigResult>("load_config");
        if (!mounted) return;

        // Validate and migrate if needed
        const validConfig = parsePersistedConfig(result.config);
        setConfig(validConfig);
        setIsLoaded(true);

        if (result.wasCreated) {
          console.log("[ConfigProvider] Created new config with defaults");
        }
        if (result.migrationApplied) {
          console.log("[ConfigProvider] Applied config migration");
          // Save migrated config
          saveConfig(validConfig);
        }
      } catch (err: unknown) {
        if (!mounted) return;

        const message =
          err instanceof Error ? err.message : "Failed to load config";
        console.error("[ConfigProvider] Load failed:", err);
        setLoadError(message);
        setIsLoaded(true);
        // Keep defaults
      }
    }

    void loadConfig();

    return () => {
      mounted = false;
    };
  }, [saveConfig]);

  // Save on beforeunload
  useEffect(() => {
    const handleBeforeUnload = (): void => {
      saveConfigImmediate();
    };

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", handleBeforeUnload);
    };
  }, [saveConfigImmediate]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, []);

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
    (tabId: TabId | null) => {
      setConfig((prev) => {
        const next = {
          ...prev,
          uiState: { ...prev.uiState, lastActiveTabId: tabId },
        };
        saveConfig(next);
        return next;
      });
    },
    [saveConfig]
  );

  const setLastTriangleRainWorktreeId = useCallback(
    (worktreeId: WorktreeId | null) => {
      setConfig((prev) => {
        const next = {
          ...prev,
          uiState: { ...prev.uiState, lastTriangleRainWorktreeId: worktreeId },
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
    setLastTriangleRainWorktreeId,
    setBundleSelection,
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
