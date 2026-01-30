// Path: app/src/hooks/use_repo_state.ts
// Description: Per-repo file state management with event subscription

import { useState, useEffect, useCallback } from "react";
import { useAgent } from "./use_agent.js";
import type {
  FileEntry,
  StagedInfo,
  AgentEvent,
} from "../shared/protocol.js";

const MAX_RECENT_FILES = 200;

export interface RepoState {
  recentDocs: FileEntry[];
  recentCode: FileEntry[];
  stagedByPath: Map<string, StagedInfo>;
  isLoading: boolean;
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
  const { subscribe } = useAgent();

  const [recentDocs, setRecentDocs] = useState<FileEntry[]>([]);
  const [recentCode, setRecentCode] = useState<FileEntry[]>([]);
  const [stagedByPath, setStagedByPath] = useState<Map<string, StagedInfo>>(
    () => new Map()
  );
  const [isLoading, setIsLoading] = useState(true);

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
          mtime: event.mtime,
        };

        if (event.kind === "docs") {
          setRecentDocs((prev) => upsertFile(prev, entry));
        } else if (event.kind === "code") {
          setRecentCode((prev) => upsertFile(prev, entry));
        }

        const stagedInfo = event.staged;
        if (stagedInfo) {
          setStagedByPath((prev) => {
            const next = new Map(prev);
            next.set(event.path, stagedInfo);
            return next;
          });
        }
      }
    },
    [repoId]
  );

  useEffect(() => {
    // Reset state when repoId changes
    setRecentDocs([]);
    setRecentCode([]);
    setStagedByPath(new Map());
    setIsLoading(true);

    const unsubscribe = subscribe(handleEvent);
    return unsubscribe;
  }, [subscribe, handleEvent, repoId]);

  return {
    recentDocs,
    recentCode,
    stagedByPath,
    isLoading,
  };
}
