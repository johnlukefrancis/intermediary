// Path: app/src/components/repo_workspace_panel.tsx
// Description: Repo workspace renderer for notes, text buffers, image previews, and text/image diffs

import type React from "react";
import { useCallback, useState } from "react";
import { ContextMenu, type ContextMenuItem } from "./context_menu.js";
import { buildSingleFileContextMenuItems } from "./file_context_menu_items.js";
import { DiffWorkspaceViewer } from "./diff_workspace.js";
import { MERGE_CONFLICT_SUBTITLE } from "./source_control/source_control_copy.js";
import { ImageDiffWorkspaceViewer } from "./image_diff_workspace.js";
import { ImageWorkspaceViewer } from "./image_workspace.js";
import { TextWorkspaceEditor } from "./text_workspace.js";
import { WorkspaceLayout } from "./layout/workspace_layout.js";
import { useConfig } from "../hooks/use_config.js";
import { useFileActions } from "../hooks/use_file_actions.js";
import { getExtension, getFileName, type RepoWorkspace } from "../hooks/repo_workspace_types.js";
import type { NoteState } from "../hooks/use_notes.js";
import type { TextWorkspaceSemanticMode } from "./text_workspace_semantics.js";

const MAX_NOTE_LENGTH = 100_000;
const MAX_SCRATCH_LENGTH = 1_000_000;

type ActiveRepoWorkspace =
  | (Extract<RepoWorkspace, { kind: "note" }> & { noteState: NoteState })
  | Exclude<RepoWorkspace, { kind: "none" | "note" }>;

interface RepoWorkspacePanelProps {
  workspace: ActiveRepoWorkspace;
  repoLabel?: string;
  isHandset: boolean;
  onClose: () => void;
  onTextChange: (content: string) => void;
  onTextFileDragStart: (path: string) => void | Promise<void>;
  onImageDragStart: (path: string) => void;
}

interface ContextMenuState {
  x: number;
  y: number;
  path: string;
}

const MARKDOWN_LIKE_EXTENSIONS = new Set(["adoc", "asciidoc", "md", "mdx", "rst", "txt", "wiki"]);

function workspaceTitle(workspace: ActiveRepoWorkspace): string {
  if (workspace.kind === "note") return "Note";
  if (workspace.kind === "diff" || workspace.kind === "imageDiff") return workspace.path;
  return getFileName(workspace.path);
}

function workspaceSubtitle(workspace: ActiveRepoWorkspace): string {
  if (workspace.kind === "note") return "Repository notes";
  if (workspace.kind === "diff") {
    if (workspace.conflict) return MERGE_CONFLICT_SUBTITLE;
    return workspace.area === "index" ? "STAGED DIFF" : "WORKTREE DIFF";
  }
  if (workspace.kind === "imageDiff") {
    if (workspace.conflict) return MERGE_CONFLICT_SUBTITLE;
    return workspace.area === "index" ? "STAGED IMAGE DIFF" : "WORKTREE IMAGE DIFF";
  }
  return workspace.path;
}

/** Existing files offer title drag-out and file actions. */
function titleFilePath(workspace: ActiveRepoWorkspace): string | null {
  if (workspace.kind === "textFile" || workspace.kind === "imageFile") return workspace.path;
  if (workspace.kind === "diff" && workspace.fileExists) return workspace.path;
  if (workspace.kind === "imageDiff" && workspace.fileExists) return workspace.path;
  return null;
}

function semanticMode(workspace: ActiveRepoWorkspace): TextWorkspaceSemanticMode {
  if (workspace.kind === "note") return "markdown";
  if (workspace.kind !== "textFile") return "plain";
  const extension = getExtension(workspace.path);
  return extension !== null && MARKDOWN_LIKE_EXTENSIONS.has(extension) ? "markdown" : "plain";
}

export function RepoWorkspacePanel({
  workspace,
  repoLabel,
  isHandset,
  onClose,
  onTextChange,
  onTextFileDragStart,
  onImageDragStart,
}: RepoWorkspacePanelProps): React.JSX.Element {
  const { config } = useConfig();
  const fileActions = useFileActions();
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const repoRoot = config.repos.find((repo) => repo.repoId === workspace.repoId)?.root;
  const filePath = titleFilePath(workspace);

  const closeContextMenu = useCallback(() => {
    setContextMenu(null);
  }, []);

  const handleTitleContextMenu = useCallback(
    (event: React.MouseEvent) => {
      if (filePath === null || !repoRoot) return;
      setContextMenu({ x: event.clientX, y: event.clientY, path: filePath });
    },
    [filePath, repoRoot]
  );

  const contextMenuItems: ContextMenuItem[] = contextMenu && repoRoot
    ? buildSingleFileContextMenuItems({
      repoRoot,
      path: contextMenu.path,
      fileActions,
      logScope: "RepoWorkspacePanel",
    })
    : [];

  const content =
    workspace.kind === "note" ? (
      <TextWorkspaceEditor
        value={workspace.noteState.content}
        onChange={workspace.noteState.onChange}
        semanticMode={semanticMode(workspace)}
        isLoading={workspace.noteState.isLoading}
        error={workspace.noteState.error}
        maxLength={MAX_NOTE_LENGTH}
        placeholder="Type notes here..."
        ariaLabel="Repository notes"
      />
    ) : workspace.kind === "textFile" ? (
      workspace.status === "loading" ? (
        <p className="empty-state empty-state--waiting">Loading text</p>
      ) : workspace.status === "error" ? (
        <p className="text-workspace-error text-workspace-error--inline">{workspace.error}</p>
      ) : (
        <TextWorkspaceEditor
          value={workspace.content}
          onChange={onTextChange}
          semanticMode={semanticMode(workspace)}
          maxLength={MAX_SCRATCH_LENGTH}
          placeholder="Empty file"
          ariaLabel={`Scratch text buffer for ${workspace.path}`}
        />
      )
    ) : workspace.kind === "diff" ? (
      <DiffWorkspaceViewer
        path={workspace.path}
        isLoading={workspace.status === "loading"}
        error={workspace.status === "error" ? workspace.error : null}
        patch={workspace.status === "ready" ? workspace.patch : undefined}
        truncated={workspace.status === "ready" ? workspace.truncated : undefined}
        binary={workspace.status === "ready" ? workspace.binary : undefined}
        conflict={workspace.conflict}
      />
    ) : workspace.kind === "imageDiff" ? (
      <ImageDiffWorkspaceViewer
        path={workspace.path}
        isLoading={workspace.status === "loading"}
        error={workspace.status === "error" ? workspace.error : null}
        before={workspace.status === "ready" ? workspace.before : null}
        after={workspace.status === "ready" ? workspace.after : null}
        conflict={workspace.conflict}
      />
    ) : (
      <ImageWorkspaceViewer
        path={workspace.path}
        isLoading={workspace.status === "loading"}
        error={workspace.status === "error" ? workspace.error : null}
        dataBase64={workspace.status === "ready" ? workspace.dataBase64 : undefined}
        mimeType={workspace.status === "ready" ? workspace.mimeType : undefined}
        onDragStart={() => {
          onImageDragStart(workspace.path);
        }}
      />
    );

  return (
    <>
      <WorkspaceLayout
        title={repoLabel ? `${repoLabel} · ${workspaceTitle(workspace)}` : workspaceTitle(workspace)}
        subtitle={workspaceSubtitle(workspace)}
        subtitleTone={
          (workspace.kind === "diff" || workspace.kind === "imageDiff") && workspace.conflict
            ? "alert"
            : undefined
        }
        onClose={onClose}
        onTitleContextMenu={filePath !== null && repoRoot ? handleTitleContextMenu : undefined}
        onTitleDragStart={
          filePath !== null
            ? () => {
              if (workspace.kind === "imageFile") onImageDragStart(filePath);
              else void onTextFileDragStart(filePath);
            }
            : undefined
        }
        content={content}
        isHandset={isHandset}
      />
      {contextMenu && repoRoot && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenuItems}
          onClose={closeContextMenu}
        />
      )}
    </>
  );
}
