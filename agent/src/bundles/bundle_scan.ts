// Path: agent/src/bundles/bundle_scan.ts
// Description: Resolve bundle entries and validate selection

import * as fs from "node:fs/promises";
import * as path from "node:path";
import { shouldIgnoreEntry } from "./ignore_rules.js";
import { AgentError } from "../util/errors.js";

export interface BundleEntry {
  sourcePath: string;
  archivePath: string;
  size: number;
}

export interface BundleScanResult {
  entries: BundleEntry[];
  fileCount: number;
  totalBytes: number;
  topLevelDirsAvailable: string[];
  topLevelDirsIncluded: string[];
}

export interface BundleScanOptions {
  repoRoot: string;
  includeRoot: boolean;
  topLevelDirs: string[];
}

/**
 * Validate top-level selection against the repo root.
 */
function validateTopLevelDirs(
  topLevelDirs: string[],
  availableDirs: string[]
): string[] {
  const available = new Set(availableDirs);
  const invalid: string[] = [];
  const normalized: string[] = [];

  for (const dir of topLevelDirs) {
    if (!dir || dir === "." || dir === ".." || dir.includes("/") || dir.includes("\\")) {
      invalid.push(dir);
      continue;
    }
    if (!available.has(dir)) {
      invalid.push(dir);
      continue;
    }
    normalized.push(dir);
  }

  if (invalid.length > 0) {
    throw new AgentError("INVALID_SELECTION", "Invalid top-level directories", {
      invalid,
      available: availableDirs,
    });
  }

  return Array.from(new Set(normalized)).sort();
}

/**
 * Scan repository content for bundle inclusion.
 */
export async function scanBundleContents(
  options: BundleScanOptions
): Promise<BundleScanResult> {
  const { repoRoot, includeRoot } = options;
  const entries: BundleEntry[] = [];

  const rootEntries = await fs.readdir(repoRoot, { withFileTypes: true });
  const topLevelDirsAvailable = rootEntries
    .filter((entry) => entry.isDirectory() && !shouldIgnoreEntry(entry.name, true))
    .map((entry) => entry.name)
    .sort();

  const topLevelDirsIncluded = validateTopLevelDirs(
    options.topLevelDirs,
    topLevelDirsAvailable
  );

  if (includeRoot) {
    for (const entry of rootEntries) {
      if (!entry.isFile()) continue;
      if (shouldIgnoreEntry(entry.name, false)) continue;
      const sourcePath = path.join(repoRoot, entry.name);
      const stat = await fs.stat(sourcePath);
      entries.push({
        sourcePath,
        archivePath: entry.name,
        size: stat.size,
      });
    }
  }

  for (const dir of topLevelDirsIncluded) {
    const dirPath = path.join(repoRoot, dir);
    await collectDirectoryEntries(entries, dirPath, dir);
  }

  const totalBytes = entries.reduce((sum, entry) => sum + entry.size, 0);

  return {
    entries,
    fileCount: entries.length,
    totalBytes,
    topLevelDirsAvailable,
    topLevelDirsIncluded,
  };
}

async function collectDirectoryEntries(
  entries: BundleEntry[],
  dirPath: string,
  archiveRoot: string
): Promise<void> {
  const dirEntries = await fs.readdir(dirPath, { withFileTypes: true });

  for (const entry of dirEntries) {
    if (entry.isSymbolicLink()) continue;
    if (shouldIgnoreEntry(entry.name, entry.isDirectory())) continue;

    const sourcePath = path.join(dirPath, entry.name);
    const archivePath = path.posix.join(archiveRoot, entry.name);

    if (entry.isDirectory()) {
      await collectDirectoryEntries(entries, sourcePath, archivePath);
      continue;
    }

    if (!entry.isFile()) continue;

    const stat = await fs.stat(sourcePath);
    entries.push({
      sourcePath,
      archivePath,
      size: stat.size,
    });
  }
}
