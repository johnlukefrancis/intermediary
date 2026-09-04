// Path: app/src/components/bundles/bundle_entry_rename_input.tsx
// Description: In-place rename input mounted in a ZIPS tree row's name slot

import type React from "react";
import { useEffect, useRef } from "react";

interface BundleEntryRenameInputProps {
  currentName: string;
  inFlight: boolean;
  onCommit: (newName: string) => void;
  onCancel: () => void;
}

export function BundleEntryRenameInput({
  currentName,
  inFlight,
  onCommit,
  onCancel,
}: BundleEntryRenameInputProps): React.JSX.Element {
  const inputRef = useRef<HTMLInputElement | null>(null);
  // Escape sets this so the blur it triggers (moving focus away) does not also commit.
  const cancelledRef = useRef(false);

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  function commit(): void {
    if (cancelledRef.current) {
      cancelledRef.current = false;
      return;
    }
    const trimmed = inputRef.current?.value.trim() ?? "";
    if (trimmed.length === 0 || trimmed === currentName) {
      onCancel();
      return;
    }
    onCommit(trimmed);
  }

  return (
    <input
      ref={inputRef}
      type="text"
      defaultValue={currentName}
      disabled={inFlight}
      className="bundle-entry-rename-input"
      aria-label={`Rename ${currentName}`}
      onClick={(event) => { event.stopPropagation(); }}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === "Enter") {
          event.preventDefault();
          commit();
        } else if (event.key === "Escape") {
          event.preventDefault();
          cancelledRef.current = true;
          onCancel();
        }
      }}
      onBlur={commit}
    />
  );
}
