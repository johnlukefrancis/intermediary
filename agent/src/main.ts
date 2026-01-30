// Path: agent/src/main.ts
// Description: Agent entry point - bootstraps WebSocket server and watchers

import type { WebSocket } from "ws";
import type {
  UiCommand,
  UiResponse,
  FileChangedEvent,
  StagedInfo,
} from "../../app/src/shared/protocol.js";
import type { RepoConfig } from "../../app/src/shared/config.js";
import { createWsServer, type WsServer } from "./server/ws_server.js";
import { createRouter, type Router } from "./server/router.js";
import { createRepoWatcher, type RepoWatcher } from "./repos/repo_watcher.js";
import { getRepoTopLevel, isValidRepoRoot } from "./repos/repo_top_level.js";
import { createStager, type Stager, type StageResult } from "./staging/stager.js";
import { type PathBridgeConfig } from "./staging/path_bridge.js";
import { shouldAutoStage } from "./util/categorizer.js";
import { logger, setLogLevel } from "./util/logger.js";
import { AgentError } from "./util/errors.js";

const AGENT_VERSION = "0.1.0";

interface AgentState {
  watchers: Map<string, RepoWatcher>;
  repoRoots: Map<string, string>;
  repoConfigs: Map<string, RepoConfig>;
  stager: Stager | null;
  autoStageOnChange: boolean;
}

const state: AgentState = {
  watchers: new Map(),
  repoRoots: new Map(),
  repoConfigs: new Map(),
  stager: null,
  autoStageOnChange: true,
};

let router: Router;
let server: WsServer;

async function handleCommand(command: UiCommand, _ws: WebSocket): Promise<UiResponse> {
  switch (command.type) {
    case "clientHello": {
      await resetWatchers();

      // Configure staging
      const config: PathBridgeConfig = {
        stagingWslRoot: command.stagingWslRoot,
        stagingWinRoot: command.stagingWinRoot,
      };
      state.stager = createStager(config);
      state.autoStageOnChange = command.autoStageOnChange ?? command.config.autoStageGlobal;

      // Start watchers for configured repos
      const watchedRepoIds: string[] = [];
      for (const repo of command.config.repos) {
        state.repoRoots.set(repo.repoId, repo.wslPath);
        state.repoConfigs.set(repo.repoId, repo);

        if (await isValidRepoRoot(repo.wslPath)) {
          await startWatcher(repo);
          watchedRepoIds.push(repo.repoId);
        } else {
          logger.warn("Invalid repo root, skipping", {
            repoId: repo.repoId,
            rootPath: repo.wslPath,
          });
        }
      }

      queueMicrotask(() => {
        for (const repoId of watchedRepoIds) {
          const watcher = state.watchers.get(repoId);
          if (!watcher) {
            continue;
          }
          router.broadcastEvent({
            type: "snapshot",
            repoId,
            recent: watcher.getRecentChanges(),
          });
        }
      });

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
      await startWatcher(repoConfig);
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
      };
    }

    case "buildBundle": {
      // TODO: Implement bundle building
      throw new AgentError("NOT_IMPLEMENTED", "Bundle building not yet implemented");
    }
  }

  const exhaustiveCheck: never = command;
  throw new AgentError("UNKNOWN_COMMAND", `Unsupported command: ${String(exhaustiveCheck)}`);
}

async function startWatcher(repoConfig: RepoConfig): Promise<void> {
  const watcher = createRepoWatcher(repoConfig.repoId, repoConfig.wslPath, {
    docsGlobs: repoConfig.docsGlobs,
    codeGlobs: repoConfig.codeGlobs,
    ignoreGlobs: repoConfig.ignoreGlobs,
  });

  watcher.onFileChange((event) => {
    const repoId = event.repoId;
    const rootPath = repoConfig.wslPath;
    const baseEvent: FileChangedEvent = {
      type: "fileChanged",
      repoId,
      path: event.relativePath,
      kind: event.kind,
      changeType: event.eventType,
      mtime: event.mtime.toISOString(),
    };

    // Auto-stage if enabled and file type qualifies
    if (
      state.autoStageOnChange &&
      state.stager &&
      event.eventType !== "unlink" &&
      shouldAutoStage(event.kind) &&
      repoConfig.autoStage
    ) {
      state.stager.scheduleAutoStage(
        repoId,
        rootPath,
        event.relativePath,
        (result: StageResult) => {
          const staged: StagedInfo = {
            wslPath: result.wslPath,
            windowsPath: result.windowsPath,
            bytesCopied: result.bytesCopied,
            mtimeMs: result.mtimeMs,
          };
          router.broadcastEvent({ ...baseEvent, staged });
        },
        () => {
          router.broadcastEvent(baseEvent);
        }
      );
    } else {
      router.broadcastEvent(baseEvent);
    }
  });

  await watcher.start();
  state.watchers.set(repoConfig.repoId, watcher);
  state.repoRoots.set(repoConfig.repoId, repoConfig.wslPath);
}

async function shutdown(): Promise<void> {
  logger.info("Shutting down agent...");
  for (const watcher of state.watchers.values()) {
    await watcher.stop();
  }
  await server.stop();
  process.exit(0);
}

async function resetWatchers(): Promise<void> {
  for (const watcher of state.watchers.values()) {
    await watcher.stop();
  }
  state.watchers.clear();
  state.repoRoots.clear();
  state.repoConfigs.clear();
}

async function main(): Promise<void> {
  if (process.env["DEBUG"]) {
    setLogLevel("debug");
  }

  router = createRouter();
  router.setHandler(handleCommand);

  server = createWsServer({ router });
  router.setBroadcaster((msg) => { server.broadcast(msg); });

  process.on("SIGINT", () => void shutdown());
  process.on("SIGTERM", () => void shutdown());

  await server.start();
  logger.info("Agent ready", { version: AGENT_VERSION, port: server.port });
}

void main();
