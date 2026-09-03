// Path: app/src/hooks/source_control/source_control_commands.ts
// Description: Public stage/unstage/discard/commit/push/pull command surface over the serialized action runner

import { useCallback } from "react";
import type { SourceControlAction } from "../../shared/protocol.js";
import type { SourceControlCommitRequest, SourceControlState } from "./source_control_types.js";

type SourceControlCommands = Pick<
  SourceControlState,
  "stage" | "unstage" | "discard" | "commit" | "push" | "pull"
>;

export function useSourceControlCommands(
  runAction: (action: SourceControlAction) => Promise<void>
): SourceControlCommands {
  return {
    stage: useCallback((scope) => { void runAction({ kind: "stage", scope }); }, [runAction]),
    unstage: useCallback((scope) => { void runAction({ kind: "unstage", scope }); }, [runAction]),
    discard: useCallback((targets) => {
      if (targets.length === 0) return;
      void runAction({ kind: "discard", targets });
    }, [runAction]),
    // The caller freezes every field from the status it reviewed; nothing here re-reads live state.
    commit: useCallback((request: SourceControlCommitRequest) => {
      if (request.message.trim().length === 0) return;
      if (request.expectedSnapshotId.length === 0) return;
      void runAction({
        kind: "commit",
        message: request.message,
        expectedSnapshotId: request.expectedSnapshotId,
      });
    }, [runAction]),
    push: useCallback(() => { void runAction({ kind: "push" }); }, [runAction]),
    pull: useCallback(() => { void runAction({ kind: "pull" }); }, [runAction]),
  };
}
