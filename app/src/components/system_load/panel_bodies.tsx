// Path: app/src/components/system_load/panel_bodies.tsx
// Description: Lean per-meter panel bodies: how much is used, a verdict, a 60 s trend and the top programs

import type React from "react";
import {
  formatBits, formatGigabytes, formatMemory, formatPercent, formatPercentFine, formatRate,
} from "../../lib/system_load/format.js";
import { busiestInterface, type LoadState } from "../../lib/system_load/load_state.js";
import type { MeterKey } from "../../lib/system_load/meters.js";
import { commitPercent, type LoadSeverity } from "../../lib/system_load/severity.js";
import type { GpuLoad } from "../../shared/protocol_system_load.js";
import { LoadSparkline } from "./load_sparkline.js";
import { PanelHeadline } from "./panel_parts.js";
import { ProcessList } from "./process_list.js";

const MIN_NET_SCALE_BYTES = 128 * 1024;
const COMPRESSED_NOTE_BYTES = 256 * 1024 * 1024;

interface BodyProps {
  state: LoadState;
  severity: LoadSeverity;
}

/** Ties go to the larger dedicated memory, so an idle iGPU never outranks the discrete card. */
function busiestGpu(gpus: readonly GpuLoad[]): GpuLoad | undefined {
  return gpus.reduce<GpuLoad | undefined>((best, gpu) => {
    if (!best || gpu.usagePercent > best.usagePercent) return gpu;
    if (gpu.usagePercent === best.usagePercent && gpu.dedicatedTotalBytes > best.dedicatedTotalBytes) return gpu;
    return best;
  }, undefined);
}

/** PDH names physical disks "1 E:" or "0 C: D:"; the drive letters are what JL recognises. */
function diskLabel(name: string): string {
  return name.replace(/^\d+\s+/, "") || name;
}

function percentTrend(state: LoadState, key: "cpu" | "gpu" | "dsk" | "mem"): React.JSX.Element {
  return <LoadSparkline series={[{ values: state.history.map((s) => s[key]), tone: "primary" }]} max={100} />;
}

function CpuBody({ state, severity }: BodyProps): React.JSX.Element {
  return (
    <>
      <PanelHeadline meterKey="cpu" severity={severity} value={formatPercent(state.snapshot.cpu.usagePercent)} />
      {percentTrend(state, "cpu")}
      <ProcessList processes={state.snapshot.processes} column={{ measure: (p) => p.cpuPercent, format: formatPercentFine }} />
    </>
  );
}

function MemoryBody({ state, severity }: BodyProps): React.JSX.Element {
  const { memory } = state.snapshot;
  return (
    <>
      <PanelHeadline
        meterKey="mem"
        severity={severity}
        value={`${formatGigabytes(memory.totalBytes - memory.availableBytes)} / ${formatGigabytes(memory.totalBytes)} GB`}
        aside={`${formatGigabytes(memory.availableBytes)} GB free`}
        note={memory.compressedBytes >= COMPRESSED_NOTE_BYTES ? `${formatGigabytes(memory.compressedBytes)} GB compressed by Windows` : null}
        warning={commitPercent(memory) >= 90 ? "Commit limit nearly reached" : null}
      />
      {percentTrend(state, "mem")}
      <ProcessList processes={state.snapshot.processes} column={{ measure: (p) => p.memoryBytes, format: formatMemory }} />
    </>
  );
}

function GpuBody({ state, severity }: BodyProps): React.JSX.Element {
  const gpu = busiestGpu(state.snapshot.gpus);
  return (
    <>
      <PanelHeadline
        meterKey="gpu"
        severity={severity}
        value={formatPercent(gpu?.usagePercent ?? 0)}
        {...(gpu ? { aside: `VRAM ${formatGigabytes(gpu.dedicatedUsedBytes)} / ${formatGigabytes(gpu.dedicatedTotalBytes)} GB` } : {})}
      />
      {percentTrend(state, "gpu")}
      <ProcessList
        processes={state.snapshot.processes}
        column={{
          measure: (p) => p.gpuPercent,
          format: formatPercentFine,
          secondary: (p) => (p.gpuMemoryBytes > 0 ? formatMemory(p.gpuMemoryBytes) : null),
        }}
      />
    </>
  );
}

function DiskBody({ state, severity }: BodyProps): React.JSX.Element {
  const { disks } = state.snapshot;
  const busiest = disks.reduce<(typeof disks)[number] | undefined>(
    (best, disk) => (!best || disk.activePercent > best.activePercent ? disk : best), undefined
  );
  const read = disks.reduce((sum, disk) => sum + disk.readBytesPerSec, 0);
  const write = disks.reduce((sum, disk) => sum + disk.writeBytesPerSec, 0);
  return (
    <>
      <PanelHeadline
        meterKey="dsk"
        severity={severity}
        value={formatPercent(busiest?.activePercent ?? 0)}
        aside={`${busiest ? `${diskLabel(busiest.name)} · ` : ""}R ${formatRate(read)} · W ${formatRate(write)}`}
      />
      {percentTrend(state, "dsk")}
      <ProcessList processes={state.snapshot.processes} column={{ measure: (p) => p.ioBytesPerSec, format: formatRate }} />
    </>
  );
}

function NetworkBody({ state, severity }: BodyProps): React.JSX.Element {
  const net = busiestInterface(state.snapshot.networks);
  const down = state.history.map((s) => s.netDown);
  const up = state.history.map((s) => s.netUp);
  return (
    <>
      <PanelHeadline
        meterKey="net"
        severity={severity}
        value={`↓ ${formatBits((net?.receivedBytesPerSec ?? 0) * 8)}`}
        aside={`↑ ${formatBits((net?.sentBytesPerSec ?? 0) * 8)}`}
      />
      <LoadSparkline
        series={[{ values: down, tone: "primary" }, { values: up, tone: "secondary" }]}
        max={Math.max(MIN_NET_SCALE_BYTES, ...down, ...up)}
      />
    </>
  );
}

export function PanelBody({ meterKey, ...props }: BodyProps & { meterKey: MeterKey }): React.JSX.Element {
  switch (meterKey) {
    case "cpu": return <CpuBody {...props} />;
    case "mem": return <MemoryBody {...props} />;
    case "gpu": return <GpuBody {...props} />;
    case "dsk": return <DiskBody {...props} />;
    case "net": return <NetworkBody {...props} />;
  }
}
