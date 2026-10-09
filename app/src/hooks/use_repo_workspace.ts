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
  const { client, helloState, config } = useAgent();
  const root = config.repos.find((repo) => repo.repoId === repoId)?.root;
  const rootIdentity = JSON.stringify([repoId, root?.kind, root?.path]);
  const [state, setState] = useState<{ rootIdentity: string; workspace: RepoWorkspace }>({
    rootIdentity,
    workspace: { kind: "none" },
  });
  const requestTokenRef = useRef(0);
  const setWorkspace = useCallback((workspace: RepoWorkspace) => {
    setState({ rootIdentity, workspace });
  }, [rootIdentity]);

  const nextRequestToken = useCallback((): number => {
    requestTokenRef.current += 1;
    return requestTokenRef.current;
  }, []);

  const closeWorkspace = useCallback(() => {
    nextRequestToken();
    setWorkspace({ kind: "none" });
  }, [nextRequestToken, setWorkspace]);

  const openNote = useCallback(() => {
    nextRequestToken();
    setWorkspace({ kind: "note", repoId });
  }, [nextRequestToken, repoId, setWorkspace]);

  const openTextFile = useCallback(
    (path: string) => {
      const requestToken = nextRequestToken();
      if (!client || helloState.status !== "ok") {
        setWorkspace({ kind: "textFile", repoId, path, status: "error", error: AGENT_INITIALIZING });
        return;
      }

      setWorkspace({ kind: "textFile", repoId, path, status: "loading" });

      void sendReadTextFile(client, repoId, path)
        .then((result) => {
          if (requestTokenRef.current !== requestToken) return;
          setWorkspace({
            kind: "textFile",
            repoId: result.repoId,
            path: result.path,
            status: "ready",
            content: result.content,
            bytes: result.bytes,
            mtimeMs: result.mtimeMs,
          });
        })
        .catch((err: unknown) => {
          if (requestTokenRef.current !== requestToken) return;
          setWorkspace({
            kind: "textFile",
            repoId,
            path,
            status: "error",
            error: errorMessage(err, "Unable to load text file"),
          });
        });
    },
    [client, helloState.status, nextRequestToken, repoId, setWorkspace]
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
    [client, helloState.status, nextRequestToken, repoId, setWorkspace]
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
    [nextRequestToken, openImageFile, openTextFile, repoId, setWorkspace]
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
    [client, helloState.status, nextRequestToken, repoId, setWorkspace]
  );

  const updateTextScratch = useCallback((content: string) => {
    setState((current) => {
      const workspace = current.workspace;
      if (current.rootIdentity !== rootIdentity || workspace.kind !== "textFile"
        || workspace.status !== "ready" || workspace.repoId !== repoId) return current;
      return { ...current, workspace: { ...workspace, content } };
    });
  }, [repoId, rootIdentity]);

  useEffect(() => {
    requestTokenRef.current += 1;
    setWorkspace({ kind: "none" });
    return () => { requestTokenRef.current += 1; };
  }, [setWorkspace]);

  const effectiveWorkspace: RepoWorkspace =
    state.rootIdentity !== rootIdentity
      || (state.workspace.kind !== "none" && state.workspace.repoId !== repoId)
      ? { kind: "none" }
      : state.workspace;

  return {
    workspace: effectiveWorkspace,
    openNote,
    openFile,
    openDiff,
    updateTextScratch,
    closeWorkspace,
  };
}
