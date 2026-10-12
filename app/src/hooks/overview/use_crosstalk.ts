// Path: app/src/hooks/overview/use_crosstalk.ts
// Description: Main-window Overview visibility and theme projected to the native companion

import { useEffect, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { RepoConfig, TabTheme } from "../../shared/config.js";
import { DEFAULT_ACCENT_HEX } from "../../lib/theme/accent_utils.js";

let pending: Promise<void> = Promise.resolve();

export function useCrosstalk(active: boolean, theme: string, repos: readonly RepoConfig[], tabThemes: Readonly<Record<string, TabTheme>>): void {
  const styles = useMemo(() => repos.map(repo => ({
    path: repo.root.path, label: repo.label,
    accent: tabThemes[repo.groupId ?? repo.repoId]?.accentHex ?? DEFAULT_ACCENT_HEX,
  })), [repos, tabThemes]);
  useEffect(() => {
    pending = pending.then(async () => { await invoke("crosstalk_configure", { active, theme, repos: styles }); })
      // TODO(ts-precision): Tauri command rejections are an external error seam.
      .catch((error: unknown) => { console.error("[crosstalk]", error); });
  }, [active, theme, styles]);
}
