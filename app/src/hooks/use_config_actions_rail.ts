// Path: app/src/hooks/use_config_actions_rail.ts
// Description: Config actions for the persisted right rail: the deck section (zips | source | terminal) and the rail width

import { useCallback, type Dispatch, type SetStateAction } from "react";
import type { ActiveRail, PersistedConfig } from "../shared/config.js";

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
