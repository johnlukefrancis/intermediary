// Path: app/src/hooks/use_repo_state.ts
// Description: Per-repo file state management with event subscription

import { useState, useEffect, useCallback, useRef } from "react";
import { useAgent } from "./use_agent.js";
import type {
  FileEntry,
  StagedInfo,
  AgentEvent,
} from "../shared/protocol.js";
import { sendGetRepoTopLevel, sendRefresh } from "../lib/agent/messages.js";

const MAX_RECENT_FILES = 200;

export interface RepoState {
  recentDocs: FileEntry[];
  recentCode: FileEntry[];
  stagedByPath: Map<string, StagedInfo>;
  isLoading: boolean;
  topLevelDirs: string[];
  topLevelFiles: string[];
  registerStaged: (relativePath: string, stagedInfo: StagedInfo) => void;
}

function sortByMtimeDesc(a: FileEntry, b: FileEntry): number {
  return new Date(b.mtime).getTime() - new Date(a.mtime).getTime();
}

function upsertFile(files: FileEntry[], entry: FileEntry): FileEntry[] {
  const idx = files.findIndex((f) => f.path === entry.path);
  let updated: FileEntry[];
  if (idx >= 0) {
    updated = [...files];
    updated[idx] = entry;
  } else {
    updated = [entry, ...files];
  }
  return updated.sort(sortByMtimeDesc).slice(0, MAX_RECENT_FILES);
}

export function useRepoState(repoId: string): RepoState {
  const { subscribe, client, connectionState, helloState } = useAgent();

  const [recentDocs, setRecentDocs] = useState<FileEntry[]>([]);
  const [recentCode, setRecentCode] = useState<FileEntry[]>([]);
  const [stagedByPath, setStagedByPath] = useState<Map<string, StagedInfo>>(
    () => new Map()
  );
  const [isLoading, setIsLoading] = useState(true);
  const [topLevelDirs, setTopLevelDirs] = useState<string[]>([]);
  const [topLevelFiles, setTopLevelFiles] = useState<string[]>([]);
  const lastHelloRefreshKeyRef = useRef<string | null>(null);

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
        const docs = event.recent
          .filter((f) => f.kind === "docs")
          .sort(sortByMtimeDesc)
          .slice(0, MAX_RECENT_FILES);
        const code = event.recent
          .filter((f) => f.kind === "code")
          .sort(sortByMtimeDesc)
          .slice(0, MAX_RECENT_FILES);

        setRecentDocs(docs);
        setRecentCode(code);
        setStagedByPath(new Map());
        setIsLoading(false);
      } else if (event.type === "fileChanged" && event.repoId === repoId) {
        const entry: FileEntry = {
          path: event.path,
          kind: event.kind,
          changeType: event.changeType,
          mtime: event.mtime,
        };

        if (event.kind === "docs") {
          setRecentDocs((prev) => upsertFile(prev, entry));
        } else if (event.kind === "code") {
          setRecentCode((prev) => upsertFile(prev, entry));
        }

        const stagedInfo = event.staged;
        if (stagedInfo) {
          registerStaged(event.path, stagedInfo);
        }
      }
    },
    [repoId, registerStaged]
  );

  useEffect(() => {
    // Reset state when repoId changes
    setRecentDocs([]);
    setRecentCode([]);
    setStagedByPath(new Map());
    setIsLoading(true);
    setTopLevelDirs([]);
    setTopLevelFiles([]);

    const unsubscribe = subscribe(handleEvent);
    return unsubscribe;
  }, [subscribe, handleEvent, repoId]);

  useEffect(() => {
    if (
      connectionState.status !== "connected" ||
      !client ||
      helloState.status !== "ok" ||
      helloState.lastHelloAt === null
    ) {
      return;
    }
    if (!helloState.watchedRepoIds.includes(repoId)) {
      return;
    }

    const refreshKey = `${repoId}:${helloState.lastHelloAt}`;
    if (lastHelloRefreshKeyRef.current === refreshKey) {
      return;
    }
    lastHelloRefreshKeyRef.current = refreshKey;

    setIsLoading(true);
    void sendRefresh(client, repoId).catch((err: unknown) => {
      console.error("[useRepoState] refresh failed:", err);
      setIsLoading(false);
    });

    void sendGetRepoTopLevel(client, repoId)
      .then((result) => {
        setTopLevelDirs(result.dirs);
        setTopLevelFiles(result.files);
      })
      .catch((err: unknown) => {
        console.error("[useRepoState] getRepoTopLevel failed:", err);
      });
  }, [
    repoId,
    client,
    connectionState.status,
    helloState.status,
    helloState.lastHelloAt,
    helloState.watchedRepoIds,
  ]);

  return {
    recentDocs,
    recentCode,
    stagedByPath,
    isLoading,
    topLevelDirs,
    topLevelFiles,
    registerStaged,
  };
}
