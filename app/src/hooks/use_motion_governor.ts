// Path: app/src/hooks/use_motion_governor.ts
// Description: Pauses motion when window is not foreground (hidden, minimized, or unfocused) to save GPU

import { useState, useEffect } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { isForegroundWindow } from "../lib/window/foreground.js";

export interface UseMotionGovernorResult {
  motionPaused: boolean;
  /** Hidden or minimized (not merely unfocused): the Stream lands cards instantly and pauses admission */
  documentHidden: boolean;
}

function isDocumentHidden(): boolean {
  return document.hidden || document.visibilityState !== "visible";
}

/**
 * Signals that animations should pause whenever the window is not truly in the
 * foreground — hidden, minimized, OR visible-but-unfocused (the user switched to
 * another app). Pausing releases the GPU compositor from repainting decorative
 * and status animations while nobody is looking; everything resumes on refocus.
 *
 * Uses document visibility plus DOM focus/blur as the primary cross-platform
 * signal, with Tauri window focus events as a secondary signal for Windows edge
 * cases where DOM blur can be unreliable.
 */
export function useMotionGovernor(): UseMotionGovernorResult {
  const [motionPaused, setMotionPaused] = useState<boolean>(
    () => !isForegroundWindow()
  );
  const [documentHidden, setDocumentHidden] = useState<boolean>(isDocumentHidden);

  useEffect(() => {
    let mounted = true;

    // Primary: recompute from live visibility + focus state.
    const recompute = (): void => {
      if (mounted) {
        setMotionPaused(!isForegroundWindow());
        setDocumentHidden(isDocumentHidden());
      }
    };
    document.addEventListener("visibilitychange", recompute);
    window.addEventListener("blur", recompute);
    window.addEventListener("focus", recompute);

    // Secondary: Tauri window focus events. On Windows, DOM blur/focus may not
    // fire reliably; the payload is authoritative, so set directly from it to
    // avoid a document.hasFocus() timing race at the transition instant.
    let unlistenFocus: (() => void) | null = null;

    const setupTauriListener = async (): Promise<void> => {
      try {
        const appWindow = getCurrentWindow();
        const unlisten = await appWindow.onFocusChanged(({ payload: focused }) => {
          if (!mounted) return;
          setMotionPaused(!focused || document.hidden);
          setDocumentHidden(isDocumentHidden());
        });

        if (mounted) {
          unlistenFocus = unlisten;
        } else {
          unlisten();
        }
      } catch {
        // Tauri APIs may not be available in dev/test environments.
        // DOM visibility + focus/blur is a sufficient fallback.
      }
    };

    void setupTauriListener();

    return () => {
      mounted = false;
      document.removeEventListener("visibilitychange", recompute);
      window.removeEventListener("blur", recompute);
      window.removeEventListener("focus", recompute);
      unlistenFocus?.();
    };
  }, []);

  return { motionPaused, documentHidden };
}
