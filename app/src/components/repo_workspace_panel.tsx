// Path: app/src/components/repo_workspace_panel.tsx
// Description: Repo workspace renderer for notes, text scratch buffers, and image previews

import type React from "react";
import { ImageWorkspaceViewer } from "./image_workspace.js";
import { TextWorkspaceEditor } from "./text_workspace.js";
import { WorkspaceLayout } from "./layout/workspace_layout.js";
import { getFileName, type RepoWorkspace } from "../hooks/use_repo_workspace.js";
import type { NoteState } from "../hooks/use_notes.js";

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
  onImageDragStart: (path: string) => void;
}

function workspaceTitle(workspace: ActiveRepoWorkspace): string {
  return workspace.kind === "note" ? "Note" : getFileName(workspace.path);
}

function workspaceSubtitle(workspace: ActiveRepoWorkspace): string {
  return workspace.kind === "note" ? "Repository notes" : workspace.path;
}

export function RepoWorkspacePanel({
  workspace,
  noteState,
  zipsContent,
  isHandset,
  onClose,
  onTextChange,
  onImageDragStart,
}: RepoWorkspacePanelProps): React.JSX.Element {
  const content =
    workspace.kind === "note" ? (
      <TextWorkspaceEditor
        value={noteState.content}
        onChange={noteState.onChange}
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
    <WorkspaceLayout
      title={workspaceTitle(workspace)}
      subtitle={workspaceSubtitle(workspace)}
      onClose={onClose}
      content={content}
      zipsContent={zipsContent}
      isHandset={isHandset}
    />
  );
}
