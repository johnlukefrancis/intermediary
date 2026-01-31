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
}

interface BundleProgressLine {
  type: "progress";
  phase: "scanning" | "zipping";
  files_done: number;
  files_total: number;
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
  onProgress?: (progress: BundleProgress) => void;
}

const CLI_PATH_ENV = "IM_BUNDLE_CLI_PATH";

async function resolveCliPath(): Promise<string | null> {
  const override = process.env[CLI_PATH_ENV];
  if (override) return override;

  const candidates = [
    path.join(process.cwd(), "target", "release", "im_bundle_cli"),
    path.join(process.cwd(), "target", "debug", "im_bundle_cli"),
  ];

  for (const candidate of candidates) {
    try {
      await fs.access(candidate);
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
  return (
    line.type === "progress" &&
    (line.phase === "scanning" || line.phase === "zipping") &&
    typeof line.files_done === "number" &&
    typeof line.files_total === "number"
  );
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
  };
}

export async function writeBundleWithRustCli(options: BundleCliOptions): Promise<BundleResult> {
  const cliPath = await resolveCliPath();
  if (!cliPath) {
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

  const planPath = `${options.outputPath}.bundle_plan.json`;
  const plan = buildPlan(options);
  await fs.writeFile(planPath, JSON.stringify(plan), "utf8");

  let bytesWritten: number | null = null;
  let fileCount: number | null = null;
  let scanMs: number | undefined;
  let zipMs: number | undefined;
  let stderr = "";

  try {
    const child = spawn(cliPath, [planPath], {
      stdio: ["ignore", "pipe", "pipe"],
    });

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
              options.onProgress?.({
                phase: parsed.phase,
                filesDone: parsed.files_done,
                filesTotal: parsed.files_total,
              });
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

    const exitCode: number = await new Promise((resolve, reject) => {
      child.on("error", (err) => {
        reject(err);
      });
      child.on("close", (code) => {
        resolve(code ?? 1);
      });
    });

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
