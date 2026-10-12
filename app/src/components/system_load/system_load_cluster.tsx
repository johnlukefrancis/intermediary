// Path: app/src/components/system_load/system_load_cluster.tsx
// Description: Status-strip instrument window of CPU/MEM/GPU/DSK/NET meters; owns the 1 Hz load state and its panel

import { memo, useCallback, useMemo, useRef } from "react";
import type React from "react";
import { useSystemLoad } from "../../hooks/use_system_load.js";
import { buildMeters, type MeterKey } from "../../lib/system_load/meters.js";
import { LoadMeter } from "./load_meter.js";
import { LoadPanel } from "./load_panel.js";
import { useMeterPanel } from "./use_meter_panel.js";
import "../../styles/system_load.css";

export const SystemLoadCluster = memo(function SystemLoadCluster({ documentHidden }: {
  documentHidden: boolean;
}): React.JSX.Element | null {
  const state = useSystemLoad(documentHidden);
  const clusterRef = useRef<HTMLDivElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const anchors = useRef(new Map<MeterKey, HTMLButtonElement>());
  const insideRefs = useMemo(() => [clusterRef, panelRef], []);
  const panel = useMeterPanel(insideRefs);
  const meters = useMemo(() => (state ? buildMeters(state) : []), [state]);

  const registerAnchor = useCallback((key: MeterKey, node: HTMLButtonElement | null) => {
    if (node) anchors.current.set(key, node);
    else anchors.current.delete(key);
  }, []);

  if (!state || meters.length === 0) return null;
  const openReading = meters.find((meter) => meter.key === panel.openKey);

  return (
    <div ref={clusterRef} className="sysload-window" role="group" aria-label="System load">
      {meters.map((reading) => (
        <LoadMeter
          key={reading.key}
          reading={reading}
          open={panel.openKey === reading.key}
          registerAnchor={registerAnchor}
          onEnter={panel.enterMeter}
          onFocus={panel.focusMeter}
          onClick={panel.clickMeter}
          onLeave={panel.leave}
          onBlur={panel.blurMeter}
        />
      ))}
      {openReading ? (
        <LoadPanel
          meterKey={openReading.key}
          anchor={anchors.current.get(openReading.key) ?? null}
          state={state}
          severity={openReading.severity}
          pinned={panel.pinned}
          onPointerEnter={panel.enterPanel}
          onPointerLeave={panel.leave}
          panelRef={panelRef}
        />
      ) : null}
    </div>
  );
});
