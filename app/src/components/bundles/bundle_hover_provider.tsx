// Path: app/src/components/bundles/bundle_hover_provider.tsx
// Description: One delayed, cancellable metadata inspection per bundle tree

import type React from "react";
import { createContext, useCallback, useContext, useEffect, useId, useMemo, useRef, useState } from "react";
import { useAgent } from "../../hooks/use_agent.js";
import type { BundleSelection } from "../../shared/protocol.js";
import type { GlobalExcludes } from "../../shared/global_excludes.js";
import type { InspectBundleEntryResult } from "../../shared/protocol_bundle_inspection.js";
import { BundleEntryHover, type BundleHoverState } from "./bundle_entry_hover.js";

interface HoverContext {
  show: (path: string, row: HTMLElement) => void;
  hide: () => void;
  activePath: string | null;
  tooltipId: string;
}

const HoverContext = createContext<HoverContext | null>(null);

interface Props {
  repoId: string;
  selection: BundleSelection;
  globalExcludes: GlobalExcludes;
  disabled: boolean;
  children: React.ReactNode;
}

export function BundleHoverProvider({ repoId, selection, globalExcludes, disabled, children }: Props): React.JSX.Element {
  const { client, helloState } = useAgent();
  const tooltipId = useId();
  const [hover, setHover] = useState<BundleHoverState | null>(null);
  const pending = useRef<{ timer: ReturnType<typeof setTimeout>; controller: AbortController } | null>(null);
  const hide = useCallback(() => {
    if (pending.current) {
      clearTimeout(pending.current.timer);
      pending.current.controller.abort();
      pending.current = null;
    }
    setHover(null);
  }, []);

  useEffect(() => {
    hide();
    return hide;
  }, [repoId, selection, globalExcludes, disabled, client, helloState.status, hide]);

  useEffect(() => {
    const keyDown = (event: KeyboardEvent): void => { if (event.key === "Escape") hide(); };
    document.addEventListener("pointerdown", hide, true);
    document.addEventListener("contextmenu", hide, true);
    document.addEventListener("scroll", hide, true);
    document.addEventListener("keydown", keyDown);
    window.addEventListener("blur", hide);
    window.addEventListener("resize", hide);
    return () => {
      document.removeEventListener("pointerdown", hide, true);
      document.removeEventListener("contextmenu", hide, true);
      document.removeEventListener("scroll", hide, true);
      document.removeEventListener("keydown", keyDown);
      window.removeEventListener("blur", hide);
      window.removeEventListener("resize", hide);
    };
  }, [hide]);

  const show = useCallback((path: string, row: HTMLElement) => {
    hide();
    if (disabled) return;
    const anchor = row.getBoundingClientRect();
    const controller = new AbortController();
    const timer = setTimeout(() => {
      if (controller.signal.aborted) return;
      const initial: BundleHoverState = { path, anchor, inspection: null, error: null };
      setHover(initial);
      if (!client || helloState.status !== "ok") {
        setHover({ ...initial, error: "Connect to the repository to measure its size." });
        return;
      }
      void client.send<InspectBundleEntryResult>({
        type: "inspectBundleEntry", repoId, path, selection, globalExcludes,
      }, controller.signal).then((result) => {
        if (!controller.signal.aborted) setHover({ ...initial, inspection: result.inspection });
      // TODO(protocol): Keep narrowing dynamic transport rejections at this boundary.
      }).catch((error: unknown) => {
        if (!controller.signal.aborted) setHover({ ...initial, error: error instanceof Error ? error.message : "Unable to measure size." });
      });
    }, 400);
    pending.current = { timer, controller };
  }, [client, disabled, globalExcludes, helloState.status, hide, repoId, selection]);

  const value = useMemo(() => ({ show, hide, activePath: hover?.path ?? null, tooltipId }), [show, hide, hover?.path, tooltipId]);
  return (
    <HoverContext.Provider value={value}>
      {children}
      {hover && <BundleEntryHover id={tooltipId} hover={hover} />}
    </HoverContext.Provider>
  );
}

export function useBundleHover(path: string): React.HTMLAttributes<HTMLDivElement> {
  const context = useContext(HoverContext);
  return {
    onPointerEnter: (event) => { if (event.pointerType !== "touch") context?.show(path, event.currentTarget); },
    onPointerLeave: () => { context?.hide(); },
    onFocus: (event) => { context?.show(path, event.currentTarget); },
    onBlur: () => { context?.hide(); },
    "aria-describedby": context?.activePath === path ? context.tooltipId : undefined,
  };
}
