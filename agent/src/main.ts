// Path: agent/src/main.ts
// Description: Agent entry point - bootstraps WebSocket server and watchers

import * as path from "node:path";
import type { WebSocket } from "ws";
import type { UiCommand, UiResponse } from "../../app/src/shared/protocol.js";
import { createWsServer, type WsServer } from "./server/ws_server.js";
import { createRouter, type Router } from "./server/router.js";
import { createRecentFilesStore } from "./repos/recent_files_store.js";
import { getRepoTopLevel, isValidRepoRoot } from "./repos/repo_top_level.js";
import { createStager } from "./staging/stager.js";
import { type PathBridgeConfig } from "./staging/path_bridge.js";
import { createBundleBuilder } from "./bundles/bundle_builder.js";
import { listBundles } from "./bundles/bundle_lister.js";
import { logger, setLogLevel } from "./util/logger.js";
import { AgentError } from "./util/errors.js";
import {
  type AgentRuntimeState,
  resetWatchers,
  shouldResetWatchers,
  shutdown,
  startWatcher,
} from "./agent_runtime.js";
import { computeConfigFingerprint } from "./util/config_fingerprint.js";

const AGENT_VERSION = "0.1.0";

const state: AgentRuntimeState = {
  watchers: new Map(),
  repoRoots: new Map(),
  repoConfigs: new Map(),
  stager: null,
  bundleBuilder: null,
  recentFilesStore: null,
  stagingWslRoot: null,
  autoStageOnChange: true,
  configFingerprint: null,
};

let router: Router;
let server: WsServer;

async function handleCommand(command: UiCommand, _ws: WebSocket): Promise<UiResponse> {
  switch (command.type) {
    case "clientHello": {
      // Compute fingerprint for new config
      const autoStageResolved = command.autoStageOnChange ?? command.config.autoStageGlobal;
      const newFingerprint = computeConfigFingerprint({
        config: command.config,
        stagingWslRoot: command.stagingWslRoot,
        autoStageOnChange: autoStageResolved,
      });

      // Check if reset is needed (idempotent reconnect)
      const needsReset = shouldResetWatchers(state, newFingerprint);

      if (needsReset) {
        logger.info("clientHello: config changed, resetting watchers", {
          isFirstHello: state.configFingerprint === null,
        });
        await resetWatchers(state);
      } else {
        logger.info("clientHello: config unchanged, keeping watchers");
      }

      // Update fingerprint
      state.configFingerprint = newFingerprint;

      // Configure staging (idempotent, safe to re-run)
      const config: PathBridgeConfig = {
        stagingWslRoot: command.stagingWslRoot,
        stagingWinRoot: command.stagingWinRoot,
      };
      state.stager = createStager(config);
      state.bundleBuilder = createBundleBuilder(config, (event) => {
        router.broadcastEvent(event);
      });
      state.stagingWslRoot = command.stagingWslRoot;
      state.autoStageOnChange = autoStageResolved;

      // Create recent files store for persistence
      // stateDir lives under staging: staging/state
      const stateDir = path.posix.join(command.stagingWslRoot, "state");
      state.recentFilesStore = createRecentFilesStore(stateDir);

      // Start watchers for configured repos (only those not already watched)
      const watchedRepoIds: string[] = [];
      for (const repo of command.config.repos) {
        // Always update maps so commands like watchRepo have correct config
        state.repoRoots.set(repo.repoId, repo.wslPath);
        state.repoConfigs.set(repo.repoId, repo);

        // Skip if already watching this repo
        if (state.watchers.has(repo.repoId)) {
          watchedRepoIds.push(repo.repoId);
          continue;
        }

        if (await isValidRepoRoot(repo.wslPath)) {
          watchedRepoIds.push(repo.repoId);
          void startWatcher(state, { router }, repo).catch((err: unknown) => {
            logger.error("Failed to start repo watcher", {
              repoId: repo.repoId,
              error: err instanceof Error ? err.message : String(err),
            });
            router.broadcastEvent({
              type: "error",
              scope: "watcher",
              message: `Watcher failed for ${repo.repoId}`,
              details: err instanceof Error ? err.message : String(err),
            });
          });
        } else {
          logger.warn("Invalid repo root, skipping", {
            repoId: repo.repoId,
            rootPath: repo.wslPath,
          });
        }
      }

      return {
        type: "clientHelloResult",
        agentVersion: AGENT_VERSION,
        watchedRepoIds,
      };
    }

    case "setOptions": {
      if (command.autoStageOnChange !== undefined) {
        state.autoStageOnChange = command.autoStageOnChange;
      }
      return {
        type: "setOptionsResult",
        autoStageOnChange: state.autoStageOnChange,
      };
    }

    case "watchRepo": {
      const existing = state.watchers.get(command.repoId);
      if (existing) {
        return { type: "watchRepoResult", repoId: command.repoId };
      }
      const rootPath = state.repoRoots.get(command.repoId);
      const repoConfig = state.repoConfigs.get(command.repoId);
      if (!rootPath) {
        throw new AgentError("UNKNOWN_REPO", `Unknown repo: ${command.repoId}`);
      }
      if (!repoConfig) {
        throw new AgentError("UNKNOWN_REPO", `Unknown repo config: ${command.repoId}`);
      }
      if (!(await isValidRepoRoot(rootPath))) {
        throw new AgentError("INVALID_REPO", `Invalid repo root: ${rootPath}`);
      }
      await startWatcher(state, { router }, repoConfig);
      return { type: "watchRepoResult", repoId: command.repoId };
    }

    case "refresh": {
      const watcher = state.watchers.get(command.repoId);
      if (!watcher) {
        throw new AgentError("UNKNOWN_REPO", `Repo not watched: ${command.repoId}`);
      }
      router.broadcastEvent({
        type: "snapshot",
        repoId: command.repoId,
        recent: watcher.getRecentChanges(),
      });
      return { type: "refreshResult", repoId: command.repoId };
    }

    case "stageFile": {
      if (!state.stager) {
        throw new AgentError("NOT_CONFIGURED", "Stager not configured");
      }
      const rootPath = state.repoRoots.get(command.repoId);
      if (!rootPath) {
        throw new AgentError("UNKNOWN_REPO", `Unknown repo: ${command.repoId}`);
      }
      const result = await state.stager.stageFile(command.repoId, rootPath, command.path);
      return {
        type: "stageFileResult",
        repoId: command.repoId,
        path: command.path,
        windowsPath: result.windowsPath,
        wslPath: result.wslPath,
        bytesCopied: result.bytesCopied,
        mtimeMs: result.mtimeMs,
      };
    }

    case "getRepoTopLevel": {
      const rootPath = state.repoRoots.get(command.repoId);
      if (!rootPath) {
        throw new AgentError("UNKNOWN_REPO", `Unknown repo: ${command.repoId}`);
      }
      const result = await getRepoTopLevel(rootPath);
      return {
        type: "getRepoTopLevelResult",
        repoId: command.repoId,
        dirs: result.dirs,
        files: result.files,
        subdirs: result.subdirs,
      };
    }

    case "buildBundle": {
      if (!state.bundleBuilder || !state.stagingWslRoot) {
        throw new AgentError("NOT_CONFIGURED", "Bundle builder not configured");
      }
      const rootPath = state.repoRoots.get(command.repoId);
      const repoConfig = state.repoConfigs.get(command.repoId);
      if (!rootPath) {
        throw new AgentError("UNKNOWN_REPO", `Unknown repo: ${command.repoId}`);
      }
      if (!repoConfig) {
        throw new AgentError("UNKNOWN_REPO", `Unknown repo config: ${command.repoId}`);
      }

      const preset = repoConfig.bundlePresets.find(
        (entry) => entry.presetId === command.presetId
      );
      if (!preset) {
        throw new AgentError(
          "UNKNOWN_PRESET",
          `Unknown preset: ${command.presetId}`
        );
      }

      const result = await state.bundleBuilder.buildBundle({
        repoId: command.repoId,
        repoRoot: rootPath,
        presetId: command.presetId,
        presetName: preset.presetName,
        selection: command.selection,
        outputDir: state.stagingWslRoot,
        ...(command.globalExcludes ? { globalExcludes: command.globalExcludes } : {}),
      });

      router.broadcastEvent({
        type: "bundleBuilt",
        repoId: command.repoId,
        presetId: command.presetId,
        windowsPath: result.windowsPath,
        aliasWindowsPath: result.aliasWindowsPath,
        bytes: result.bytes,
        fileCount: result.fileCount,
        builtAtIso: result.builtAtIso,
      });

      return {
        type: "buildBundleResult",
        repoId: command.repoId,
        presetId: command.presetId,
        windowsPath: result.windowsPath,
        wslPath: result.wslPath,
        aliasWindowsPath: result.aliasWindowsPath,
        bytes: result.bytes,
        fileCount: result.fileCount,
        builtAtIso: result.builtAtIso,
      };
    }

    case "listBundles": {
      if (!state.stagingWslRoot) {
        throw new AgentError("NOT_CONFIGURED", "Staging not configured");
      }
      const bundles = await listBundles({
        bundleDir: state.stagingWslRoot,
        repoId: command.repoId,
        presetId: command.presetId,
      });
      return {
        type: "listBundlesResult",
        repoId: command.repoId,
        presetId: command.presetId,
        bundles: bundles.map((b) => ({
          windowsPath: b.windowsPath,
          fileName: b.fileName,
          bytes: b.bytes,
          mtimeMs: b.mtimeMs,
          isLatestAlias: b.isLatestAlias,
        })),
      };
    }
  }

  const exhaustiveCheck: never = command;
  throw new AgentError("UNKNOWN_COMMAND", `Unsupported command: ${String(exhaustiveCheck)}`);
}

async function main(): Promise<void> {
  if (process.env["DEBUG"]) {
    setLogLevel("debug");
  }

  router = createRouter();
  router.setHandler(handleCommand);

  server = createWsServer({ router });
  router.setBroadcaster((msg) => { server.broadcast(msg); });

  process.on("SIGINT", () => void shutdown(state, { router, server }));
  process.on("SIGTERM", () => void shutdown(state, { router, server }));

  await server.start();
  logger.info("Agent ready", { version: AGENT_VERSION, port: server.port });
}

void main();
