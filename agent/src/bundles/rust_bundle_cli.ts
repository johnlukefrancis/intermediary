// Path: agent/src/bundles/rust_bundle_cli.ts
// Description: Run the Rust im_bundle_cli to scan and build bundle zips with progress parsing

import { spawn } from "node:child_process";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { AgentError, isNodeError } from "../util/errors.js";
import { logger } from "../util/logger.js";

interface BundleSelectionPlan {
  includeRoot: boolean;
  topLevelDirs: string[];
  excludedSubdirs: string[];
}

interface GlobalExcludesPlan {
  presets: {
    modelWeights: boolean;
    modelFormats: boolean;
    modelDirs: boolean;
    hfCaches: boolean;
    experimentLogs: boolean;
  };
  extensions: string[];
  patterns: string[];
}

interface BundleGitPlan {
  headSha?: string;
  shortSha?: string;
  branch?: string;
}

interface BundlePlan {
  outputPath: string;
  repoRoot: string;
  repoId: string;
  presetId: string;
  presetName: string;
  selection: BundleSelectionPlan;
  git: BundleGitPlan;
  builtAtIso: string;
  globalExcludes: GlobalExcludesPlan;
}

interface BundleProgressLine {
  type: "progress";
  phase: "scanning" | "zipping";
  files_done: number;
  files_total: number;
  current_file?: string;
  current_bytes_done?: number;
  current_bytes_total?: number;
  bytes_done_total_best_effort?: number;
}

interface BundleDoneLine {
  type: "done";
  bytes_written: number;
  file_count: number;
  scan_ms: number;
  zip_ms: number;
}

export interface BundleProgress {
  phase: "scanning" | "zipping";
  filesDone: number;
  filesTotal: number;
  currentFile?: string;
  currentBytesDone?: number;
  currentBytesTotal?: number;
  bytesDoneTotalBestEffort?: number;
}

export interface BundleResult {
  bytes: number;
  fileCount: number;
  scanMs?: number;
  zipMs?: number;
}

export interface BundleCliOptions {
  outputPath: string;
  repoRoot: string;
  repoId: string;
  presetId: string;
  presetName: string;
  selection: BundleSelectionPlan;
  git: BundleGitPlan;
  builtAtIso: string;
  globalExcludes?: GlobalExcludesPlan;
  onProgress?: (progress: BundleProgress) => void;
}

const CLI_PATH_ENV = "IM_BUNDLE_CLI_PATH";
const PROGRESS_STALL_TIMEOUT_MS = 30_000;

type BundleCliProfile = "release" | "debug";

interface ResolvedCliPath {
  path: string;
  profile: BundleCliProfile;
}

async function resolveCliPath(): Promise<ResolvedCliPath | null> {
  const override = process.env[CLI_PATH_ENV];
  if (override) return { path: override, profile: "release" };

  const candidates: Array<{ path: string; profile: BundleCliProfile }> = [
    { path: path.join(process.cwd(), "target", "release", "im_bundle_cli"), profile: "release" },
    { path: path.join(process.cwd(), "target", "debug", "im_bundle_cli"), profile: "debug" },
  ];

  for (const candidate of candidates) {
    try {
      await fs.access(candidate.path);
      return candidate;
    } catch {
      // Try next candidate.
    }
  }

  return null;
}

function isProgressLine(value: unknown): value is BundleProgressLine {
  if (!value || typeof value !== "object") return false;
  const line = value as Record<string, unknown>;
  const baseValid = (
    line.type === "progress" &&
    (line.phase === "scanning" || line.phase === "zipping") &&
    typeof line.files_done === "number" &&
    typeof line.files_total === "number"
  );
  if (!baseValid) return false;
  if (line.current_file !== undefined && typeof line.current_file !== "string") return false;
  if (line.current_bytes_done !== undefined && typeof line.current_bytes_done !== "number") return false;
  if (line.current_bytes_total !== undefined && typeof line.current_bytes_total !== "number") return false;
  if (line.bytes_done_total_best_effort !== undefined && typeof line.bytes_done_total_best_effort !== "number") {
    return false;
  }
  return true;
}

function isDoneLine(value: unknown): value is BundleDoneLine {
  if (!value || typeof value !== "object") return false;
  const line = value as Record<string, unknown>;
  return (
    line.type === "done" &&
    typeof line.bytes_written === "number" &&
    typeof line.file_count === "number" &&
    typeof line.scan_ms === "number" &&
    typeof line.zip_ms === "number"
  );
}

function buildPlan(options: BundleCliOptions): BundlePlan {
  const git: BundleGitPlan = {};
  if (options.git.headSha !== undefined) {
    git.headSha = options.git.headSha;
  }
  if (options.git.shortSha !== undefined) {
    git.shortSha = options.git.shortSha;
  }
  if (options.git.branch !== undefined) {
    git.branch = options.git.branch;
  }

  return {
    outputPath: options.outputPath,
    repoRoot: options.repoRoot,
    repoId: options.repoId,
    presetId: options.presetId,
    presetName: options.presetName,
    selection: {
      includeRoot: options.selection.includeRoot,
      topLevelDirs: options.selection.topLevelDirs,
      excludedSubdirs: options.selection.excludedSubdirs,
    },
    git,
    builtAtIso: options.builtAtIso,
    globalExcludes: options.globalExcludes ?? {
      presets: {
        modelWeights: true,
        modelFormats: true,
        modelDirs: true,
        hfCaches: true,
        experimentLogs: true,
      },
      extensions: [],
      patterns: [],
    },
  };
}

export async function writeBundleWithRustCli(options: BundleCliOptions): Promise<BundleResult> {
  const resolved = await resolveCliPath();
  if (!resolved) {
    throw new AgentError(
      "BUNDLE_CLI_NOT_FOUND",
      "im_bundle_cli not found; build the Rust CLI or set IM_BUNDLE_CLI_PATH",
      {
        tried: [
          path.join(process.cwd(), "target", "release", "im_bundle_cli"),
          path.join(process.cwd(), "target", "debug", "im_bundle_cli"),
        ],
      }
    );
  }
  const cliPath = resolved.path;

  const planPath = `${options.outputPath}.bundle_plan.json`;
  const plan = buildPlan(options);
  await fs.writeFile(planPath, JSON.stringify(plan), "utf8");

  let bytesWritten: number | null = null;
  let fileCount: number | null = null;
  let scanMs: number | undefined;
  let zipMs: number | undefined;
  let stderr = "";

  try {
    logger.info("Bundle CLI resolved", {
      path: cliPath,
      profile: resolved.profile,
    });

    const child = spawn(cliPath, [planPath], {
      stdio: ["ignore", "pipe", "pipe"],
    });
    let watchdog: NodeJS.Timeout | null = null;
    let lastProgress: { phase: BundleProgress["phase"]; currentFile?: string } | null = null;
    let stallReject: ((err: AgentError) => void) | null = null;
    const stallPromise = new Promise<never>((_, reject) => {
      stallReject = reject;
    });

    const stopWatchdog = (): void => {
      if (watchdog) {
        clearTimeout(watchdog);
        watchdog = null;
      }
    };

    const resetWatchdog = (): void => {
      stopWatchdog();
      watchdog = setTimeout(() => {
        if (!stallReject) return;
        const fileSuffix = lastProgress?.currentFile ? ` (last file: ${lastProgress.currentFile})` : "";
        const error = new AgentError(
          "BUNDLE_CLI_STALLED",
          `Bundle build stalled during ${lastProgress?.phase ?? "unknown"} phase${fileSuffix}`,
          {
            phase: lastProgress?.phase ?? null,
            currentFile: lastProgress?.currentFile ?? null,
          }
        );
        stallReject(error);
        stallReject = null;
        child.kill("SIGKILL");
      }, PROGRESS_STALL_TIMEOUT_MS);
    };

    resetWatchdog();

    let buffer = "";
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk: string) => {
      buffer += chunk;
      let newlineIndex = buffer.indexOf("\n");
      while (newlineIndex >= 0) {
        const line = buffer.slice(0, newlineIndex).trim();
        buffer = buffer.slice(newlineIndex + 1);
        if (line.length > 0) {
          try {
            const parsed = JSON.parse(line) as unknown;
            if (isProgressLine(parsed)) {
              const progress: BundleProgress = {
                phase: parsed.phase,
                filesDone: parsed.files_done,
                filesTotal: parsed.files_total,
              };
              if (parsed.current_file !== undefined) {
                progress.currentFile = parsed.current_file;
              }
              if (parsed.current_bytes_done !== undefined) {
                progress.currentBytesDone = parsed.current_bytes_done;
              }
              if (parsed.current_bytes_total !== undefined) {
                progress.currentBytesTotal = parsed.current_bytes_total;
              }
              if (parsed.bytes_done_total_best_effort !== undefined) {
                progress.bytesDoneTotalBestEffort = parsed.bytes_done_total_best_effort;
              }
              const snapshot: { phase: BundleProgress["phase"]; currentFile?: string } = {
                phase: parsed.phase,
              };
              if (parsed.current_file !== undefined) {
                snapshot.currentFile = parsed.current_file;
              }
              lastProgress = snapshot;
              resetWatchdog();
              options.onProgress?.(progress);
            } else if (isDoneLine(parsed)) {
              bytesWritten = parsed.bytes_written;
              fileCount = parsed.file_count;
              scanMs = parsed.scan_ms;
              zipMs = parsed.zip_ms;
            }
          } catch (err) {
            logger.warn("Failed to parse im_bundle_cli output", {
              line,
              error: err instanceof Error ? err.message : String(err),
            });
          }
        }
        newlineIndex = buffer.indexOf("\n");
      }
    });

    child.stderr.setEncoding("utf8");
    child.stderr.on("data", (chunk: string) => {
      stderr += chunk;
    });

    const exitPromise = new Promise<number>((resolve, reject) => {
      child.on("error", (err) => {
        stopWatchdog();
        stallReject = null;
        reject(err);
      });
      child.on("close", (code) => {
        stopWatchdog();
        stallReject = null;
        resolve(code ?? 1);
      });
    });

    const exitCode: number = await Promise.race([exitPromise, stallPromise]);

    if (exitCode !== 0) {
      const message = stderr.trim() || `im_bundle_cli exited with code ${exitCode}`;
      throw new AgentError("BUNDLE_CLI_FAILED", message, { exitCode });
    }

    const bytes =
      typeof bytesWritten === "number"
        ? bytesWritten
        : await getOutputBytes(options.outputPath);
    const files =
      typeof fileCount === "number"
        ? fileCount
        : 0;

    const result: BundleResult = { bytes, fileCount: files };
    if (scanMs !== undefined) {
      result.scanMs = scanMs;
    }
    if (zipMs !== undefined) {
      result.zipMs = zipMs;
    }
    return result;
  } catch (err) {
    if (isNodeError(err) && err.code === "ENOENT") {
      throw new AgentError("BUNDLE_CLI_NOT_FOUND", "im_bundle_cli not found", { cliPath });
    }
    throw err;
  } finally {
    try {
      await fs.unlink(planPath);
    } catch {
      // Ignore cleanup errors.
    }
  }
}

async function getOutputBytes(outputPath: string): Promise<number> {
  try {
    const stat = await fs.stat(outputPath);
    return stat.size;
  } catch {
    return 0;
  }
}
