// Path: agent/src/repos/recent_files_store.ts
// Description: Persistence layer for recent files with debounced atomic writes

import * as fs from "node:fs";
import * as fsPromises from "node:fs/promises";
import * as path from "node:path";
import * as crypto from "node:crypto";
import type { FileEntry } from "../../../app/src/shared/protocol.js";
import { logger } from "../util/logger.js";

const PERSIST_DEBOUNCE_MS = 500;
const SCHEMA_VERSION = 1;

interface PersistedRecentFiles {
  version: number;
  repoId: string;
  repoRoot: string;
  updatedAtIso: string;
  entries: FileEntry[];
}

export interface RecentFilesStore {
  /** Sync load on startup; returns [] if missing/corrupt */
  load(repoId: string, repoRoot: string): FileEntry[];
  /** Schedule debounced save */
  scheduleSave(repoId: string, repoRoot: string, entries: FileEntry[]): void;
  /** Cancel pending save for a repo */
  cancelPending(repoId: string): void;
  /** Flush all pending writes immediately (for shutdown) */
  flush(): Promise<void>;
}

function getFilePath(stateDir: string, repoId: string): string {
  return path.join(stateDir, "recent_files", `${repoId}.json`);
}

function isValidSchema(data: unknown): data is PersistedRecentFiles {
  if (typeof data !== "object" || data === null) return false;
  const obj = data as Record<string, unknown>;
  return (
    typeof obj["version"] === "number" &&
    typeof obj["repoId"] === "string" &&
    typeof obj["repoRoot"] === "string" &&
    typeof obj["updatedAtIso"] === "string" &&
    Array.isArray(obj["entries"])
  );
}

export function createRecentFilesStore(stateDir: string): RecentFilesStore {
  const pendingWrites = new Map<string, NodeJS.Timeout>();
  const pendingData = new Map<string, { repoRoot: string; entries: FileEntry[] }>();

  function load(repoId: string, repoRoot: string): FileEntry[] {
    const filePath = getFilePath(stateDir, repoId);
    try {
      if (!fs.existsSync(filePath)) {
        logger.debug("No persisted recent files found", { repoId, filePath });
        return [];
      }

      const content = fs.readFileSync(filePath, "utf-8");
      const data: unknown = JSON.parse(content);

      if (!isValidSchema(data)) {
        logger.warn("Invalid recent files schema", { repoId, filePath });
        return [];
      }

      if (data.version !== SCHEMA_VERSION) {
        logger.warn("Unsupported recent files version", {
          repoId,
          found: data.version,
          expected: SCHEMA_VERSION,
        });
        return [];
      }

      if (data.repoRoot !== repoRoot) {
        logger.info("Repo root changed, keeping entries", {
          repoId,
          stored: data.repoRoot,
          current: repoRoot,
        });
      }

      logger.debug("Loaded persisted recent files", {
        repoId,
        count: data.entries.length,
      });
      return data.entries;
    } catch (err) {
      if (err instanceof SyntaxError) {
        logger.warn("Corrupt recent files JSON", { repoId, filePath });
      } else {
        logger.error("Failed to load recent files", {
          repoId,
          error: err instanceof Error ? err.message : String(err),
        });
      }
      return [];
    }
  }

  function scheduleSave(repoId: string, repoRoot: string, entries: FileEntry[]): void {
    // Store the latest data
    pendingData.set(repoId, { repoRoot, entries });

    // Cancel existing timeout
    const existing = pendingWrites.get(repoId);
    if (existing) {
      clearTimeout(existing);
    }

    // Schedule new debounced write
    const timeout = setTimeout(() => {
      pendingWrites.delete(repoId);
      void writeFile(repoId);
    }, PERSIST_DEBOUNCE_MS);

    pendingWrites.set(repoId, timeout);
  }

  function cancelPending(repoId: string): void {
    const existing = pendingWrites.get(repoId);
    if (existing) {
      clearTimeout(existing);
      pendingWrites.delete(repoId);
    }
    pendingData.delete(repoId);
  }

  async function writeFile(repoId: string): Promise<void> {
    const data = pendingData.get(repoId);
    if (!data) return;
    pendingData.delete(repoId);

    const filePath = getFilePath(stateDir, repoId);
    const dir = path.dirname(filePath);

    const payload: PersistedRecentFiles = {
      version: SCHEMA_VERSION,
      repoId,
      repoRoot: data.repoRoot,
      updatedAtIso: new Date().toISOString(),
      entries: data.entries,
    };

    try {
      await fsPromises.mkdir(dir, { recursive: true });

      // Atomic write: temp file + rename
      const tempPath = `${filePath}.${crypto.randomUUID()}.tmp`;
      await fsPromises.writeFile(tempPath, JSON.stringify(payload, null, 2), "utf-8");
      await fsPromises.rename(tempPath, filePath);

      logger.debug("Persisted recent files", { repoId, count: data.entries.length });
    } catch (err) {
      logger.error("Failed to persist recent files", {
        repoId,
        error: err instanceof Error ? err.message : String(err),
      });
    }
  }

  async function flush(): Promise<void> {
    // Cancel all pending timeouts
    for (const timeout of pendingWrites.values()) {
      clearTimeout(timeout);
    }
    pendingWrites.clear();

    // Write all pending data immediately
    const writes = Array.from(pendingData.keys()).map((repoId) => writeFile(repoId));
    await Promise.all(writes);
  }

  return {
    load,
    scheduleSave,
    cancelPending,
    flush,
  };
}
