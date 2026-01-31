// Path: agent/src/bundles/zip_writer.ts
// Description: Zip writer wrapper with Rust CLI primary path and archiver fallback

import archiver from "archiver";
import * as fs from "node:fs";
import { logger } from "../util/logger.js";
import type { BundleEntry } from "./bundle_scan.js";
import { AgentError } from "../util/errors.js";
import { writeZipWithRustCli, type RustZipProgress } from "./rust_zip_cli.js";

export interface ZipWriterOptions {
  outputPath: string;
  entries: BundleEntry[];
  /** Manifest JSON to include at zip root */
  manifestJson?: string;
  onProgress?: (progress: RustZipProgress) => void;
}

export interface ZipWriteResult {
  bytes: number;
  fileCount: number;
}

/**
 * Create a zip file from repository content
 */
export async function writeZip(options: ZipWriterOptions): Promise<ZipWriteResult> {
  try {
    return await writeZipWithRustCli(options);
  } catch (err) {
    if (err instanceof AgentError && err.code === "ZIP_CLI_NOT_FOUND") {
      logger.warn("im_zip_cli unavailable, falling back to archiver", {
        error: err.message,
        details: err.details,
      });
      return writeZipWithArchiver(options);
    }
    throw err;
  }
}

async function writeZipWithArchiver(options: ZipWriterOptions): Promise<ZipWriteResult> {
  const { outputPath, entries, manifestJson } = options;

  const fileHandle = await fs.promises.open(outputPath, "w");
  const output = fs.createWriteStream(outputPath, {
    fd: fileHandle.fd,
    autoClose: false,
  });
  const archive = archiver("zip", { zlib: { level: 6 } });

  const fileCount = entries.length + (manifestJson ? 1 : 0);

  return new Promise((resolve, reject) => {
    let settled = false;

    const fail = async (err: unknown): Promise<void> => {
      if (settled) return;
      settled = true;
      try {
        await fileHandle.close();
      } catch {
        // Ignore close failures.
      }
      reject(err instanceof Error ? err : new Error(String(err)));
    };

    output.on("finish", () => {
      void (async () => {
        if (settled) return;
        try {
          await fileHandle.sync();
          await fileHandle.close();
          settled = true;
          resolve({ bytes: archive.pointer(), fileCount });
        } catch (err) {
          await fail(err);
        }
      })();
    });

    output.on("error", (err) => { void fail(err); });

    archive.on("error", (err) => { void fail(err); });
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
        void fail(err);
      }
    })();
  });
}
