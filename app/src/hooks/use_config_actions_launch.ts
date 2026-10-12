// Path: app/src/hooks/use_config_actions_launch.ts
// Description: Config action persisting a repo's chosen project-launch target

import { useCallback, type Dispatch, type SetStateAction } from "react";
import type { PersistedConfig } from "../shared/config.js";

type SetConfig = Dispatch<SetStateAction<PersistedConfig>>;
type SaveConfig = (config: PersistedConfig) => void;

export function useSetLaunchTarget(
  setConfig: SetConfig,
  saveConfig: SaveConfig
): (repoId: string, relativePath: string) => void {
  return useCallback(
    (repoId: string, relativePath: string) => {
      setConfig((prev) => {
        if (prev.launchTargets[repoId] === relativePath) return prev;
        const next: PersistedConfig = {
          ...prev,
          launchTargets: { ...prev.launchTargets, [repoId]: relativePath },
        };
        saveConfig(next);
        return next;
      });
    },
    [setConfig, saveConfig]
  );
}
