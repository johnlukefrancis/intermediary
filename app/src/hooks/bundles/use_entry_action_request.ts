// Path: app/src/hooks/bundles/use_entry_action_request.ts
// Description: Owns the ZIPS-tree entry-action transaction: refuse-first with conflict confirmation, inline error reporting

import { useCallback, useEffect, useRef, useState } from "react";
import { useAgent } from "../use_agent.js";
import { sendWorktreeAction } from "../../lib/agent/messages_worktree.js";
import type { ConflictPolicy, WorktreeAction } from "../../shared/protocol.js";
import {
  appliedPathsFromError,
  entryConflictPaths,
  isEntryConflictError,
} from "../../lib/agent/error_codes.js";

export type EntryActionKind = "delete" | "move" | "copy" | "rename";

export interface PendingReplace {
  action: "move" | "copy";
  paths: string[];
  directory: string;
  conflicts: string[];
}

export interface UseEntryActionRequestOptions {
  repoId: string;
  /** Called after a successful action so the explorer can refresh listings and selection. */
  onApplied: (kind: EntryActionKind, entries: string[], sourcePaths: string[]) => void;
}

export interface EntryActionRequestState {
  deleteEntries: (paths: string[]) => void;
  moveEntries: (paths: string[], directory: string) => void;
  copyEntries: (paths: string[], directory: string) => void;
  renameEntry: (path: string, newName: string) => void;
  inFlight: boolean;
  pendingReplace: PendingReplace | null;
  confirmReplace: () => void;
  cancelReplace: () => void;
  error: string | null;
  dismissError: () => void;
}

export function useEntryActionRequest({
  repoId,
  onApplied,
}: UseEntryActionRequestOptions): EntryActionRequestState {
  const { client } = useAgent();
  const [inFlight, setInFlight] = useState(false);
  const [pendingReplace, setPendingReplace] = useState<PendingReplace | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Only the newest send may write state; a superseded request's settle is ignored.
  const seqRef = useRef(0);

  // Per-repo mutation state must not survive a repo switch (RepoTab renders unkeyed).
  useEffect(() => {
    seqRef.current += 1;
    setPendingReplace(null);
    setError(null);
    setInFlight(false);
  }, [repoId]);

  const run = useCallback(
    (action: WorktreeAction, sourcePaths: string[]): void => {
      if (!client) {
        setError("Not connected");
        return;
      }
      const seq = ++seqRef.current;
      setInFlight(true);
      setError(null);
      sendWorktreeAction(client, repoId, action)
        .then((result) => {
          if (seqRef.current !== seq) return;
          setInFlight(false);
          setPendingReplace(null);
          onApplied(result.kind, result.entries, sourcePaths);
        })
        .catch((err: unknown) => {
          if (seqRef.current !== seq) return;
          setInFlight(false);
          if (
            isEntryConflictError(err) &&
            (action.kind === "move" || action.kind === "copy")
          ) {
            setPendingReplace({
              action: action.kind,
              paths: action.paths,
              directory: action.directory,
              conflicts: entryConflictPaths(err),
            });
            return;
          }
          const message = err instanceof Error ? err.message : "Action failed";
          const applied = appliedPathsFromError(err);
          setError(applied === null ? message : `${applied.length} applied — ${message}`);
        });
    },
    [client, onApplied, repoId]
  );

  const deleteEntries = useCallback(
    (paths: string[]): void => {
      run({ kind: "delete", paths }, paths);
    },
    [run]
  );

  const moveOrCopy = useCallback(
    (kind: "move" | "copy", paths: string[], directory: string, onConflict: ConflictPolicy): void => {
      run({ kind, paths, directory, onConflict }, paths);
    },
    [run]
  );

  const moveEntries = useCallback(
    (paths: string[], directory: string): void => {
      moveOrCopy("move", paths, directory, "refuse");
    },
    [moveOrCopy]
  );

  const copyEntries = useCallback(
    (paths: string[], directory: string): void => {
      moveOrCopy("copy", paths, directory, "refuse");
    },
    [moveOrCopy]
  );

  const renameEntry = useCallback(
    (path: string, newName: string): void => {
      run({ kind: "rename", path, newName }, [path]);
    },
    [run]
  );

  const confirmReplace = useCallback((): void => {
    if (!pendingReplace) return;
    moveOrCopy(pendingReplace.action, pendingReplace.paths, pendingReplace.directory, {
      replace: pendingReplace.conflicts,
    });
  }, [moveOrCopy, pendingReplace]);

  const cancelReplace = useCallback((): void => {
    setPendingReplace(null);
  }, []);

  const dismissError = useCallback((): void => {
    setError(null);
  }, []);

  return {
    deleteEntries,
    moveEntries,
    copyEntries,
    renameEntry,
    inFlight,
    pendingReplace,
    confirmReplace,
    cancelReplace,
    error,
    dismissError,
  };
}
