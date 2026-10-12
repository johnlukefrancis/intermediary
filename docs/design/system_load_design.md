# System Load Design
Updated on: 2026-10-12
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

---

## Problem

JL runs Unreal, Blender, several WSL agents, Chrome and games on one machine whose RAM is
tight. Knowing whether the machine is about to page, or which program is eating the CPU, GPU
or disk, currently means opening Task Manager. Intermediary is always open; its status strip
(connection LED and OPTIONS) should answer those questions at a glance.

## Goals

- Five always-visible meters in the status strip: CPU, MEM, GPU, DSK, NET. The state of each
  should read in a fraction of a second: calm, busy, hot, or (for memory) paging.
- Hovering a meter opens a detail panel with a 60-second history, the subsystem's breakdown
  and the programs using the most of that resource, named and grouped like Task Manager
  ("Google Chrome ×28").
- Numbers agree with Task Manager closely enough that JL never has to open it to check.
- The strip stays minimal and reads as part of the instrument deck.

## Non-goals

- Ending tasks, changing priorities or any other process control.
- Per-process network usage, which Windows exposes only through ETW tracing.
- A breakdown of the processes inside the WSL VM (`VmmemWSL`). The WSL agent could report
  them later.
- macOS sampling. On non-Windows hosts the command returns an error and the strip shows no
  meters.

## Sources (Windows)

| Signal | Source | Task Manager parity |
|---|---|---|
| Process list, CPU time, private working set, I/O bytes | One `NtQuerySystemInformation(SystemProcessInformation)` call per sample | Memory column is the private working set; CPU is the share of all logical processors |
| CPU total, per-core, speed | PDH `\Processor Information(*)\% Processor Utility`, `Processor Frequency`, `% Processor Performance` | Utility is the counter Task Manager uses; clamp to 100 |
| Physical memory, commit | `GlobalMemoryStatusEx` | In use = total − available |
| Paging | PDH `\Memory\Pages Input/sec` (hard-fault page reads) | Shown as a rate |
| GPU utilisation | PDH `\GPU Engine(*)\Utilization Percentage` | Sum per engine across processes; adapter = busiest engine; process = its busiest engine |
| GPU memory | PDH `\GPU Adapter Memory(*)\Dedicated Usage` / `Shared Usage`, `\GPU Process Memory(*)\Local Usage`; DXGI adapter name and totals | Adapter dedicated and shared memory as Task Manager; per-process VRAM is resident local memory, because per-process Dedicated Usage over-reports for `dwm` |
| Disk | PDH `\PhysicalDisk(*)\% Idle Time`, `Disk Read Bytes/sec`, `Disk Write Bytes/sec` | Active time = 100 − idle |
| Network | PDH `\Network Interface(*)\Bytes Received/sec`, `Bytes Sent/sec`, `Current Bandwidth` | Utilisation = bits/s ÷ link speed |
| Friendly names | `FileDescription` from the executable's version resource, cached per image name | "Antimalware Service Executable" instead of `MsMpEng` |

Counters are added with `PdhAddEnglishCounterW` so they work on any Windows display language.

## Ownership

- `crates/im_system_load` is the only owner of OS telemetry. It has no Tauri dependency, so its
  Windows code can be checked from WSL against `x86_64-pc-windows-msvc` and its parsing tested
  on Linux. `Sampler::new()` opens the PDH query and reads the adapter list once;
  `Sampler::sample()` returns one `SystemLoadSnapshot`.
- `src-tauri/src/lib/system_load.rs` holds one `Sampler` in managed state behind a mutex and exposes
  the `sample_system_load` command, which runs the sample on a blocking thread.
- `app/src/shared/protocol_system_load.ts` is the wire contract (zod) mirroring
  `crates/im_system_load/src/snapshot.rs`.
- The frontend hook owns the cadence. It issues the next request one second after the previous one
  resolves, and only while the document is visible, so a minimised Intermediary costs nothing and
  requests never overlap. Rates are computed over the real interval between samples.

## Wire contract

`SystemLoadSnapshot` (camelCase JSON): `sampledAtMs`, `intervalMs` (0 on the first sample, when
every rate is 0), `cpu`, `memory`, `gpus[]`, `disks[]`, `networks[]`, `processes[]`.
`processes` holds process groups (all processes sharing an image name), taking the union of the
top 8 groups by CPU, memory, I/O, GPU and GPU memory; the frontend sorts each panel's list
itself. The System Idle Process is excluded.

## Strip layout

```
● CONNECTED  · error ticker…        ┃ CPU ▮▮▮▯▯▯▯▯▯▯ 22% │ MEM ▮▮▮▮▮▮▯▯▯▯ 59% │ GPU ▮▯▯▯▯▯ 10% │ DSK ▯ 1% │ NET ↓0.2 ↑0.1 ┃  OPTIONS
```

- Left: connection LED and label (unchanged states), then the error ticker.
- Right: the meter cluster sits in a recessed instrument window (darker inset, hairline border,
  deck radius) beside OPTIONS. Meters are separated by hairlines.
- Each meter: a tiny wide-tracked label, a segmented LED bar and a tabular value. Lit segments
  ramp through the activity tokens (low → mid → high → hot) by position, so a full bar ends
  red; unlit segments use `--color-activity-idle`. A peak-hold segment holds the 5-second peak.
- NET has no natural percentage. Its bar is log-scaled throughput (1 KB/s … 1 GB/s) and its
  value shows ↓/↑ rates.
- Narrow widths (handset) drop values, then shorten bars; labels and bars always remain.

## Severity

| Level | CPU / GPU / DSK | MEM | Visual |
|---|---|---|---|
| calm | < 60% | < 75% | muted, no glow |
| busy | 60–85% | 75–90% | value text warms |
| hot | ≥ 85% | ≥ 90% or commit ≥ 90% of limit | lit segments glow, value turns hot |
| paging | — | in use ≥ 85% and hard faults ≥ 100/s for 3 consecutive samples | label reads PAGING, slow pulse |

Pulses stop under reduced motion and while `.app[data-motion="paused"]`.

## Detail panel

Opens 120 ms after the pointer rests on a meter and stays open while the pointer is over the
meter or the panel. Moving to another meter switches immediately. Clicking pins the panel until
an outside click or Escape. Meters are buttons, so focus opens the panel too. The panel is a glass
surface below the meter, clamped to the window, `:: CPU` style header.

| Panel | Contents |
|---|---|
| CPU | usage, effective speed, processes/threads, 60 s history, one micro-bar per logical core, top 6 by CPU |
| MEMORY | in use / total, available, committed / limit, hard faults per second, 60 s history, top 6 by private working set |
| GPU | per adapter: name, usage, engine bars (3D, Copy, Video Decode, …), dedicated and shared memory bars; 60 s history; top 6 by GPU with VRAM |
| DISK | per physical disk: active time, read and write rate; 60 s history of the busiest disk; top 6 by I/O |
| NETWORK | per active interface: ↓/↑ rate, link speed; 60 s history of receive and send |

Process rows: friendly label, `×N` group count, value, and a micro-bar scaled to the panel's
largest row.

## Accepted boundaries

- The process I/O rate counts all read and write transfers (`ReadTransferCount` +
  `WriteTransferCount`), so it can include some network and device I/O, unlike Task Manager's ETW
  disk column.
- Groups are formed by image name, not by Task Manager's app tree.
- Network interfaces are reported as PDH lists them, including virtual adapters. The panel shows
  the active ones.

## Acceptance

- The installed Windows app shows live meters that track Task Manager for CPU, memory, GPU, disk
  and network within normal sampling noise.
- Hovering each meter shows its panel, and the top-process lists name the same programs as Task
  Manager's sorted columns.
- Minimising Intermediary stops sampling.
- JL accepts the look and feel.
