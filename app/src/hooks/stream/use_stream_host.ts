// Path: app/src/hooks/stream/use_stream_host.ts
// Description: Mounted once: the one agent subscription and host facts pushed into every Stream store

import { useEffect, useRef, useState } from "react";
import { useAgent } from "../use_agent.js";
import { useConfig } from "../use_config.js";
import { isWslTransportError } from "../agent/wsl_transport_errors.js";
import { streamStoreRegistry } from "../../lib/stream/stream_store_registry.js";
import type { StreamRepoRootKind } from "../../lib/stream/stream_types.js";

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

export function useStreamHost(documentHidden: boolean): void {
  const { subscribe, connectionState, helloState, rehydrateEpoch, agentError } = useAgent();
  const { config } = useConfig();
  const reducedMotion = usePrefersReducedMotion();
  const [wslOnline, setWslOnline] = useState(true);
  const previousEpochRef = useRef(rehydrateEpoch);
  const knownRepoIdsRef = useRef<ReadonlySet<string>>(new Set());

  // ONE ordered dispatcher: every stream-relevant event enters the registry from here
  useEffect(
    () =>
      subscribe((event) => {
        if (event.type === "wslBackendStatus") setWslOnline(event.status === "online");
        streamStoreRegistry.routeAgentEvent(event);
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
  }, [config.repos, connectionState.status, documentHidden, helloState.agentVersion, helloState.status, reducedMotion, wslOnline]);

  // A repo removed from the config takes its store with it
  useEffect(() => {
    const current = new Set(config.repos.map((repo) => repo.repoId));
    for (const repoId of knownRepoIdsRef.current) {
      if (!current.has(repoId)) streamStoreRegistry.disposeRepo(repoId);
    }
    knownRepoIdsRef.current = current;
  }, [config.repos]);

  useEffect(() => {
    if (previousEpochRef.current === rehydrateEpoch) return;
    previousEpochRef.current = rehydrateEpoch;
    streamStoreRegistry.markRehydrated();
  }, [rehydrateEpoch]);
}
