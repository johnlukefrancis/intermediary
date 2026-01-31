// Path: agent/src/bundles/zip_writer.ts
// Description: Zip writer wrapper that invokes the Rust zip CLI
import type { BundleEntry } from "./bundle_scan.js";
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
  return await writeZipWithRustCli(options);
}
