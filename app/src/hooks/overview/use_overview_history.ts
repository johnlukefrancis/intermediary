// Path: app/src/hooks/overview/use_overview_history.ts
// Description: Serial initial and reconnect history refresh across connected roots, with host roots first

import { useEffect, useRef } from "react";
import { useAgent } from "../use_agent.js";
import { useConfig } from "../use_config.js";
import { sendRefresh } from "../../lib/agent/messages.js";
import { parseAgentErrorCode } from "../../lib/agent/error_codes.js";
import { overviewStore } from "../../lib/overview/overview_store.js";
import { activitySupport } from "../../lib/agent/activity_support.js";

export function useOverviewHistory(wslConnected: boolean, wslOnline: boolean): void {
  const { client, connectionState, helloState, rehydrateEpoch } = useAgent();
  const { config } = useConfig();
  const workRef = useRef<{ pending: (() => Promise<void>)[]; running: number }>({ pending: [], running: 0 });

  useEffect(() => {
    const work = workRef.current;
    work.pending = [];
    if (client === null || connectionState.status !== "connected" || helloState.status !== "ok"
      || activitySupport(helloState.agentVersion) !== "supported") return;
    let cancelled = false;
    const isCancelled = (): boolean => cancelled;
    const roots = config.repos.filter((repo) => repo.root.kind !== "wsl" || wslConnected);
    roots.sort((a, b) => Number(a.root.kind === "wsl") - Number(b.root.kind === "wsl"));
    work.pending = roots.map((repo) => async () => {
      if (isCancelled()) return;
      try {
        await sendRefresh(client, repo.repoId);
      } catch (error) {
        if (!isCancelled()) {
          const code = parseAgentErrorCode(error);
          overviewStore.setRepoIssue(repo.repoId, repo.root,
            `History unavailable: ${error instanceof Error ? error.message : "refresh failed"}`,
            code === "WATCHER_STOPPED" || code === "UNKNOWN_REPO");
        }
      }
    });

    const pump = (): void => {
      while (work.running < 1 && work.pending.length > 0) {
        const request = work.pending.shift();
        if (!request) break;
        work.running += 1;
        void request().finally(() => { work.running -= 1; pump(); });
      }
    };
    pump();
    return () => { cancelled = true; work.pending = []; };
  }, [client, config.repos, connectionState.status, helloState.agentVersion, helloState.lastHelloAt,
    helloState.status, rehydrateEpoch, wslConnected, wslOnline]);
}
