// Path: app/src/hooks/bundles/use_bundle_events.ts
// Description: Agent event handling for bundle build state

import { useCallback, type Dispatch, type MutableRefObject, type SetStateAction } from "react";
import type { AgentEvent } from "../../shared/protocol.js";
import type {
  BundleBuildProgress,
  BundlePresetState,
  BundleProgressThrottleEntry,
} from "./bundle_state_types.js";

const PROGRESS_THROTTLE_MS = 500;

interface UseBundleEventsOptions {
  repoId: string;
  refreshBundles: (presetId: string) => Promise<void>;
  setPresets: Dispatch<SetStateAction<Map<string, BundlePresetState>>>;
  lastProgressUpdateRef: MutableRefObject<Map<string, BundleProgressThrottleEntry>>;
}

function buildProgress(event: Extract<AgentEvent, { type: "bundleBuildProgress" }>): BundleBuildProgress {
  const progress: BundleBuildProgress = {
    phase: event.phase,
    filesDone: event.filesDone,
    filesTotal: event.filesTotal,
  };
  if (event.currentFile !== undefined) {
    progress.currentFile = event.currentFile;
  }
  if (event.currentBytesDone !== undefined) {
    progress.currentBytesDone = event.currentBytesDone;
  }
  if (event.currentBytesTotal !== undefined) {
    progress.currentBytesTotal = event.currentBytesTotal;
  }
  if (event.bytesDoneTotalBestEffort !== undefined) {
    progress.bytesDoneTotalBestEffort = event.bytesDoneTotalBestEffort;
  }
  return progress;
}

function buildThrottleEntry(
  event: Extract<AgentEvent, { type: "bundleBuildProgress" }>,
  ts: number
): BundleProgressThrottleEntry {
  const snapshot: BundleProgressThrottleEntry = {
    ts,
    phase: event.phase,
    filesDone: event.filesDone,
    filesTotal: event.filesTotal,
  };
  if (event.currentFile !== undefined) {
    snapshot.currentFile = event.currentFile;
  }
  return snapshot;
}

function shouldApplyProgress(
  event: Extract<AgentEvent, { type: "bundleBuildProgress" }>,
  lastEntry: BundleProgressThrottleEntry | undefined,
  now: number
): boolean {
  return (
    !lastEntry ||
    event.phase !== lastEntry.phase ||
    event.currentFile !== lastEntry.currentFile ||
    event.filesDone !== lastEntry.filesDone ||
    event.filesTotal !== lastEntry.filesTotal ||
    now - lastEntry.ts >= PROGRESS_THROTTLE_MS
  );
}

export function useBundleEvents({
  repoId,
  refreshBundles,
  setPresets,
  lastProgressUpdateRef,
}: UseBundleEventsOptions): (event: AgentEvent) => void {
  return useCallback(
    (event: AgentEvent) => {
      if (event.type === "bundleBuilt" && event.repoId === repoId) {
        setPresets((prev) => {
          const next = new Map(prev);
          const preset = next.get(event.presetId);
          if (preset) {
            next.set(event.presetId, {
              ...preset,
              isBuilding: false,
              isCancelling: false,
              activeBuildId: null,
              buildProgress: null,
              lastBuildError: null,
              freshlyBuiltAt: Date.now(),
            });
          }
          return next;
        });
        void refreshBundles(event.presetId);
      }

      if (event.type === "bundleBuildProgress" && event.repoId === repoId) {
        const now = Date.now();
        const lastEntry = lastProgressUpdateRef.current.get(event.presetId);
        if (!shouldApplyProgress(event, lastEntry, now)) {
          return;
        }
        setPresets((prev) => {
          const preset = prev.get(event.presetId);
          if (!preset?.activeBuildId || !preset.isBuilding) {
            return prev;
          }
          lastProgressUpdateRef.current.set(event.presetId, buildThrottleEntry(event, now));
          const next = new Map(prev);
          next.set(event.presetId, {
            ...preset,
            isBuilding: true,
            isCancelling: preset.isCancelling,
            buildProgress: buildProgress(event),
            lastBuildError: null,
          });
          return next;
        });
      }
    },
    [lastProgressUpdateRef, refreshBundles, repoId, setPresets]
  );
}
