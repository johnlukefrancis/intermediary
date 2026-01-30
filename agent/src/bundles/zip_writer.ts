// Path: agent/src/bundles/zip_writer.ts
// Description: Archiver wrapper for creating bundle zip files

import archiver from "archiver";
import * as fs from "node:fs";
import * as fsPromises from "node:fs/promises";
import * as path from "node:path";
import { shouldIgnoreEntry } from "./ignore_rules.js";
import { logger } from "../util/logger.js";

export interface ZipWriterOptions {
  outputPath: string;
  repoRoot: string;
  includeRoot: boolean;
  /** Top-level directories to include (empty = ALL) */
  topLevelDirs: string[];
  /** Manifest JSON to include at zip root */
  manifestJson?: string;
}

export interface ZipWriteResult {
  bytes: number;
  fileCount: number;
}

/**
 * Create a zip file from repository content
 */
export async function writeZip(options: ZipWriterOptions): Promise<ZipWriteResult> {
  const { outputPath, repoRoot, includeRoot, topLevelDirs, manifestJson } = options;

  const output = fs.createWriteStream(outputPath);
  const archive = archiver("zip", { zlib: { level: 6 } });

  let fileCount = 0;

  return new Promise((resolve, reject) => {
    output.on("close", () => {
      resolve({ bytes: archive.pointer(), fileCount });
    });

    archive.on("error", (err) => { reject(err); });
    archive.on("warning", (err) => {
      logger.warn("Archiver warning", { error: err.message });
    });

    archive.pipe(output);

    void (async () => {
      try {
        // Get top-level entries
        const entries = await fsPromises.readdir(repoRoot, { withFileTypes: true });
        const includeDirs = topLevelDirs.length > 0 ? new Set(topLevelDirs) : null;

        // Add root files if requested
        if (includeRoot) {
          for (const entry of entries) {
            if (entry.isFile() && !shouldIgnoreEntry(entry.name, false)) {
              const filePath = path.join(repoRoot, entry.name);
              archive.file(filePath, { name: entry.name });
              fileCount++;
            }
          }
        }

        // Add selected directories
        for (const entry of entries) {
          if (!entry.isDirectory()) continue;
          if (shouldIgnoreEntry(entry.name, true)) continue;
          if (includeDirs && !includeDirs.has(entry.name)) continue;

          const dirPath = path.join(repoRoot, entry.name);
          const addedCount = await addDirectoryToArchive(
            archive,
            dirPath,
            entry.name
          );
          fileCount += addedCount;
        }

        // Add manifest if provided
        if (manifestJson) {
          archive.append(manifestJson, { name: "INTERMEDIARY_MANIFEST.json" });
          fileCount++;
        }

        await archive.finalize();
      } catch (err) {
        reject(err instanceof Error ? err : new Error(String(err)));
      }
    })();
  });
}

/**
 * Recursively add directory contents to archive
 */
async function addDirectoryToArchive(
  archive: archiver.Archiver,
  dirPath: string,
  archivePath: string
): Promise<number> {
  let count = 0;
  const entries = await fsPromises.readdir(dirPath, { withFileTypes: true });

  for (const entry of entries) {
    if (shouldIgnoreEntry(entry.name, entry.isDirectory())) continue;

    const fullPath = path.join(dirPath, entry.name);
    const entryArchivePath = path.posix.join(archivePath, entry.name);

    if (entry.isDirectory()) {
      count += await addDirectoryToArchive(archive, fullPath, entryArchivePath);
    } else if (entry.isFile()) {
      archive.file(fullPath, { name: entryArchivePath });
      count++;
    }
  }

  return count;
}
