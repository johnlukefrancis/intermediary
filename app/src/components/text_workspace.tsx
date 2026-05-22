// Path: app/src/components/text_workspace.tsx
// Description: Shared minimal textarea surface for notes and scratch file viewing

import type React from "react";

interface TextWorkspaceEditorProps {
  value: string;
  onChange: (value: string) => void;
  ariaLabel: string;
  placeholder: string;
  isLoading?: boolean;
  error?: string | null;
  maxLength: number;
}

function countText(value: string): { lines: number; characters: number } {
  if (value.length === 0) {
    return { lines: 0, characters: 0 };
  }

  const normalized = value.replace(/\r\n/g, "\n").replace(/\r/g, "\n");
  return {
    lines: normalized.split("\n").length,
    characters: value.length,
  };
}

export function TextWorkspaceEditor({
  value,
  onChange,
  ariaLabel,
  placeholder,
  isLoading = false,
  error = null,
  maxLength,
}: TextWorkspaceEditorProps): React.JSX.Element {
  const counts = countText(value);

  if (isLoading) {
    return <p className="empty-state empty-state--waiting">Loading text</p>;
  }

  return (
    <div className="text-workspace-editor-shell">
      {error && <p className="text-workspace-error">{error}</p>}
      <textarea
        className="text-workspace-textarea"
        value={value}
        onChange={(event) => {
          onChange(event.target.value);
        }}
        placeholder={placeholder}
        maxLength={maxLength}
        spellCheck={false}
        aria-label={ariaLabel}
      />
      <div className="text-workspace-count" aria-live="polite">
        {counts.lines} lines / {counts.characters} chars
      </div>
    </div>
  );
}
