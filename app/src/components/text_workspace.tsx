// Path: app/src/components/text_workspace.tsx
// Description: Shared textarea surface for notes and scratch file viewing

import type React from "react";
import { useCallback, useMemo, useRef } from "react";
import {
  TextWorkspaceSemantics,
  type TextWorkspaceSemanticMode,
} from "./text_workspace_semantics.js";

interface TextWorkspaceEditorProps {
  value: string;
  onChange: (value: string) => void;
  ariaLabel: string;
  placeholder: string;
  semanticMode: TextWorkspaceSemanticMode;
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
  semanticMode,
  isLoading = false,
  error = null,
  maxLength,
}: TextWorkspaceEditorProps): React.JSX.Element {
  const counts = useMemo(() => countText(value), [value]);
  const editorShellRef = useRef<HTMLDivElement>(null);
  const handleScroll = useCallback((event: React.UIEvent<HTMLTextAreaElement>): void => {
    const shell = editorShellRef.current;
    if (!shell) return;

    shell.style.setProperty("--text-workspace-scroll-x", `${-event.currentTarget.scrollLeft}px`);
    shell.style.setProperty("--text-workspace-scroll-y", `${-event.currentTarget.scrollTop}px`);
  }, []);

  if (isLoading) {
    return <p className="empty-state empty-state--waiting">Loading text</p>;
  }

  return (
    <div
      ref={editorShellRef}
      className="text-workspace-editor-shell"
      data-semantic-mode={semanticMode}
    >
      {error && <p className="text-workspace-error">{error}</p>}
      <TextWorkspaceSemantics
        value={value}
        placeholder={placeholder}
        mode={semanticMode}
      />
      <textarea
        className="text-workspace-textarea"
        value={value}
        onChange={(event) => {
          onChange(event.target.value);
        }}
        onScroll={handleScroll}
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
