// Path: app/src/components/repo_workspace_panel.tsx
// Description: Repo workspace renderer for notes, text scratch buffers, and image previews

import type React from "react";
import { useCallback, useState } from "react";
import { ContextMenu, type ContextMenuItem } from "./context_menu.js";
import { buildSingleFileContextMenuItems } from "./file_context_menu_items.js";
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
  zipsContent: React.ReactNode;
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
  return workspace.kind === "note" ? "Note" : getFileName(workspace.path);
}

function workspaceSubtitle(workspace: ActiveRepoWorkspace): string {
  return workspace.kind === "note" ? "Repository notes" : workspace.path;
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
  zipsContent,
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

  const closeContextMenu = useCallback(() => {
    setContextMenu(null);
  }, []);

  const handleTitleContextMenu = useCallback(
    (event: React.MouseEvent) => {
      if (workspace.kind !== "textFile" || !repoRoot) return;
      setContextMenu({ x: event.clientX, y: event.clientY, path: workspace.path });
    },
    [repoRoot, workspace]
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
        onTitleContextMenu={
          workspace.kind === "textFile" && repoRoot ? handleTitleContextMenu : undefined
        }
        onTitleDragStart={
          workspace.kind === "textFile"
            ? () => {
              void onTextFileDragStart(workspace.path);
            }
            : undefined
        }
        content={content}
        zipsContent={zipsContent}
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
