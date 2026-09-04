// Path: app/src/hooks/terminal/use_terminal_host.ts
// Description: Adopts the active terminal tab into a host element for its mount, parks on cleanup, and keeps it fitted, themed and focused

import { useCallback, useEffect, useLayoutEffect, useState } from "react";
import type React from "react";
import { terminalRegistry } from "../../lib/terminal/terminal_registry.js";

/** Present while a modal owns the keyboard; the terminal must not steal focus then */
const MODAL_ROOT_SELECTOR = "[data-intermediary-modal-root]";

/**
 * Returns the ref callback for the host element. Adopt/park run in a layout effect so the
 * element is moved before React detaches the host on a rail, repo, handset or mode switch;
 * every step is idempotent at the registry, which keeps StrictMode's double effects harmless.
 */
export function useTerminalHost(
  repoId: string,
  tabId: string | null,
  themeKey: string
): React.RefCallback<HTMLDivElement> {
  const [host, setHost] = useState<HTMLDivElement | null>(null);
  const hostRef = useCallback<React.RefCallback<HTMLDivElement>>((element) => {
    setHost(element);
  }, []);

  useLayoutEffect(() => {
    if (host === null || tabId === null) return;
    terminalRegistry.adopt(repoId, tabId, host);
    if (document.querySelector(MODAL_ROOT_SELECTOR) === null) terminalRegistry.focusTab(tabId);

    let frame: number | null = null;
    const scheduleFit = (): void => {
      if (frame !== null) return;
      frame = requestAnimationFrame(() => {
        frame = null;
        terminalRegistry.fitTab(tabId);
      });
    };
    const observer = new ResizeObserver(scheduleFit);
    observer.observe(host);
    let cancelled = false;
    void document.fonts.ready.then(() => {
      if (!cancelled) scheduleFit();
    });

    return () => {
      cancelled = true;
      observer.disconnect();
      if (frame !== null) cancelAnimationFrame(frame);
      terminalRegistry.park(tabId);
    };
  }, [host, repoId, tabId]);

  // The tokens the theme is read from (mode, opacity on the document root; accent inline on `.app`)
  // are stamped by App's effects, which run after this hook's; one frame later they are all current.
  useEffect(() => {
    const frame = requestAnimationFrame(() => {
      terminalRegistry.applyTheme();
    });
    return () => {
      cancelAnimationFrame(frame);
    };
  }, [themeKey]);

  return hostRef;
}
