// Path: agent/src/bundles/zip_writer.ts
// Description: Archiver wrapper for creating bundle zip files

import archiver from "archiver";
import * as fs from "node:fs";
import { logger } from "../util/logger.js";
import type { BundleEntry } from "./bundle_scan.js";

export interface ZipWriterOptions {
  outputPath: string;
  entries: BundleEntry[];
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
  const { outputPath, entries, manifestJson } = options;

  const output = fs.createWriteStream(outputPath);
  const archive = archiver("zip", { zlib: { level: 6 } });

  const fileCount = entries.length + (manifestJson ? 1 : 0);

  return new Promise((resolve, reject) => {
    output.on("close", () => {
      resolve({ bytes: archive.pointer(), fileCount });
    });
    output.on("error", (err) => { reject(err); });

    archive.on("error", (err) => { reject(err); });
    archive.on("warning", (err) => {
      logger.warn("Archiver warning", { error: err.message });
    });

    archive.pipe(output);

    void (async () => {
      try {
        for (const entry of entries) {
          archive.file(entry.sourcePath, { name: entry.archivePath });
        }

        if (manifestJson) {
          archive.append(manifestJson, { name: "INTERMEDIARY_MANIFEST.json" });
        }

        await archive.finalize();
      } catch (err) {
        reject(err instanceof Error ? err : new Error(String(err)));
      }
    })();
  });
}
