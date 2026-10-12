// Path: app/src/hooks/overview/use_crosstalk.ts
// Description: Main-window Overview visibility and theme projected to the native companion

import { useEffect, useMemo, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { RepoConfig, TabTheme } from "../../shared/config.js";
import { DEFAULT_ACCENT_HEX } from "../../lib/theme/accent_utils.js";
import { overviewLanes, type Turn } from "../../lib/overview/overview_lanes.js";
import { statusRegistry } from "../../lib/source_control/status_registry.js";

let pending: Promise<void> = Promise.resolve();

export function useCrosstalk(active: boolean, theme: string, repos: readonly RepoConfig[], tabThemes: Readonly<Record<string, TabTheme>>): void {
  const subscription = useRef<Promise<UnlistenFn> | null>(null);
  useEffect(() => {
    const ready = listen<Turn>("crosstalk-turn", ({ payload }) => {
      const repoId = overviewLanes.receive(payload, Date.now());
      if (repoId !== undefined && payload.text === "idle") statusRegistry.refresh(repoId);
    });
    subscription.current = ready;
    return () => {
      // TODO(ts-precision): Native event subscription rejection is an external error seam.
      void ready.then(unlisten => { unlisten(); }).catch((error: unknown) => { console.error("[crosstalk]", error); });
    };
  }, []);
  useEffect(() => { overviewLanes.setRepositories(repos); }, [repos]);
  const styles = useMemo(() => repos.map(repo => ({
    path: repo.root.path, label: repo.label,
    accent: tabThemes[repo.groupId ?? repo.repoId]?.accentHex ?? DEFAULT_ACCENT_HEX,
  })), [repos, tabThemes]);
  useEffect(() => {
    pending = pending.then(async () => {
      await subscription.current;
      await invoke("crosstalk_configure", { active, theme, repos: styles });
    })
      // TODO(ts-precision): Tauri command rejections are an external error seam.
      .catch((error: unknown) => { console.error("[crosstalk]", error); });
  }, [active, theme, styles]);
}
