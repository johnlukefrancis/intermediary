// Path: app/src/hooks/use_repo_workspace.ts
// Description: Repo-tab workspace state for notes, text buffers, image previews, and text/image diffs

import { useCallback, useEffect, useRef, useState } from "react";
import type { SourceControlArea, SourceControlEntry } from "../shared/protocol.js";
import { sendReadImageFile, sendReadTextFile } from "../lib/agent/messages.js";
import { loadImageDiff, loadTextDiff } from "./repo_workspace_diff_loaders.js";
import { useAgent } from "./use_agent.js";
import {
  errorMessage,
  isPreviewImagePath,
  isUnsupportedImagePath,
  type RepoWorkspace,
} from "./repo_workspace_types.js";

export { getFileName, type RepoWorkspace } from "./repo_workspace_types.js";

const AGENT_INITIALIZING = "Agent session initializing; retry in a moment.";

interface RepoWorkspaceState {
  workspace: RepoWorkspace;
  openNote: () => void;
  openFile: (path: string) => void;
  openDiff: (entry: SourceControlEntry) => void;
  updateTextScratch: (content: string) => void;
  closeWorkspace: () => void;
}

export function useRepoWorkspace(repoId: string): RepoWorkspaceState {
  const { client, helloState } = useAgent();
  const [workspace, setWorkspace] = useState<RepoWorkspace>({ kind: "none" });
  const requestTokenRef = useRef(0);

  const nextRequestToken = useCallback((): number => {
    requestTokenRef.current += 1;
    return requestTokenRef.current;
  }, []);

  const closeWorkspace = useCallback(() => {
    nextRequestToken();
    setWorkspace({ kind: "none" });
  }, [nextRequestToken]);

  const openNote = useCallback(() => {
    nextRequestToken();
    setWorkspace({ kind: "note", repoId });
  }, [nextRequestToken, repoId]);

  const openTextFile = useCallback(
    (path: string) => {
      const requestToken = nextRequestToken();
      if (!client || helloState.status !== "ok") return;

      void sendReadTextFile(client, repoId, path)
        .then((result) => {
          if (requestTokenRef.current !== requestToken) return;
          setWorkspace({
            kind: "textFile",
            repoId: result.repoId,
            path: result.path,
            content: result.content,
            bytes: result.bytes,
            mtimeMs: result.mtimeMs,
          });
        })
        .catch((err: unknown) => {
          if (requestTokenRef.current !== requestToken) return;
          console.info("[useRepoWorkspace] readTextFile skipped:", err);
        });
    },
    [client, helloState.status, nextRequestToken, repoId]
  );

  const openImageFile = useCallback(
    (path: string) => {
      const requestToken = nextRequestToken();

      if (!client || helloState.status !== "ok") {
        setWorkspace({ kind: "imageFile", repoId, path, status: "error", error: AGENT_INITIALIZING });
        return;
      }

      setWorkspace({ kind: "imageFile", repoId, path, status: "loading" });

      void sendReadImageFile(client, repoId, path)
        .then((result) => {
          if (requestTokenRef.current !== requestToken) return;
          setWorkspace({
            kind: "imageFile",
            repoId: result.repoId,
            path: result.path,
            status: "ready",
            dataBase64: result.dataBase64,
            mimeType: result.mimeType,
            bytes: result.bytes,
            mtimeMs: result.mtimeMs,
          });
        })
        .catch((err: unknown) => {
          if (requestTokenRef.current !== requestToken) return;
          setWorkspace({
            kind: "imageFile",
            repoId,
            path,
            status: "error",
            error: errorMessage(err, "Unable to load image preview"),
          });
        });
    },
    [client, helloState.status, nextRequestToken, repoId]
  );

  const openFile = useCallback(
    (path: string) => {
      if (isPreviewImagePath(path)) {
        openImageFile(path);
        return;
      }

      if (isUnsupportedImagePath(path)) {
        nextRequestToken();
        setWorkspace({
          kind: "imageFile",
          repoId,
          path,
          status: "error",
          error: "Image preview supports PNG, JPEG, WebP, GIF, BMP, and AVIF.",
        });
        return;
      }

      openTextFile(path);
    },
    [nextRequestToken, openImageFile, openTextFile, repoId]
  );

  /** Index diff for STAGED rows, worktree diff for CHANGES/MERGE rows (untracked shows as all-added) */
  const openDiff = useCallback(
    (entry: SourceControlEntry) => {
      const requestToken = nextRequestToken();
      const area: SourceControlArea = entry.area === "index" ? "index" : "worktree";
      const entryFacts = {
        repoId,
        path: entry.path,
        area,
        originalPath: entry.originalPath ?? null,
        fileExists: entry.change !== "deleted",
        conflict: entry.area === "conflict",
      };
      const isStale = (): boolean => requestTokenRef.current !== requestToken;
      const isImage = isPreviewImagePath(entry.path);
      const base = isImage
        ? { kind: "imageDiff" as const, ...entryFacts }
        : { kind: "diff" as const, ...entryFacts };

      if (!client || helloState.status !== "ok") {
        setWorkspace({ ...base, status: "error", error: AGENT_INITIALIZING });
        return;
      }

      const context = { client, repoId, isStale, setWorkspace };
      if (base.kind === "imageDiff") {
        loadImageDiff(context, base);
        return;
      }
      loadTextDiff(context, base);
    },
    [client, helloState.status, nextRequestToken, repoId]
  );

  const updateTextScratch = useCallback((content: string) => {
    setWorkspace((current) => {
      if (current.kind !== "textFile" || current.repoId !== repoId) return current;
      return { ...current, content };
    });
  }, [repoId]);

  useEffect(() => {
    requestTokenRef.current += 1;
    setWorkspace({ kind: "none" });
  }, [repoId]);

  const effectiveWorkspace: RepoWorkspace =
    workspace.kind !== "none" && workspace.repoId !== repoId
      ? { kind: "none" }
      : workspace;

  return {
    workspace: effectiveWorkspace,
    openNote,
    openFile,
    openDiff,
    updateTextScratch,
    closeWorkspace,
  };
}
