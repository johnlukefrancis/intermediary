// Path: app/src/components/file_list_column.tsx
// Description: Column wrapper that renders a list of FileRow components

import type React from "react";
import { FileRow } from "./file_row.js";
import type { FileEntry, StagedInfo } from "../shared/protocol.js";

interface FileListColumnProps {
  files: FileEntry[];
  stagedByPath: Map<string, StagedInfo>;
  repoId: string;
  emptyMessage?: string;
  onDragStart: (
    repoId: string,
    relativePath: string,
    stagedInfo: StagedInfo | undefined
  ) => void | Promise<void>;
}

export function FileListColumn({
  files,
  stagedByPath,
  repoId,
  emptyMessage = "No files",
  onDragStart,
}: FileListColumnProps): React.JSX.Element {
  if (files.length === 0) {
    return <p className="placeholder">{emptyMessage}</p>;
  }

  return (
    <div className="file-list">
      {files.map((file) => (
        <FileRow
          key={file.path}
          file={file}
          repoId={repoId}
          stagedInfo={stagedByPath.get(file.path)}
          onDragStart={onDragStart}
        />
      ))}
    </div>
  );
}
