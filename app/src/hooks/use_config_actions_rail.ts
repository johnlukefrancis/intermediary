// Path: app/src/hooks/use_config_actions_rail.ts
// Description: Config actions for persisted panel state: the rail section, the rail width, and the left files mode

import { useCallback, type Dispatch, type SetStateAction } from "react";
import type { ActiveRail, PersistedConfig } from "../shared/config.js";
import type { FilesMode } from "../shared/config/ui_state_schema.js";

type SetConfig = Dispatch<SetStateAction<PersistedConfig>>;
type SaveConfig = (config: PersistedConfig) => void;

export function useSetActiveRail(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (rail: ActiveRail) => void {
  return useCallback(
    (rail: ActiveRail) => {
      setConfig((prev) => {
        if (prev.uiState.activeRail === rail) return prev;
        const next: PersistedConfig = {
          ...prev,
          uiState: { ...prev.uiState, activeRail: rail },
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetRailWidthPercent(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (percent: number) => void {
  return useCallback(
    (percent: number) => {
      setConfig((prev) => {
        if (prev.uiState.railWidthPercent === percent) return prev;
        const next: PersistedConfig = {
          ...prev,
          uiState: { ...prev.uiState, railWidthPercent: percent },
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}

export function useSetFilesMode(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (mode: FilesMode) => void {
  return useCallback(
    (mode: FilesMode) => {
      setConfig((prev) => {
        if (prev.uiState.filesMode === mode) return prev;
        const next: PersistedConfig = {
          ...prev,
          uiState: { ...prev.uiState, filesMode: mode },
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}
