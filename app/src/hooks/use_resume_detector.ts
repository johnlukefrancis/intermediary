// Path: app/src/hooks/use_resume_detector.ts
// Description: Detects likely OS sleep/wake resume using time gaps plus visibility/focus signals

import { useEffect, useRef } from "react";

interface UseResumeDetectorOptions {
  enabled?: boolean;
  onResume: () => void;
  gapThresholdMs?: number;
  pollIntervalMs?: number;
  cooldownMs?: number;
}

const DEFAULT_GAP_THRESHOLD_MS = 30_000;
const DEFAULT_POLL_INTERVAL_MS = 5_000;
const DEFAULT_COOLDOWN_MS = 15_000;

function isForegroundWindow(): boolean {
  return !document.hidden && document.visibilityState === "visible" && document.hasFocus();
}

export function useResumeDetector(options: UseResumeDetectorOptions): void {
  const {
    enabled = true,
    onResume,
    gapThresholdMs = DEFAULT_GAP_THRESHOLD_MS,
    pollIntervalMs = DEFAULT_POLL_INTERVAL_MS,
    cooldownMs = DEFAULT_COOLDOWN_MS,
  } = options;
  const onResumeRef = useRef(onResume);
  const lastTickAtRef = useRef<number>(Date.now());
  const lastResumeAtRef = useRef<number | null>(null);
  const pendingResumeRef = useRef(false);

  useEffect(() => {
    onResumeRef.current = onResume;
  }, [onResume]);

  useEffect(() => {
    if (!enabled) {
      lastTickAtRef.current = Date.now();
      lastResumeAtRef.current = null;
      pendingResumeRef.current = false;
      return;
    }

    const maybeTriggerResume = (): void => {
      const now = Date.now();
      const gapMs = now - lastTickAtRef.current;
      lastTickAtRef.current = now;
      if (gapMs >= gapThresholdMs) {
        pendingResumeRef.current = true;
      }

      if (!pendingResumeRef.current || !isForegroundWindow()) {
        return;
      }
      const lastResumeAt = lastResumeAtRef.current;
      if (lastResumeAt !== null && now - lastResumeAt < cooldownMs) {
        return;
      }
      pendingResumeRef.current = false;
      lastResumeAtRef.current = now;
      onResumeRef.current();
    };

    const intervalId = window.setInterval(maybeTriggerResume, pollIntervalMs);

    const handleVisibilityChange = (): void => {
      if (document.visibilityState === "visible") {
        maybeTriggerResume();
      }
    };

    const handleFocus = (): void => {
      maybeTriggerResume();
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    window.addEventListener("focus", handleFocus);

    return () => {
      window.clearInterval(intervalId);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      window.removeEventListener("focus", handleFocus);
    };
  }, [cooldownMs, enabled, gapThresholdMs, pollIntervalMs]);
}
