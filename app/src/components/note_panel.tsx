// Path: app/src/components/note_panel.tsx
// Description: Plain-text monospace note editor panel content

import type React from "react";
import type { NoteState } from "../hooks/use_notes.js";

interface NotePanelProps {
  noteState: NoteState;
}

export function NotePanel({ noteState }: NotePanelProps): React.JSX.Element {
  const { content, isLoading, error, onChange } = noteState;

  if (isLoading) {
    return <p className="empty-state empty-state--waiting">Loading notes</p>;
  }

  return (
    <>
      {error && <p className="note-error">{error}</p>}
      <textarea
        className="note-textarea"
        value={content}
        onChange={(e) => {
          onChange(e.target.value);
        }}
        placeholder="Type notes here..."
        maxLength={100_000}
        spellCheck={false}
        aria-label="Repository notes"
      />
    </>
  );
}
