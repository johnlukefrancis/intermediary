// Path: app/src/components/system_load/network_panel.tsx
// Description: NETWORK detail-panel body: receive/send history and per-interface rates with link speed

import type React from "react";
import { formatBits, formatLinkSpeed, formatPercent } from "../../lib/system_load/format.js";
import { busiestInterface, networkUtilPercent, type LoadState } from "../../lib/system_load/load_state.js";
import type { NetworkLoad } from "../../shared/protocol_system_load.js";
import { LoadSparkline } from "./load_sparkline.js";
import { MicroBar, PanelEmpty, PanelReadout, PanelSection } from "./panel_parts.js";
import { percentSeverity } from "../../lib/system_load/severity.js";

const MIN_SCALE_BYTES = 128 * 1024;

function InterfaceRow({ net }: { net: NetworkLoad }): React.JSX.Element {
  const util = networkUtilPercent(net);
  return (
    <div className="sysload-iface" title={net.name}>
      <div className="sysload-iface__head">
        <span className="sysload-iface__name">{net.name}</span>
        <span className="sysload-iface__link">{net.linkBitsPerSec > 0 ? formatLinkSpeed(net.linkBitsPerSec) : "—"}</span>
      </div>
      <div className="sysload-iface__rates">
        <span className="sysload-iface__down">↓ {formatBits(net.receivedBytesPerSec * 8)}</span>
        <span className="sysload-iface__up">↑ {formatBits(net.sentBytesPerSec * 8)}</span>
        <MicroBar percent={util} severity={percentSeverity(util)} />
        <span className="sysload-iface__util">{formatPercent(util)}</span>
      </div>
    </div>
  );
}

export function NetworkPanel({ state }: { state: LoadState }): React.JSX.Element {
  const { networks } = state.snapshot;
  const top = busiestInterface(networks);
  const active = networks
    .filter((net) => net.receivedBytesPerSec + net.sentBytesPerSec > 0)
    .sort((a, b) => b.receivedBytesPerSec + b.sentBytesPerSec - (a.receivedBytesPerSec + a.sentBytesPerSec));
  const down = state.history.map((s) => s.netDown);
  const up = state.history.map((s) => s.netUp);
  const scale = Math.max(MIN_SCALE_BYTES, ...down, ...up);
  return (
    <>
      <PanelReadout
        value={`↓ ${formatBits((top?.receivedBytesPerSec ?? 0) * 8)}`}
        aside={`↑ ${formatBits((top?.sentBytesPerSec ?? 0) * 8)}`}
        subtitle={top && active.length > 0 ? top.name : "No traffic"}
      />
      <LoadSparkline
        series={[{ values: down, tone: "primary" }, { values: up, tone: "secondary" }]}
        max={scale}
        scaleLabel={formatBits(scale * 8)}
      />
      <div className="sysload-legend" aria-hidden="true">
        <span data-tone="primary">Receive</span>
        <span data-tone="secondary">Send</span>
      </div>
      <PanelSection title={`Active interfaces · ${active.length}`}>
        {active.length === 0
          ? <PanelEmpty>Idle on {networks.length} interface{networks.length === 1 ? "" : "s"}</PanelEmpty>
          : active.map((net) => <InterfaceRow key={net.name} net={net} />)}
      </PanelSection>
    </>
  );
}
