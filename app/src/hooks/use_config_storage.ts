// Path: app/src/hooks/use_config_storage.ts
// Description: Config persistence + loading hook for use_config

import { useCallback, useEffect, useRef, useState, type Dispatch, type SetStateAction } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  type PersistedConfig,
  type LoadConfigResult,
  getDefaultPersistedConfig,
  parsePersistedConfig,
} from "../shared/config.js";

const SAVE_DEBOUNCE_MS = 500;

interface ConfigStorageState {
  config: PersistedConfig;
  setConfig: Dispatch<SetStateAction<PersistedConfig>>;
  isLoaded: boolean;
  loadError: string | null;
  saveError: string | null;
  saveConfig: (newConfig: PersistedConfig) => void;
  saveConfigNow: (newConfig: PersistedConfig) => void;
}

function getErrorMessage(err: unknown, fallback: string): string {
  if (err instanceof Error) return err.message;
  if (typeof err === "string") return err;
  return fallback;
}

export function useConfigStorage(): ConfigStorageState {
  const [config, setConfig] = useState<PersistedConfig>(() =>
    getDefaultPersistedConfig()
  );
  const [isLoaded, setIsLoaded] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  const saveTimeoutRef = useRef<number | null>(null);
  const configRef = useRef(config);
  const isDirtyRef = useRef(false);

  useEffect(() => {
    configRef.current = config;
  }, [config]);

  const saveConfig = useCallback((newConfig: PersistedConfig) => {
    if (saveTimeoutRef.current !== null) {
      clearTimeout(saveTimeoutRef.current);
    }

    isDirtyRef.current = true;

    saveTimeoutRef.current = window.setTimeout(() => {
      saveTimeoutRef.current = null;
      isDirtyRef.current = false;

      invoke("save_config", { config: newConfig })
        .then(() => {
          setSaveError(null);
        })
        .catch((err: unknown) => {
          const message = getErrorMessage(err, "Failed to save config");
          console.error("[ConfigProvider] Failed to save config:", err);
          setSaveError(message);
        });
    }, SAVE_DEBOUNCE_MS);
  }, []);

  const saveConfigNow = useCallback((newConfig: PersistedConfig) => {
    if (saveTimeoutRef.current !== null) {
      clearTimeout(saveTimeoutRef.current);
      saveTimeoutRef.current = null;
    }
    isDirtyRef.current = false;
    configRef.current = newConfig;
    invoke("save_config", { config: newConfig })
      .then(() => {
        setSaveError(null);
      })
      .catch((err: unknown) => {
        const message = getErrorMessage(err, "Failed to save config");
        console.error("[ConfigProvider] Failed to save config:", err);
        setSaveError(message);
      });
  }, []);

  const saveConfigImmediate = useCallback(() => {
    if (!isDirtyRef.current) return;
    saveConfigNow(configRef.current);
  }, [saveConfigNow]);

  useEffect(() => {
    let mounted = true;

    async function loadConfig(): Promise<void> {
      try {
        const result = await invoke<LoadConfigResult>("load_config");
        if (!mounted) return;

        const validConfig = parsePersistedConfig(result.config);
        setConfig(validConfig);
        setIsLoaded(true);

        if (result.wasCreated) {
          console.log("[ConfigProvider] Created new config with defaults");
        }
        if (result.migrationApplied) {
          console.log("[ConfigProvider] Applied config migration");
          saveConfig(validConfig);
        }
      } catch (err: unknown) {
        if (!mounted) return;

        const message = getErrorMessage(err, "Failed to load config");
        console.error("[ConfigProvider] Load failed:", err);
        setLoadError(message);
        setIsLoaded(true);
      }
    }

    void loadConfig();

    return () => {
      mounted = false;
    };
  }, [saveConfig]);

  useEffect(() => {
    const handleBeforeUnload = (): void => {
      saveConfigImmediate();
    };

    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => {
      window.removeEventListener("beforeunload", handleBeforeUnload);
    };
  }, [saveConfigImmediate]);

  useEffect(() => {
    return () => {
      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, []);

  return {
    config,
    setConfig,
    isLoaded,
    loadError,
    saveError,
    saveConfig,
    saveConfigNow,
  };
}
