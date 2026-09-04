// Path: app/src/hooks/terminal/use_terminal_group.ts
// Description: Subscribes a component to one repo's terminal group snapshot from the module-level registry

import { useCallback, useSyncExternalStore } from "react";
import { terminalRegistry } from "../../lib/terminal/terminal_registry.js";
import type { TerminalGroupSnapshot } from "../../lib/terminal/terminal_types.js";

function subscribe(listener: () => void): () => void {
  return terminalRegistry.subscribe(listener);
}

export function useTerminalGroup(repoId: string): TerminalGroupSnapshot {
  const getSnapshot = useCallback(() => terminalRegistry.getGroupSnapshot(repoId), [repoId]);
  return useSyncExternalStore(subscribe, getSnapshot, getSnapshot);
}
