// Path: agent/src/bundles/rust_zip_cli.ts
// Description: Run the Rust im_zip_cli to build zip bundles with progress parsing

import { spawn } from "node:child_process";
import * as fs from "node:fs/promises";
import * as path from "node:path";
import { AgentError, isNodeError } from "../util/errors.js";
import { logger } from "../util/logger.js";
import type { BundleEntry } from "./bundle_scan.js";

interface RustZipPlanEntry {
  sourcePath: string;
  archivePath: string;
  sizeBytes?: number;
}

interface RustZipPlan {
  outputPath: string;
  entries: RustZipPlanEntry[];
  manifestJson?: string;
}

interface RustZipProgressLine {
  type: "progress";
  files_done: number;
  files_total: number;
  phase: string;
}

interface RustZipDoneLine {
  type: "done";
  bytes_written: number;
  file_count: number;
}

export interface RustZipProgress {
  filesDone: number;
  filesTotal: number;
}

export interface RustZipResult {
  bytes: number;
  fileCount: number;
}

export interface RustZipOptions {
  outputPath: string;
  entries: BundleEntry[];
  manifestJson?: string;
  onProgress?: (progress: RustZipProgress) => void;
}

const CLI_PATH_ENV = "IM_ZIP_CLI_PATH";

async function resolveCliPath(): Promise<string | null> {
  const override = process.env[CLI_PATH_ENV];
  if (override) return override;

  const candidates = [
    path.join(process.cwd(), "target", "release", "im_zip_cli"),
    path.join(process.cwd(), "target", "debug", "im_zip_cli"),
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

function isProgressLine(value: unknown): value is RustZipProgressLine {
  if (!value || typeof value !== "object") return false;
  const line = value as Record<string, unknown>;
  return (
    line.type === "progress" &&
    typeof line.files_done === "number" &&
    typeof line.files_total === "number"
  );
}

function isDoneLine(value: unknown): value is RustZipDoneLine {
  if (!value || typeof value !== "object") return false;
  const line = value as Record<string, unknown>;
  return (
    line.type === "done" &&
    typeof line.bytes_written === "number" &&
    typeof line.file_count === "number"
  );
}

function buildPlan(options: RustZipOptions): RustZipPlan {
  const plan: RustZipPlan = {
    outputPath: options.outputPath,
    entries: options.entries.map((entry) => ({
      sourcePath: entry.sourcePath,
      archivePath: entry.archivePath,
      sizeBytes: entry.size,
    })),
  };
  if (options.manifestJson !== undefined) {
    plan.manifestJson = options.manifestJson;
  }
  return plan;
}

export async function writeZipWithRustCli(options: RustZipOptions): Promise<RustZipResult> {
  const cliPath = await resolveCliPath();
  if (!cliPath) {
    throw new AgentError(
      "ZIP_CLI_NOT_FOUND",
      "im_zip_cli not found; build the Rust CLI or set IM_ZIP_CLI_PATH",
      {
        tried: [
          path.join(process.cwd(), "target", "release", "im_zip_cli"),
          path.join(process.cwd(), "target", "debug", "im_zip_cli"),
        ],
      }
    );
  }

  const planPath = `${options.outputPath}.plan.json`;
  const plan = buildPlan(options);
  await fs.writeFile(planPath, JSON.stringify(plan), "utf8");

  let bytesWritten: number | null = null;
  let fileCount: number | null = null;
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
                filesDone: parsed.files_done,
                filesTotal: parsed.files_total,
              });
            } else if (isDoneLine(parsed)) {
              bytesWritten = parsed.bytes_written;
              fileCount = parsed.file_count;
            }
          } catch (err) {
            logger.warn("Failed to parse im_zip_cli output", {
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
      const message = stderr.trim() || `im_zip_cli exited with code ${exitCode}`;
      throw new AgentError("ZIP_CLI_FAILED", message, { exitCode });
    }

    return await resolveZipResult(bytesWritten, fileCount, options);
  } catch (err) {
    if (isNodeError(err) && err.code === "ENOENT") {
      throw new AgentError("ZIP_CLI_NOT_FOUND", "im_zip_cli not found", { cliPath });
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

async function resolveZipResult(
  bytesWritten: number | null,
  fileCount: number | null,
  options: RustZipOptions
): Promise<RustZipResult> {
  const bytes =
    typeof bytesWritten === "number"
      ? bytesWritten
      : await getOutputBytes(options.outputPath);
  const files =
    typeof fileCount === "number"
      ? fileCount
      : options.entries.length + (options.manifestJson ? 1 : 0);
  return { bytes, fileCount: files };
}
