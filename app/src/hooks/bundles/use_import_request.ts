// Path: app/src/hooks/bundles/use_import_request.ts
// Description: Owns the import transaction: refuse-first with conflict confirmation, inline error reporting

import { useCallback, useEffect, useRef, useState } from "react";
import { useAgent } from "../use_agent.js";
import { sendImportFiles } from "../../lib/agent/messages_import.js";
import type { ConflictPolicy } from "../../shared/protocol.js";
import {
  entryConflictPaths,
  importedCountFromError,
  isEntryConflictError,
} from "../../lib/agent/error_codes.js";

export interface PendingReplace {
  directory: string;
  sources: string[];
  conflicts: string[];
}

export interface UseImportRequestOptions {
  repoId: string;
}

export interface ImportRequestState {
  importFiles: (directory: string, sources: string[]) => void;
  inFlight: boolean;
  pendingReplace: PendingReplace | null;
  confirmReplace: () => void;
  cancelReplace: () => void;
  error: string | null;
  dismissError: () => void;
}

export function useImportRequest({ repoId }: UseImportRequestOptions): ImportRequestState {
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

  const runImport = useCallback(
    (directory: string, sources: string[], onConflict: ConflictPolicy): void => {
      if (!client) {
        setError("Not connected");
        return;
      }
      const seq = ++seqRef.current;
      setInFlight(true);
      setError(null);
      sendImportFiles(client, repoId, directory, sources, onConflict)
        .then(() => {
          if (seqRef.current !== seq) return;
          setInFlight(false);
          setPendingReplace(null);
        })
        .catch((err: unknown) => {
          if (seqRef.current !== seq) return;
          setInFlight(false);
          if (isEntryConflictError(err)) {
            setPendingReplace({ directory, sources, conflicts: entryConflictPaths(err) });
            return;
          }
          const message = err instanceof Error ? err.message : "Import failed";
          const importedCount = importedCountFromError(err);
          setError(importedCount === null ? message : `${importedCount} copied — ${message}`);
        });
    },
    [client, repoId]
  );

  const importFiles = useCallback(
    (directory: string, sources: string[]): void => {
      runImport(directory, sources, "refuse");
    },
    [runImport]
  );

  const confirmReplace = useCallback((): void => {
    if (!pendingReplace) return;
    runImport(pendingReplace.directory, pendingReplace.sources, { replace: pendingReplace.conflicts });
  }, [pendingReplace, runImport]);

  const cancelReplace = useCallback((): void => {
    setPendingReplace(null);
  }, []);

  const dismissError = useCallback((): void => {
    setError(null);
  }, []);

  return { importFiles, inFlight, pendingReplace, confirmReplace, cancelReplace, error, dismissError };
}
