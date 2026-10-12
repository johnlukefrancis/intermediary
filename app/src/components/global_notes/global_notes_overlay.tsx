// Path: app/src/components/global_notes/global_notes_overlay.tsx
// Description: Full-page global markdown scratchpad layered over the current view

import type React from "react";
import { useCallback } from "react";
import { useNotes } from "../../hooks/use_notes.js";
import { TextWorkspaceEditor } from "../text_workspace.js";
import "../../styles/global_notes.css";

const MAX_NOTE_LENGTH = 100_000;

export function GlobalNotesOverlay({ onClose }: { onClose: () => void }): React.JSX.Element {
  const { content, isLoading, error, onChange } = useNotes({ kind: "global" });

  const handleKeyDown = useCallback((event: React.KeyboardEvent) => {
    if (event.key !== "Escape") return;
    event.stopPropagation();
    onClose();
  }, [onClose]);

  return (
    <section className="global-notes" role="dialog" aria-label="Global notes" onKeyDown={handleKeyDown}>
      <div className="global-notes-sheet panel">
        <button type="button" className="global-notes-close" onClick={onClose}
          aria-label="Close notes" title="Close (Esc)">
          ×
        </button>
        <div className="global-notes-body">
          <TextWorkspaceEditor
            value={content}
            onChange={onChange}
            semanticMode="markdown"
            isLoading={isLoading}
            error={error}
            maxLength={MAX_NOTE_LENGTH}
            placeholder=""
            ariaLabel="Global notes"
            autoFocus
          />
        </div>
      </div>
    </section>
  );
}
