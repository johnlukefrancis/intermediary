// Path: app/src/hooks/overview/use_overview.ts
// Description: Overview subscription and visible-only age clock

import { useEffect, useState, useSyncExternalStore } from "react";
import { overviewStore } from "../../lib/overview/overview_store.js";
import type { OverviewSnapshot } from "../../lib/overview/overview_types.js";

const subscribe = (listener: () => void): (() => void) => overviewStore.subscribe(listener);
const getSnapshot = (): OverviewSnapshot => overviewStore.getSnapshot();

export function useOverview(documentHidden: boolean): {
  snapshot: OverviewSnapshot;
  now: number;
} {
  const snapshot = useSyncExternalStore(subscribe, getSnapshot);
  const [now, setNow] = useState(Date.now);

  useEffect(() => {
    if (documentHidden) return;
    setNow(Date.now());
    const timer = window.setInterval(() => { setNow(Date.now()); }, 5_000);
    return () => { window.clearInterval(timer); };
  }, [documentHidden]);

  return { snapshot, now: Math.max(now, Date.now()) };
}
