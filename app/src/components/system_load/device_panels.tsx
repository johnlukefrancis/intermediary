// Path: app/src/components/system_load/device_panels.tsx
// Description: GPU and DISK detail-panel bodies: per-adapter engines and memory, per-disk activity, top processes

import type React from "react";
import {
  formatGigabytes, formatMemory, formatPercent, formatPercentFine, formatRate,
} from "../../lib/system_load/format.js";
import type { LoadState } from "../../lib/system_load/load_state.js";
import type { DiskLoad, GpuLoad } from "../../shared/protocol_system_load.js";
import { LoadSparkline } from "./load_sparkline.js";
import { BarRow, PanelEmpty, PanelReadout, PanelSection } from "./panel_parts.js";
import { ProcessList } from "./process_list.js";

const ENGINE_LIMIT = 6;
const CORE_ENGINES = new Set(["3d", "copy", "videoencode", "videodecode"]);
const BUSY_ENGINE_PERCENT = 1;

/** "Video Decode 1", "VideoDecode" and "Video_Decode" all key as "videodecode". */
function engineFamily(kind: string): string {
  return kind.toLowerCase().replace(/[\s_]+/g, "").replace(/\d+$/, "");
}

function engineLabel(kind: string): string {
  return kind.replace(/_/g, " ").replace(/([a-z])([A-Z])/g, "$1 $2");
}

function busiest<T>(items: readonly T[], measure: (item: T) => number): T | undefined {
  return items.reduce<T | undefined>((best, item) => (!best || measure(item) > measure(best) ? item : best), undefined);
}

function share(used: number, total: number): number {
  return total > 0 ? (used / total) * 100 : 0;
}

function AdapterSection({ gpu }: { gpu: GpuLoad }): React.JSX.Element {
  const engines = gpu.engines
    .filter((engine) => CORE_ENGINES.has(engineFamily(engine.kind)) || engine.usagePercent >= BUSY_ENGINE_PERCENT)
    .sort((a, b) => b.usagePercent - a.usagePercent)
    .slice(0, ENGINE_LIMIT);
  return (
    <PanelSection title={`${gpu.name} · ${formatPercent(gpu.usagePercent)}`}>
      <div className="sysload-bars">
        {engines.map((engine) => (
          <BarRow key={engine.kind} label={engineLabel(engine.kind)} value={formatPercent(engine.usagePercent)} percent={engine.usagePercent} />
        ))}
        <BarRow
          label="Dedicated"
          value={`${formatGigabytes(gpu.dedicatedUsedBytes)} / ${formatGigabytes(gpu.dedicatedTotalBytes)} GB`}
          percent={share(gpu.dedicatedUsedBytes, gpu.dedicatedTotalBytes)}
          title="Dedicated GPU memory"
        />
        <BarRow
          label="Shared"
          value={`${formatGigabytes(gpu.sharedUsedBytes)} / ${formatGigabytes(gpu.sharedTotalBytes)} GB`}
          percent={share(gpu.sharedUsedBytes, gpu.sharedTotalBytes)}
          title="Shared GPU memory"
        />
      </div>
    </PanelSection>
  );
}

export function GpuPanel({ state }: { state: LoadState }): React.JSX.Element {
  const { gpus, processes } = state.snapshot;
  const top = busiest(gpus, (gpu) => gpu.usagePercent);
  return (
    <>
      <PanelReadout
        value={formatPercent(top?.usagePercent ?? 0)}
        {...(top ? { aside: `${formatGigabytes(top.dedicatedUsedBytes)} / ${formatGigabytes(top.dedicatedTotalBytes)} GB VRAM` } : {})}
        subtitle={top?.name ?? "No GPU adapter reported"}
      />
      <LoadSparkline series={[{ values: state.history.map((s) => s.gpu), tone: "primary" }]} max={100} scaleLabel="100%" />
      {gpus.length === 0 ? <PanelEmpty>No GPU adapters reported</PanelEmpty> : gpus.map((gpu, index) => (
        <AdapterSection key={`${gpu.name}-${index}`} gpu={gpu} />
      ))}
      <ProcessList
        processes={processes}
        column={{
          measure: (p) => p.gpuPercent,
          format: formatPercentFine,
          secondary: (p) => (p.gpuMemoryBytes > 0 ? formatMemory(p.gpuMemoryBytes) : null),
        }}
      />
    </>
  );
}

function DiskRow({ disk }: { disk: DiskLoad }): React.JSX.Element {
  return (
    <div className="sysload-disk">
      <BarRow label={disk.name} value={formatPercent(disk.activePercent)} percent={disk.activePercent} title={`${disk.name} active time`} />
      <div className="sysload-disk__rates">
        <span>R {formatRate(disk.readBytesPerSec)}</span>
        <span>W {formatRate(disk.writeBytesPerSec)}</span>
      </div>
    </div>
  );
}

export function DiskPanel({ state }: { state: LoadState }): React.JSX.Element {
  const { disks, processes } = state.snapshot;
  const top = busiest(disks, (disk) => disk.activePercent);
  return (
    <>
      <PanelReadout
        value={formatPercent(top?.activePercent ?? 0)}
        {...(top ? { aside: `R ${formatRate(top.readBytesPerSec)} · W ${formatRate(top.writeBytesPerSec)}` } : {})}
        subtitle={top ? `Busiest · ${top.name}` : "No physical disks reported"}
      />
      <LoadSparkline series={[{ values: state.history.map((s) => s.dsk), tone: "primary" }]} max={100} scaleLabel="100%" />
      <PanelSection title="Physical disks · active time">
        {disks.length === 0 ? <PanelEmpty>No physical disks reported</PanelEmpty> : (
          <div className="sysload-bars">{disks.map((disk) => <DiskRow key={disk.name} disk={disk} />)}</div>
        )}
      </PanelSection>
      <ProcessList processes={processes} column={{ measure: (p) => p.ioBytesPerSec, format: formatRate }} />
    </>
  );
}
