// Path: app/src/components/system_load/load_panel.tsx
// Description: Fixed glass detail panel ("context window") placed below its meter and clamped to the window

import { useEffect, useLayoutEffect, type RefObject } from "react";
import type React from "react";
import { createPortal } from "react-dom";
import type { LoadState } from "../../lib/system_load/load_state.js";
import { PANEL_TITLES, type MeterKey } from "../../lib/system_load/meters.js";
import type { LoadSeverity } from "../../lib/system_load/severity.js";
import { CpuPanel, MemoryPanel } from "./cpu_memory_panels.js";
import { DiskPanel, GpuPanel } from "./device_panels.js";
import { NetworkPanel } from "./network_panel.js";
import "../../styles/context_menu.css";
import "../../styles/system_load_panel.css";
import "../../styles/system_load_breakdown.css";

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

function PanelBody({ meterKey, state }: { meterKey: MeterKey; state: LoadState }): React.JSX.Element {
  switch (meterKey) {
    case "cpu": return <CpuPanel state={state} />;
    case "mem": return <MemoryPanel state={state} />;
    case "gpu": return <GpuPanel state={state} />;
    case "dsk": return <DiskPanel state={state} />;
    case "net": return <NetworkPanel state={state} />;
  }
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
      data-pinned={pinned ? "true" : undefined}
      onPointerEnter={onPointerEnter}
      onPointerLeave={onPointerLeave}
    >
      <header className="sysload-panel__header">
        <h2 className="sysload-panel__title">{PANEL_TITLES[meterKey]}</h2>
        <span className="sysload-panel__hint">{pinned ? "Pinned · Esc" : "Click to pin"}</span>
      </header>
      <div className="sysload-panel__body">
        <PanelBody meterKey={meterKey} state={state} />
      </div>
    </div>,
    document.body
  );
}
