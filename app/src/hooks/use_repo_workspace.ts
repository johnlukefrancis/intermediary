// Path: app/src/hooks/use_repo_workspace.ts
// Description: Repo-tab workspace state for notes, text scratch buffers, and image previews

import { useCallback, useEffect, useRef, useState } from "react";
import { sendReadImageFile, sendReadTextFile } from "../lib/agent/messages.js";
import { useAgent } from "./use_agent.js";

const IMAGE_PREVIEW_EXTENSIONS = new Set(["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif"]);
const UNSUPPORTED_IMAGE_EXTENSIONS = new Set(["tif", "tiff", "heic", "heif"]);

export type RepoWorkspace =
  | { kind: "none" }
  | { kind: "note"; repoId: string }
  | {
      kind: "textFile";
      repoId: string;
      path: string;
      content: string;
      bytes: number;
      mtimeMs: number;
    }
  | {
      kind: "imageFile";
      repoId: string;
      path: string;
      status: "loading";
    }
  | {
      kind: "imageFile";
      repoId: string;
      path: string;
      status: "ready";
      dataBase64: string;
      mimeType: string;
      bytes: number;
      mtimeMs: number;
    }
  | {
      kind: "imageFile";
      repoId: string;
      path: string;
      status: "error";
      error: string;
    };

interface RepoWorkspaceState {
  workspace: RepoWorkspace;
  openNote: () => void;
  openFile: (path: string) => void;
  updateTextScratch: (content: string) => void;
  closeWorkspace: () => void;
}

export function getFileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] ?? path;
}

function getExtension(path: string): string | null {
  const fileName = getFileName(path);
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex === -1 || dotIndex === fileName.length - 1) return null;
  return fileName.slice(dotIndex + 1).toLowerCase();
}

function isPreviewImagePath(path: string): boolean {
  const extension = getExtension(path);
  return extension !== null && IMAGE_PREVIEW_EXTENSIONS.has(extension);
}

function isUnsupportedImagePath(path: string): boolean {
  const extension = getExtension(path);
  return extension !== null && UNSUPPORTED_IMAGE_EXTENSIONS.has(extension);
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

export function useRepoWorkspace(repoId: string): RepoWorkspaceState {
  const { client, helloState } = useAgent();
  const [workspace, setWorkspace] = useState<RepoWorkspace>({ kind: "none" });
  const requestTokenRef = useRef(0);

  const closeWorkspace = useCallback(() => {
    requestTokenRef.current += 1;
    setWorkspace({ kind: "none" });
  }, []);

  const openNote = useCallback(() => {
    requestTokenRef.current += 1;
    setWorkspace({ kind: "note", repoId });
  }, [repoId]);

  const openTextFile = useCallback(
    (path: string) => {
      const requestToken = requestTokenRef.current + 1;
      requestTokenRef.current = requestToken;
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
    [client, helloState.status, repoId]
  );

  const openImageFile = useCallback(
    (path: string) => {
      const requestToken = requestTokenRef.current + 1;
      requestTokenRef.current = requestToken;

      if (!client || helloState.status !== "ok") {
        setWorkspace({
          kind: "imageFile",
          repoId,
          path,
          status: "error",
          error: "Agent session initializing; retry in a moment.",
        });
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
    [client, helloState.status, repoId]
  );

  const openFile = useCallback(
    (path: string) => {
      if (isPreviewImagePath(path)) {
        openImageFile(path);
        return;
      }

      if (isUnsupportedImagePath(path)) {
        requestTokenRef.current += 1;
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
    [openImageFile, openTextFile, repoId]
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
    updateTextScratch,
    closeWorkspace,
  };
}
