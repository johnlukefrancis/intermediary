// Path: app/src/hooks/use_config_actions_rail.ts
// Description: Config action for the persisted right-rail deck section (zips | source)

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
