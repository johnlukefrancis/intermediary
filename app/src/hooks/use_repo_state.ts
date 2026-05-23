// Path: app/src/hooks/use_repo_state.ts
// Description: Per-repo file state management with event subscription

import { useState, useEffect, useCallback, useRef } from "react";
import { useAgent } from "./use_agent.js";
import { useConfig } from "./use_config.js";
import type {
  FileEntry,
  StagedInfo,
  AgentEvent,
} from "../shared/protocol.js";
import {
  sendGetRepoTopLevel,
  sendRefresh,
  sendWatchRepo,
} from "../lib/agent/messages.js";
import {
  computeTransientRetryDelayMs,
  isTransientWslTransportError,
} from "../lib/agent/transient_wsl_error.js";
import { isVisibleFileKind } from "../lib/files/file_feed.js";

export type RepoHydrationStatus =
  | "waiting_for_agent"
  | "hydrating"
  | "retrying"
  | "ready"
  | "error";

export interface RepoState {
  recentFiles: FileEntry[];
  stagedByPath: Map<string, StagedInfo>;
  isLoading: boolean;
  hydrationStatus: RepoHydrationStatus;
  topLevelDirs: string[];
  topLevelFiles: string[];
  /** Nested subdirectory paths within each top-level dir, up to repo depth 4 */
  topLevelSubdirs: Record<string, string[]>;
  isTopologyReady: boolean;
  /** Dir names excluded by default (e.g. node_modules, .git, target) */
  defaultExcluded: string[];
  registerStaged: (relativePath: string, stagedInfo: StagedInfo) => void;
}

function sortByMtimeDesc(a: FileEntry, b: FileEntry): number {
  return new Date(b.mtime).getTime() - new Date(a.mtime).getTime();
}

function upsertFile(files: FileEntry[], entry: FileEntry, limit: number): FileEntry[] {
  const idx = files.findIndex((f) => f.path === entry.path);
  let updated: FileEntry[];
  if (idx >= 0) {
    updated = [...files];
    updated[idx] = entry;
  } else {
    updated = [entry, ...files];
  }
  return updated.sort(sortByMtimeDesc).slice(0, limit);
}

function removeFile(files: FileEntry[], path: string): FileEntry[] {
  return files.filter((file) => file.path !== path);
}

export function useRepoState(repoId: string): RepoState {
  const { subscribe, client, connectionState, helloState, rehydrateEpoch } = useAgent();
  const { config } = useConfig();
  const recentFilesLimit = config.recentFilesLimit;

  const [recentFiles, setRecentFiles] = useState<FileEntry[]>([]);
  const [stagedByPath, setStagedByPath] = useState<Map<string, StagedInfo>>(
    () => new Map()
  );
  const [isLoading, setIsLoading] = useState(true);
  const [hydrationStatus, setHydrationStatus] = useState<RepoHydrationStatus>(
    "waiting_for_agent"
  );
  const [topLevelDirs, setTopLevelDirs] = useState<string[]>([]);
  const [topLevelFiles, setTopLevelFiles] = useState<string[]>([]);
  const [topLevelSubdirs, setTopLevelSubdirs] = useState<Record<string, string[]>>({});
  const [topologyReadyRepoId, setTopologyReadyRepoId] = useState<string | null>(null);
  const [defaultExcluded, setDefaultExcluded] = useState<string[]>([]);
  const lastHelloRefreshKeyRef = useRef<string | null>(null);
  const refreshInFlightKeyRef = useRef<string | null>(null);
  const currentRepoIdRef = useRef(repoId);
  const topologyRefreshSeqRef = useRef(0);
  const retryTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  if (currentRepoIdRef.current !== repoId) {
    currentRepoIdRef.current = repoId;
    topologyRefreshSeqRef.current += 1;
  }

  const registerStaged = useCallback((relativePath: string, stagedInfo: StagedInfo) => {
    setStagedByPath((prev) => {
      const next = new Map(prev);
      next.set(relativePath, stagedInfo);
      return next;
    });
  }, []);

  const handleEvent = useCallback(
    (event: AgentEvent) => {
      if (event.type === "snapshot" && event.repoId === repoId) {
        const files = event.recent
          .filter((file) => isVisibleFileKind(file.kind))
          .sort(sortByMtimeDesc)
          .slice(0, recentFilesLimit);

        setRecentFiles(files);
        setStagedByPath(new Map());
        setIsLoading(false);
        setHydrationStatus("ready");
      } else if (event.type === "fileChanged" && event.repoId === repoId) {
        const entry: FileEntry = {
          path: event.path,
          kind: event.kind,
          changeType: event.changeType,
          mtime: event.mtime,
          activity: event.activity,
        };

        if (isVisibleFileKind(event.kind)) {
          setRecentFiles((prev) => (
            event.changeType === "unlink"
              ? removeFile(prev, event.path)
              : upsertFile(prev, entry, recentFilesLimit)
          ));
        }

        setStagedByPath((prev) => {
          const next = new Map(prev);
          if (event.changeType === "unlink" || !event.staged) {
            next.delete(event.path);
          } else {
            next.set(event.path, event.staged);
          }
          return next;
        });
      } else if (event.type === "repoTopologyChanged" && event.repoId === repoId && client) {
        const requestRepoId = event.repoId;
        const requestSeq = topologyRefreshSeqRef.current + 1;
        topologyRefreshSeqRef.current = requestSeq;
        const isStaleTopologyRefresh = (): boolean =>
          currentRepoIdRef.current !== requestRepoId ||
          topologyRefreshSeqRef.current !== requestSeq;
        void sendGetRepoTopLevel(client, requestRepoId)
          .then((result) => {
            if (isStaleTopologyRefresh()) return;
            setTopLevelDirs(result.dirs);
            setTopLevelFiles(result.files);
            setTopLevelSubdirs(result.subdirs ?? {});
            setDefaultExcluded(result.defaultExcluded);
            setTopologyReadyRepoId(requestRepoId);
          })
          .catch((err: unknown) => {
            if (isStaleTopologyRefresh()) return;
            console.error("[useRepoState] repo topology refresh failed:", err);
          });
      }
    },
    [client, repoId, recentFilesLimit]
  );

  useEffect(() => {
    setRecentFiles([]);
    setStagedByPath(new Map());
    setIsLoading(true);
    setHydrationStatus("waiting_for_agent");
    setTopLevelDirs([]);
    setTopLevelFiles([]);
    setTopLevelSubdirs({});
    setTopologyReadyRepoId(null);
    setDefaultExcluded([]);
    lastHelloRefreshKeyRef.current = null;
    refreshInFlightKeyRef.current = null;
    if (retryTimeoutRef.current) {
      clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = null;
    }
  }, [repoId]);

  useEffect(() => {
    const unsubscribe = subscribe(handleEvent);
    return unsubscribe;
  }, [subscribe, handleEvent]);

  useEffect(() => {
    if (
      connectionState.status !== "connected" ||
      !client ||
      helloState.status !== "ok" ||
      helloState.lastHelloAt === null
    ) {
      if (retryTimeoutRef.current) {
        clearTimeout(retryTimeoutRef.current);
        retryTimeoutRef.current = null;
      }
      refreshInFlightKeyRef.current = null;
      setHydrationStatus("waiting_for_agent");
      setIsLoading(true);
      return;
    }

    const refreshKey = `${repoId}:${helloState.lastHelloAt}:${rehydrateEpoch}`;
    if (lastHelloRefreshKeyRef.current === refreshKey) {
      return;
    }
    if (refreshInFlightKeyRef.current === refreshKey) {
      return;
    }
    let cancelled = false;
    let retryAttempt = 0;
    const clearRetryTimeout = (): void => {
      if (!retryTimeoutRef.current) return;
      clearTimeout(retryTimeoutRef.current);
      retryTimeoutRef.current = null;
    };
    const isStale = (): boolean =>
      cancelled || refreshInFlightKeyRef.current !== refreshKey;
    const runHydration = async (): Promise<void> => {
      refreshInFlightKeyRef.current = refreshKey;
      setHydrationStatus(retryAttempt === 0 ? "hydrating" : "retrying");
      setIsLoading(true);
      try {
        if (!helloState.watchedRepoIds.includes(repoId)) {
          await sendWatchRepo(client, repoId);
        }
        if (isStale()) return;
        await sendRefresh(client, repoId);
        if (isStale()) return;
        const result = await sendGetRepoTopLevel(client, repoId);
        if (isStale()) return;
        topologyRefreshSeqRef.current += 1;
        setTopLevelDirs(result.dirs);
        setTopLevelFiles(result.files);
        setTopLevelSubdirs(result.subdirs ?? {});
        setDefaultExcluded(result.defaultExcluded);
        setTopologyReadyRepoId(repoId);
        lastHelloRefreshKeyRef.current = refreshKey;
        clearRetryTimeout();
        setHydrationStatus("ready");
        setIsLoading(false);
      } catch (err: unknown) {
        if (isStale()) return;
        if (isTransientWslTransportError(err)) {
          const delay = computeTransientRetryDelayMs(retryAttempt);
          retryAttempt += 1;
          clearRetryTimeout();
          setHydrationStatus("retrying");
          setIsLoading(true);
          retryTimeoutRef.current = setTimeout(() => {
            if (cancelled) return;
            void runHydration();
          }, delay);
          return;
        }
        console.error("[useRepoState] repo hydration failed:", err);
        clearRetryTimeout();
        setHydrationStatus("error");
        setIsLoading(false);
      } finally {
        if (refreshInFlightKeyRef.current === refreshKey) {
          refreshInFlightKeyRef.current = null;
        }
      }
    };
    void runHydration();

    return () => {
      cancelled = true;
      clearRetryTimeout();
      if (refreshInFlightKeyRef.current === refreshKey) {
        refreshInFlightKeyRef.current = null;
      }
    };
  }, [
    repoId,
    client,
    connectionState.status,
    helloState.status,
    helloState.lastHelloAt,
    helloState.watchedRepoIds,
    rehydrateEpoch,
  ]);

  return {
    recentFiles,
    stagedByPath,
    isLoading,
    hydrationStatus,
    topLevelDirs,
    topLevelFiles,
    topLevelSubdirs,
    isTopologyReady: topologyReadyRepoId === repoId,
    defaultExcluded,
    registerStaged,
  };
}
