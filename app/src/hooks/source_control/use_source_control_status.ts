// Path: app/src/hooks/source_control/use_source_control_status.ts
// Description: Read side of source-control status: demands the repo while mounted and selects its registry slot

import { useCallback, useEffect, useSyncExternalStore } from "react";
import { statusRegistry } from "../../lib/source_control/status_registry.js";
import type { StatusSlotView } from "../../lib/source_control/status_registry_types.js";

export function useSourceControlStatus(repoId: string): StatusSlotView {
  useEffect(() => statusRegistry.retain(repoId), [repoId]);
  const subscribe = useCallback(
    (listener: () => void) => statusRegistry.subscribe(repoId, listener),
    [repoId]
  );
  const getSnapshot = useCallback(() => statusRegistry.getSlot(repoId), [repoId]);
  return useSyncExternalStore(subscribe, getSnapshot);
}
