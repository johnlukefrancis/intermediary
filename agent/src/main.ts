// Path: agent/src/main.ts
// Description: Agent entry point - bootstraps WebSocket server and watchers

import type { WebSocket } from "ws";
import type {
  UiCommand,
  UiResponse,
  FileChangedEvent,
  StagedInfo,
} from "../../app/src/shared/protocol.js";
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
  stager: Stager | null;
  autoStageOnChange: boolean;
}

const state: AgentState = {
  watchers: new Map(),
  repoRoots: new Map(),
  stager: null,
  autoStageOnChange: true,
};

let router: Router;
let server: WsServer;

async function handleCommand(command: UiCommand, _ws: WebSocket): Promise<UiResponse> {
  switch (command.type) {
    case "clientHello": {
      // Configure staging
      const config: PathBridgeConfig = {
        stagingWslRoot: command.stagingWslRoot,
        stagingWinRoot: command.stagingWinRoot,
      };
      state.stager = createStager(config);
      state.autoStageOnChange = command.autoStageOnChange ?? true;

      // Start watchers for configured repos
      const watchedRepoIds: string[] = [];
      for (const [repoId, rootPath] of Object.entries(command.repos)) {
        if (await isValidRepoRoot(rootPath)) {
          await startWatcher(repoId, rootPath);
          watchedRepoIds.push(repoId);
        } else {
          logger.warn("Invalid repo root, skipping", { repoId, rootPath });
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
      if (!rootPath) {
        throw new AgentError("UNKNOWN_REPO", `Unknown repo: ${command.repoId}`);
      }
      await startWatcher(command.repoId, rootPath);
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
}

async function startWatcher(repoId: string, rootPath: string): Promise<void> {
  const watcher = createRepoWatcher(repoId, rootPath);

  watcher.onFileChange((event) => {
    const baseEvent: FileChangedEvent = {
      type: "fileChanged",
      repoId: event.repoId,
      path: event.relativePath,
      kind: event.kind,
      mtime: event.mtime.toISOString(),
    };

    // Auto-stage if enabled and file type qualifies
    if (
      state.autoStageOnChange &&
      state.stager &&
      event.eventType !== "unlink" &&
      shouldAutoStage(event.kind)
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
        }
      );
    } else {
      router.broadcastEvent(baseEvent);
    }
  });

  await watcher.start();
  state.watchers.set(repoId, watcher);
  state.repoRoots.set(repoId, rootPath);
}

async function shutdown(): Promise<void> {
  logger.info("Shutting down agent...");
  for (const watcher of state.watchers.values()) {
    await watcher.stop();
  }
  await server.stop();
  process.exit(0);
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
