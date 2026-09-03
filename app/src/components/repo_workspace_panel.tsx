// Path: app/src/components/repo_workspace_panel.tsx
// Description: Repo workspace renderer for notes, text scratch buffers, image previews, and diffs

import type React from "react";
import { useCallback, useState } from "react";
import { ContextMenu, type ContextMenuItem } from "./context_menu.js";
import { buildSingleFileContextMenuItems } from "./file_context_menu_items.js";
import { DiffWorkspaceViewer } from "./diff_workspace.js";
import { ImageWorkspaceViewer } from "./image_workspace.js";
import { TextWorkspaceEditor } from "./text_workspace.js";
import { WorkspaceLayout } from "./layout/workspace_layout.js";
import { useConfig } from "../hooks/use_config.js";
import { useFileActions } from "../hooks/use_file_actions.js";
import { getFileName, type RepoWorkspace } from "../hooks/use_repo_workspace.js";
import type { NoteState } from "../hooks/use_notes.js";
import type { TextWorkspaceSemanticMode } from "./text_workspace_semantics.js";

const MAX_NOTE_LENGTH = 100_000;
const MAX_SCRATCH_LENGTH = 1_000_000;

type ActiveRepoWorkspace = Exclude<RepoWorkspace, { kind: "none" }>;

interface RepoWorkspacePanelProps {
  workspace: ActiveRepoWorkspace;
  noteState: NoteState;
  railContent: React.ReactNode;
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
  if (workspace.kind === "diff") return workspace.path;
  return getFileName(workspace.path);
}

function workspaceSubtitle(workspace: ActiveRepoWorkspace): string {
  if (workspace.kind === "note") return "Repository notes";
  if (workspace.kind === "diff") return workspace.area === "index" ? "STAGED DIFF" : "WORKTREE DIFF";
  return workspace.path;
}

/** Path whose title offers drag-out and file actions: text buffers, and diffs of files still on disk */
function titleFilePath(workspace: ActiveRepoWorkspace): string | null {
  if (workspace.kind === "textFile") return workspace.path;
  if (workspace.kind === "diff" && workspace.fileExists) return workspace.path;
  return null;
}

function getExtension(path: string): string | null {
  const fileName = getFileName(path);
  const dotIndex = fileName.lastIndexOf(".");
  if (dotIndex === -1 || dotIndex === fileName.length - 1) return null;
  return fileName.slice(dotIndex + 1).toLowerCase();
}

function semanticMode(workspace: ActiveRepoWorkspace): TextWorkspaceSemanticMode {
  if (workspace.kind === "note") return "markdown";
  if (workspace.kind !== "textFile") return "plain";
  const extension = getExtension(workspace.path);
  return extension !== null && MARKDOWN_LIKE_EXTENSIONS.has(extension) ? "markdown" : "plain";
}

export function RepoWorkspacePanel({
  workspace,
  noteState,
  railContent,
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
        value={noteState.content}
        onChange={noteState.onChange}
        semanticMode={semanticMode(workspace)}
        isLoading={noteState.isLoading}
        error={noteState.error}
        maxLength={MAX_NOTE_LENGTH}
        placeholder="Type notes here..."
        ariaLabel="Repository notes"
      />
    ) : workspace.kind === "textFile" ? (
      <TextWorkspaceEditor
        value={workspace.content}
        onChange={onTextChange}
        semanticMode={semanticMode(workspace)}
        maxLength={MAX_SCRATCH_LENGTH}
        placeholder="Empty file"
        ariaLabel={`Scratch text buffer for ${workspace.path}`}
      />
    ) : workspace.kind === "diff" ? (
      <DiffWorkspaceViewer
        path={workspace.path}
        isLoading={workspace.status === "loading"}
        error={workspace.status === "error" ? workspace.error : null}
        patch={workspace.status === "ready" ? workspace.patch : undefined}
        truncated={workspace.status === "ready" ? workspace.truncated : undefined}
        binary={workspace.status === "ready" ? workspace.binary : undefined}
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
        title={workspaceTitle(workspace)}
        subtitle={workspaceSubtitle(workspace)}
        onClose={onClose}
        onTitleContextMenu={filePath !== null && repoRoot ? handleTitleContextMenu : undefined}
        onTitleDragStart={
          filePath !== null
            ? () => {
              void onTextFileDragStart(filePath);
            }
            : undefined
        }
        content={content}
        railContent={railContent}
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
