// Path: app/src/components/system_load/load_panel.tsx
// Description: Fixed glass detail panel ("context window") placed below its meter and clamped to the window

import { useEffect, useLayoutEffect, type RefObject } from "react";
import type React from "react";
import { createPortal } from "react-dom";
import type { LoadState } from "../../lib/system_load/load_state.js";
import { PANEL_TITLES, type MeterKey } from "../../lib/system_load/meters.js";
import type { LoadSeverity } from "../../lib/system_load/severity.js";
import { PanelBody } from "./panel_bodies.js";
import "../../styles/context_menu.css";
import "../../styles/system_load_panel.css";

export const LOAD_PANEL_ID = "sysload-panel";
const EDGE_PX = 8;
const GAP_PX = 6;

interface LoadPanelProps {
  meterKey: MeterKey;
  anchor: HTMLElement | null;
  state: LoadState;
  severity: LoadSeverity;
  pinned: boolean;
  onPointerEnter: () => void;
  onPointerLeave: () => void;
  panelRef: RefObject<HTMLDivElement>;
}

function placePanel(panel: HTMLDivElement | null, anchor: HTMLElement | null): void {
  if (!panel || !anchor) return;
  const rect = anchor.getBoundingClientRect();
  const width = panel.offsetWidth;
  const centered = rect.left + rect.width / 2 - width / 2;
  const top = rect.bottom + GAP_PX;
  panel.style.left = `${Math.max(EDGE_PX, Math.min(centered, window.innerWidth - width - EDGE_PX))}px`;
  panel.style.top = `${top}px`;
  panel.style.maxHeight = `${Math.max(120, window.innerHeight - top - EDGE_PX)}px`;
}

export function LoadPanel({
  meterKey, anchor, state, severity, pinned, onPointerEnter, onPointerLeave, panelRef,
}: LoadPanelProps): React.JSX.Element {
  useLayoutEffect(() => { placePanel(panelRef.current, anchor); });
  useEffect(() => {
    const onResize = (): void => { placePanel(panelRef.current, anchor); };
    window.addEventListener("resize", onResize);
    return () => { window.removeEventListener("resize", onResize); };
  }, [anchor, panelRef]);

  return createPortal(
    <div
      ref={panelRef}
      id={LOAD_PANEL_ID}
      role="group"
      aria-label={`${PANEL_TITLES[meterKey]} detail`}
      className="glass-popover sysload-panel"
      data-severity={severity}
      data-meter={meterKey}
      data-pinned={pinned ? "true" : undefined}
      onPointerEnter={onPointerEnter}
      onPointerLeave={onPointerLeave}
    >
      <header className="sysload-panel__header">
        <h2 className="sysload-panel__title">{PANEL_TITLES[meterKey]}</h2>
        {pinned ? <span className="sysload-panel__hint">Pinned</span> : null}
      </header>
      <div className="sysload-panel__body">
        <PanelBody meterKey={meterKey} state={state} severity={severity} />
      </div>
    </div>,
    document.body
  );
}
