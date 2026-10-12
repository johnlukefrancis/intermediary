// Path: app/src/shared/protocol_system_load.ts
// Description: `sample_system_load` Tauri command response schema (mirrors crates/im_system_load/src/snapshot.rs)

import { z } from "zod";

export const CpuLoadSchema = z.object({
  name: z.string().nullable(),
  usagePercent: z.number(),
  corePercents: z.array(z.number()),
  speedMhz: z.number().nullable(),
  baseSpeedMhz: z.number().nullable(),
  processCount: z.number(),
  threadCount: z.number(),
});
export type CpuLoad = z.infer<typeof CpuLoadSchema>;

export const MemoryLoadSchema = z.object({
  totalBytes: z.number(),
  availableBytes: z.number(),
  commitBytes: z.number(),
  commitLimitBytes: z.number(),
  hardFaultsPerSec: z.number(),
  /** Windows' compressed page store; never listed as a process group. */
  compressedBytes: z.number(),
});
export type MemoryLoad = z.infer<typeof MemoryLoadSchema>;

export const GpuEngineLoadSchema = z.object({
  kind: z.string(),
  usagePercent: z.number(),
});
export type GpuEngineLoad = z.infer<typeof GpuEngineLoadSchema>;

export const GpuLoadSchema = z.object({
  name: z.string(),
  usagePercent: z.number(),
  engines: z.array(GpuEngineLoadSchema),
  dedicatedUsedBytes: z.number(),
  dedicatedTotalBytes: z.number(),
  sharedUsedBytes: z.number(),
  sharedTotalBytes: z.number(),
});
export type GpuLoad = z.infer<typeof GpuLoadSchema>;

export const DiskLoadSchema = z.object({
  name: z.string(),
  activePercent: z.number(),
  readBytesPerSec: z.number(),
  writeBytesPerSec: z.number(),
});
export type DiskLoad = z.infer<typeof DiskLoadSchema>;

export const NetworkLoadSchema = z.object({
  name: z.string(),
  receivedBytesPerSec: z.number(),
  sentBytesPerSec: z.number(),
  linkBitsPerSec: z.number(),
});
export type NetworkLoad = z.infer<typeof NetworkLoadSchema>;

/** All processes sharing one image name; `label` is the executable description when known. */
export const ProcessGroupLoadSchema = z.object({
  name: z.string(),
  label: z.string(),
  count: z.number(),
  cpuPercent: z.number(),
  memoryBytes: z.number(),
  ioBytesPerSec: z.number(),
  gpuPercent: z.number(),
  gpuMemoryBytes: z.number(),
});
export type ProcessGroupLoad = z.infer<typeof ProcessGroupLoadSchema>;

/** One machine-wide sample; `intervalMs` is 0 (and every rate 0) on the sampler's first sample. */
export const SystemLoadSnapshotSchema = z.object({
  sampledAtMs: z.number(),
  intervalMs: z.number(),
  cpu: CpuLoadSchema,
  memory: MemoryLoadSchema,
  gpus: z.array(GpuLoadSchema),
  disks: z.array(DiskLoadSchema),
  networks: z.array(NetworkLoadSchema),
  processes: z.array(ProcessGroupLoadSchema),
});
export type SystemLoadSnapshot = z.infer<typeof SystemLoadSnapshotSchema>;
