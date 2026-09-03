// Path: app/src/hooks/repo_workspace_types.ts
// Description: RepoWorkspace union (note, text, image, diff) and path helpers shared by the workspace hook

import type { SourceControlArea } from "../shared/protocol.js";

const IMAGE_PREVIEW_EXTENSIONS = new Set(["png", "jpg", "jpeg", "webp", "gif", "bmp", "avif"]);
const UNSUPPORTED_IMAGE_EXTENSIONS = new Set(["tif", "tiff", "heic", "heif"]);

interface DiffWorkspaceBase {
  kind: "diff";
  repoId: string;
  path: string;
  /** index = HEAD->index (STAGED DIFF); worktree = index->worktree (WORKTREE DIFF) */
  area: SourceControlArea;
  originalPath: string | null;
  /** False for deleted entries: the title then offers no drag-out or file actions */
  fileExists: boolean;
}

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
  | { kind: "imageFile"; repoId: string; path: string; status: "loading" }
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
  | { kind: "imageFile"; repoId: string; path: string; status: "error"; error: string }
  | (DiffWorkspaceBase & { status: "loading" })
  | (DiffWorkspaceBase & { status: "ready"; patch: string; truncated: boolean; binary: boolean })
  | (DiffWorkspaceBase & { status: "error"; error: string });

export function getFileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1] ?? path;
}

export function getExtension(path: string): string | null {
  const fileName = getFileName(path);
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex === -1 || dotIndex === fileName.length - 1) return null;
  return fileName.slice(dotIndex + 1).toLowerCase();
}

export function isPreviewImagePath(path: string): boolean {
  const extension = getExtension(path);
  return extension !== null && IMAGE_PREVIEW_EXTENSIONS.has(extension);
}

export function isUnsupportedImagePath(path: string): boolean {
  const extension = getExtension(path);
  return extension !== null && UNSUPPORTED_IMAGE_EXTENSIONS.has(extension);
}

export function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}
