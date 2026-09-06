// Path: app/src/hooks/stream/use_deferred_click.ts
// Description: A single click that fires only once the double-click grace has passed; shared by file cards and image strips

import { useCallback, useEffect, useRef } from "react";
import { DBLCLICK_GRACE_MS } from "../../lib/stream/stream_bounds.js";

export interface DeferredClick {
  /** Arm the action; a second click inside DBLCLICK_GRACE_MS must call `cancel` first */
  click: () => void;
  cancel: () => void;
}

/** A single click expands only once the double-click grace has passed without a second click */
export function useDeferredClick(action: () => void): DeferredClick {
  const timerRef = useRef<number | null>(null);
  const cancel = useCallback(() => {
    if (timerRef.current === null) return;
    window.clearTimeout(timerRef.current);
    timerRef.current = null;
  }, []);
  useEffect(() => cancel, [cancel]);
  const click = useCallback(() => {
    cancel();
    timerRef.current = window.setTimeout(() => {
      timerRef.current = null;
      action();
    }, DBLCLICK_GRACE_MS);
  }, [action, cancel]);
  return { click, cancel };
}
