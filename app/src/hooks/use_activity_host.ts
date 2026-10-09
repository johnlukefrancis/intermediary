// Path: app/src/hooks/use_activity_host.ts
// Description: One app-lifetime dispatcher for Stream and all-repository Overview activity

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { useAgent } from "./use_agent.js";
import { useConfig } from "./use_config.js";
import { useOverviewHistory } from "./overview/use_overview_history.js";
import { isWslTransportError } from "./agent/wsl_transport_errors.js";
import { streamStoreRegistry } from "../lib/stream/stream_store_registry.js";
import { overviewStore } from "../lib/overview/overview_store.js";
import type { RepoRoot } from "../shared/config.js";
import type { StreamRepoRootKind } from "../lib/stream/stream_types.js";

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

function usePrefersReducedMotion(): boolean {
  const [reduced, setReduced] = useState<boolean>(() => window.matchMedia(REDUCED_MOTION_QUERY).matches);
  useEffect(() => {
    const query = window.matchMedia(REDUCED_MOTION_QUERY);
    const update = (): void => { setReduced(query.matches); };
    query.addEventListener("change", update);
    return () => { query.removeEventListener("change", update); };
  }, []);
  return reduced;
}

export function useActivityHost(documentHidden: boolean): void {
  const { subscribe, connectionState, helloState, rehydrateEpoch, agentError } = useAgent();
  const { config } = useConfig();
  const reducedMotion = usePrefersReducedMotion();
  const [wslConnected, setWslConnected] = useState(true);
  const [wslOnline, setWslOnline] = useState(true);
  const previousEpochRef = useRef(rehydrateEpoch);
  const previousHelloRef = useRef(helloState.lastHelloAt);
  const previousWslOnlineRef = useRef(wslOnline);
  const knownRootsRef = useRef<ReadonlyMap<string, RepoRoot>>(new Map());
  useOverviewHistory(wslConnected, wslOnline);

  useLayoutEffect(() => {
    const current = new Map(config.repos.map((repo) => [repo.repoId, repo.root] as const));
    streamStoreRegistry.setRepositories(config.repos);
    knownRootsRef.current = current;
    overviewStore.setRepositories(config.repos);
  }, [config.repos]);

  // Root identity excludes delayed activity from a removed or replaced watch.
  useEffect(
    () =>
      subscribe((event) => {
        if (event.type === "wslBackendStatus") {
          setWslConnected(event.status === "online");
          setWslOnline(event.status === "online");
        }
        if ("rootPath" in event && knownRootsRef.current.get(event.repoId)?.path !== event.rootPath) return;
        streamStoreRegistry.routeAgentEvent(event);
        overviewStore.intake(event);
      }),
    [subscribe]
  );

  useEffect(() => {
    if (isWslTransportError(agentError)) setWslOnline(false);
  }, [agentError]);

  useEffect(() => {
    const rootKinds = new Map<string, StreamRepoRootKind>(
      config.repos.map((repo) => [repo.repoId, repo.root.kind] as const)
    );
    streamStoreRegistry.setHostState({
      transport: {
        connected: connectionState.status === "connected",
        helloOk: helloState.status === "ok",
        agentVersion: helloState.agentVersion,
        wslOnline,
      },
      documentHidden,
      reducedMotion,
      rootKinds,
    });
    overviewStore.setTransport({
      connected: connectionState.status === "connected",
      helloOk: helloState.status === "ok",
      agentVersion: helloState.agentVersion,
      watchedRepoIds: helloState.watchedRepoIds,
      wslOnline,
      error: helloState.status === "error" ? helloState.lastError : null,
    });
  }, [config.repos, connectionState.status, documentHidden, helloState.agentVersion, helloState.lastError, helloState.status, helloState.watchedRepoIds, reducedMotion, wslOnline]);

  useEffect(() => {
    const resumed = previousEpochRef.current !== rehydrateEpoch;
    const backendRecovered = !previousWslOnlineRef.current && wslOnline;
    previousWslOnlineRef.current = wslOnline;
    const reconciled = previousHelloRef.current !== null && helloState.lastHelloAt !== null
      && previousHelloRef.current !== helloState.lastHelloAt;
    previousEpochRef.current = rehydrateEpoch;
    if (helloState.lastHelloAt !== null) previousHelloRef.current = helloState.lastHelloAt;
    if (!resumed && !reconciled && !backendRecovered) return;
    streamStoreRegistry.markRehydrated();
    overviewStore.markRehydrated();
  }, [helloState.lastHelloAt, rehydrateEpoch, wslOnline]);
}
