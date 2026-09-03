// Path: app/src/hooks/source_control/source_control_commands.ts
// Description: Public stage/unstage/discard/commit/push/pull command surface over the serialized action runner

import { useCallback } from "react";
import type { SourceControlAction } from "../../shared/protocol.js";
import type { SourceControlState } from "./source_control_types.js";

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
    discard: useCallback((paths) => {
      if (paths.length === 0) return;
      void runAction({ kind: "discard", paths });
    }, [runAction]),
    commit: useCallback((message) => {
      if (message.trim().length === 0) return;
      void runAction({ kind: "commit", message });
    }, [runAction]),
    push: useCallback(() => { void runAction({ kind: "push" }); }, [runAction]),
    pull: useCallback(() => { void runAction({ kind: "pull" }); }, [runAction]),
  };
}
