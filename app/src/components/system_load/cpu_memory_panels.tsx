// Path: app/src/components/system_load/cpu_memory_panels.tsx
// Description: CPU and MEMORY detail-panel bodies: readout, history, breakdown and top processes

import type React from "react";
import {
  formatCount, formatGhz, formatGigabytes, formatMemory, formatPercent, formatPercentFine,
} from "../../lib/system_load/format.js";
import type { LoadState } from "../../lib/system_load/load_state.js";
import { commitPercent, memoryInUsePercent, memorySeverity, percentSeverity } from "../../lib/system_load/severity.js";
import { LoadSparkline } from "./load_sparkline.js";
import { BarRow, PanelReadout, PanelSection, StatGrid } from "./panel_parts.js";
import { ProcessList } from "./process_list.js";

export function CpuPanel({ state }: { state: LoadState }): React.JSX.Element {
  const { cpu } = state.snapshot;
  const speed = cpu.speedMhz !== null ? formatGhz(cpu.speedMhz) : "—";
  return (
    <>
      <PanelReadout value={formatPercent(cpu.usagePercent)} aside={speed} subtitle={cpu.name ?? "Processor"} />
      <LoadSparkline series={[{ values: state.history.map((s) => s.cpu), tone: "primary" }]} max={100} scaleLabel="100%" />
      <StatGrid stats={[
        ["Speed", speed],
        ["Base", cpu.baseSpeedMhz !== null ? formatGhz(cpu.baseSpeedMhz) : "—"],
        ["Processes", formatCount(cpu.processCount)],
        ["Threads", formatCount(cpu.threadCount)],
      ]} />
      <PanelSection title={`Logical processors · ${cpu.corePercents.length}`}>
        <div className="sysload-cores" role="img" aria-label={`Per-core load, ${cpu.corePercents.length} logical processors`}>
          {cpu.corePercents.map((percent, index) => (
            <span
              key={index}
              className="sysload-core"
              data-severity={percentSeverity(percent)}
              title={`CPU ${index}: ${formatPercent(percent)}`}
            >
              <span className="sysload-core__fill" style={{ height: `${Math.min(100, Math.max(0, percent))}%` }} />
            </span>
          ))}
        </div>
      </PanelSection>
      <ProcessList processes={state.snapshot.processes} column={{ measure: (p) => p.cpuPercent, format: formatPercentFine }} />
    </>
  );
}

export function MemoryPanel({ state }: { state: LoadState }): React.JSX.Element {
  const { memory } = state.snapshot;
  const inUseBytes = memory.totalBytes - memory.availableBytes;
  const inUse = memoryInUsePercent(memory);
  const commit = commitPercent(memory);
  const severity = memorySeverity(memory, state.pagingStreak);
  const subtitle = severity === "paging"
    ? `Paging · ${formatCount(memory.hardFaultsPerSec)} hard faults/s`
    : `${formatPercent(inUse)} in use · ${formatMemory(memory.availableBytes)} available`;
  return (
    <>
      <PanelReadout
        value={`${formatGigabytes(inUseBytes)} GB`}
        aside={`of ${formatGigabytes(memory.totalBytes)} GB`}
        subtitle={subtitle}
      />
      <LoadSparkline series={[{ values: state.history.map((s) => s.mem), tone: "primary" }]} max={100} scaleLabel="100%" />
      <div className="sysload-bars">
        <BarRow
          label="In use"
          value={`${formatGigabytes(inUseBytes)} / ${formatGigabytes(memory.totalBytes)} GB`}
          percent={inUse}
          severity={severity === "paging" ? "hot" : severity}
        />
        <BarRow
          label="Available"
          value={formatMemory(memory.availableBytes)}
          percent={memory.totalBytes > 0 ? (memory.availableBytes / memory.totalBytes) * 100 : 0}
          severity="calm"
        />
        <BarRow
          label="Commit"
          value={`${formatGigabytes(memory.commitBytes)} / ${formatGigabytes(memory.commitLimitBytes)} GB`}
          percent={commit}
          severity={commit >= 90 ? "hot" : commit >= 75 ? "busy" : "calm"}
        />
      </div>
      <StatGrid stats={[
        ["Hard faults/s", formatCount(memory.hardFaultsPerSec)],
        ["Commit", formatPercent(commit)],
      ]} />
      <ProcessList processes={state.snapshot.processes} column={{ measure: (p) => p.memoryBytes, format: formatMemory }} />
    </>
  );
}
