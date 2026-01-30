// Path: agent/src/repos/repo_watcher.ts
// Description: Chokidar file watcher setup with event emission and ignore patterns

import chokidar, { type FSWatcher } from "chokidar";
import * as path from "node:path";
import type { FileEntry, FileKind } from "../../../app/src/shared/protocol.js";
import { RingBuffer } from "../util/ring_buffer.js";
import { type CategorizerConfig, categorizeFile } from "../util/categorizer.js";
import { logger } from "../util/logger.js";

/** Patterns to ignore when watching */
const DEFAULT_IGNORE_PATTERNS = [
  "**/node_modules/**",
  "**/.git/**",
  "**/dist/**",
  "**/build/**",
  "**/target/**",
  "**/*.log",
  "**/logs/**",
  "**/.DS_Store",
  "**/Thumbs.db",
];

const RING_BUFFER_CAPACITY = 200;

export interface RepoWatcherConfig {
  docsGlobs: string[];
  codeGlobs: string[];
  ignoreGlobs: string[];
}

export interface FileChangeEvent {
  repoId: string;
  relativePath: string;
  kind: FileKind;
  mtime: Date;
  eventType: "add" | "change" | "unlink";
}

export type FileChangeHandler = (event: FileChangeEvent) => void;

export interface RepoWatcher {
  start(): Promise<void>;
  stop(): Promise<void>;
  onFileChange(handler: FileChangeHandler): void;
  getRecentChanges(): FileEntry[];
  getOtherChanges(): FileEntry[];
  readonly repoId: string;
  readonly rootPath: string;
}

export function createRepoWatcher(
  repoId: string,
  rootPath: string,
  config: RepoWatcherConfig
): RepoWatcher {
  let watcher: FSWatcher | null = null;
  const handlers: FileChangeHandler[] = [];
  const recentChanges = new RingBuffer<FileEntry>(RING_BUFFER_CAPACITY);
  const otherChanges = new RingBuffer<FileEntry>(RING_BUFFER_CAPACITY);
  const categorizerConfig: CategorizerConfig = {
    docsGlobs: config.docsGlobs,
    codeGlobs: config.codeGlobs,
  };

  function emitChange(event: FileChangeEvent): void {
    for (const handler of handlers) {
      try {
        handler(event);
      } catch (err) {
        logger.error("File change handler error", {
          repoId,
          error: err instanceof Error ? err.message : String(err),
        });
      }
    }
  }

  function handleFileEvent(
    eventType: "add" | "change" | "unlink",
    filePath: string,
    stats?: { mtime?: Date }
  ): void {
    const relativePath = path.relative(rootPath, filePath).split(path.sep).join("/");
    if (relativePath.startsWith("..")) {
      logger.warn("Skipping path outside repo root", { repoId, filePath });
      return;
    }

    const kind = categorizeFile(relativePath, categorizerConfig);
    const mtime = stats?.mtime ?? new Date();

    const entry: FileEntry = {
      path: relativePath,
      kind,
      changeType: eventType,
      mtime: mtime.toISOString(),
    };

    if (eventType !== "unlink") {
      if (kind === "other") {
        otherChanges.push(entry);
        return;
      }
      recentChanges.push(entry);
    }

    if (kind !== "other") {
      emitChange({
        repoId,
        relativePath,
        kind,
        mtime,
        eventType,
      });
    }
  }

  async function start(): Promise<void> {
    return new Promise((resolve, reject) => {
      logger.info("Starting repo watcher", { repoId, rootPath });

      const ignorePatterns = Array.from(
        new Set([...DEFAULT_IGNORE_PATTERNS, ...config.ignoreGlobs])
      );

      watcher = chokidar.watch(rootPath, {
        ignored: ignorePatterns,
        persistent: true,
        ignoreInitial: true,
        awaitWriteFinish: {
          stabilityThreshold: 100,
          pollInterval: 50,
        },
      });

      watcher.on("ready", () => {
        logger.info("Repo watcher ready", { repoId });
        resolve();
      });

      watcher.on("error", (err) => {
        logger.error("Repo watcher error", { repoId, error: err.message });
        reject(err);
      });

      watcher.on("add", (filePath, stats) => {
        handleFileEvent("add", filePath, stats);
      });

      watcher.on("change", (filePath, stats) => {
        handleFileEvent("change", filePath, stats);
      });

      watcher.on("unlink", (filePath) => {
        handleFileEvent("unlink", filePath);
      });
    });
  }

  async function stop(): Promise<void> {
    if (watcher) {
      await watcher.close();
      watcher = null;
      logger.info("Repo watcher stopped", { repoId });
    }
  }

  return {
    start,
    stop,
    onFileChange(handler) {
      handlers.push(handler);
    },
    getRecentChanges() {
      return recentChanges.toArray();
    },
    getOtherChanges() {
      return otherChanges.toArray();
    },
    get repoId() {
      return repoId;
    },
    get rootPath() {
      return rootPath;
    },
  };
}
