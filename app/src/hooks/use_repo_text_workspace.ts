// Path: app/src/hooks/use_repo_text_workspace.ts
// Description: Repo-tab text workspace state for notes and ephemeral file scratch buffers

import { useCallback, useEffect, useRef, useState } from "react";
import { sendReadTextFile } from "../lib/agent/messages.js";
import { useAgent } from "./use_agent.js";

export type RepoTextWorkspace =
  | { kind: "none" }
  | { kind: "note"; repoId: string }
  | {
      kind: "fileScratch";
      repoId: string;
      path: string;
      content: string;
      bytes: number;
      mtimeMs: number;
    };

interface RepoTextWorkspaceState {
  workspace: RepoTextWorkspace;
  openNote: () => void;
  openFileScratch: (path: string) => void;
  updateFileScratch: (content: string) => void;
  closeWorkspace: () => void;
}

export function getFileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] ?? path;
}

export function useRepoTextWorkspace(repoId: string): RepoTextWorkspaceState {
  const { client, helloState } = useAgent();
  const [workspace, setWorkspace] = useState<RepoTextWorkspace>({ kind: "none" });
  const requestTokenRef = useRef(0);

  const closeWorkspace = useCallback(() => {
    requestTokenRef.current += 1;
    setWorkspace({ kind: "none" });
  }, []);

  const openNote = useCallback(() => {
    requestTokenRef.current += 1;
    setWorkspace({ kind: "note", repoId });
  }, [repoId]);

  const openFileScratch = useCallback(
    (path: string) => {
      const requestToken = requestTokenRef.current + 1;
      requestTokenRef.current = requestToken;
      if (!client || helloState.status !== "ok") return;

      void sendReadTextFile(client, repoId, path)
        .then((result) => {
          if (requestTokenRef.current !== requestToken) return;
          setWorkspace({
            kind: "fileScratch",
            repoId: result.repoId,
            path: result.path,
            content: result.content,
            bytes: result.bytes,
            mtimeMs: result.mtimeMs,
          });
        })
        .catch((err: unknown) => {
          if (requestTokenRef.current !== requestToken) return;
          console.info("[useRepoTextWorkspace] readTextFile skipped:", err);
        });
    },
    [client, helloState.status, repoId]
  );

  const updateFileScratch = useCallback((content: string) => {
    setWorkspace((current) => {
      if (current.kind !== "fileScratch" || current.repoId !== repoId) return current;
      return { ...current, content };
    });
  }, [repoId]);

  useEffect(() => {
    requestTokenRef.current += 1;
    setWorkspace({ kind: "none" });
  }, [repoId]);

  const effectiveWorkspace: RepoTextWorkspace =
    workspace.kind !== "none" && workspace.repoId !== repoId
      ? { kind: "none" }
      : workspace;

  return {
    workspace: effectiveWorkspace,
    openNote,
    openFileScratch,
    updateFileScratch,
    closeWorkspace,
  };
}
