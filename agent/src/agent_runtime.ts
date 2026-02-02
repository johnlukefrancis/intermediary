// Path: agent/src/agent_runtime.ts
// Description: Watcher lifecycle helpers and shutdown logic for the agent runtime

import type { FileChangedEvent, StagedInfo } from "../../app/src/shared/protocol.js";
import type { RepoConfig } from "../../app/src/shared/config.js";
import { createRepoWatcher, type RepoWatcher } from "./repos/repo_watcher.js";
import { type RecentFilesStore } from "./repos/recent_files_store.js";
import { type Stager, type StageResult } from "./staging/stager.js";
import { type BundleBuilder } from "./bundles/bundle_builder.js";
import { shouldAutoStage } from "./util/categorizer.js";
import { logger } from "./util/logger.js";
import { type Router } from "./server/router.js";
import { type WsServer } from "./server/ws_server.js";

export interface AgentRuntimeState {
  watchers: Map<string, RepoWatcher>;
  repoRoots: Map<string, string>;
  repoConfigs: Map<string, RepoConfig>;
  stager: Stager | null;
  bundleBuilder: BundleBuilder | null;
  recentFilesStore: RecentFilesStore | null;
  stagingWslRoot: string | null;
  autoStageOnChange: boolean;
}

export interface AgentRuntimeDeps {
  router: Router;
  server: WsServer;
}

export async function startWatcher(
  state: AgentRuntimeState,
  deps: Pick<AgentRuntimeDeps, "router">,
  repoConfig: RepoConfig
): Promise<void> {
  const repoId = repoConfig.repoId;
  const repoRoot = repoConfig.wslPath;

  const initialEntries = state.recentFilesStore?.load(repoId, repoRoot) ?? [];

  const watcher = createRepoWatcher(repoId, repoRoot, {
    docsGlobs: repoConfig.docsGlobs,
    codeGlobs: repoConfig.codeGlobs,
    ignoreGlobs: repoConfig.ignoreGlobs,
    initialEntries,
    onPersist: (entries) => {
      state.recentFilesStore?.scheduleSave(repoId, repoRoot, entries);
    },
    onBeforeStop: () => {
      return state.recentFilesStore?.flush() ?? Promise.resolve();
    },
  });

  watcher.onFileChange((event) => {
    const baseEvent: FileChangedEvent = {
      type: "fileChanged",
      repoId: event.repoId,
      path: event.relativePath,
      kind: event.kind,
      changeType: event.eventType,
      mtime: event.mtime.toISOString(),
    };

    if (
      state.autoStageOnChange &&
      state.stager &&
      event.eventType !== "unlink" &&
      shouldAutoStage(event.kind) &&
      repoConfig.autoStage
    ) {
      state.stager.scheduleAutoStage(
        event.repoId,
        repoRoot,
        event.relativePath,
        (result: StageResult) => {
          const staged: StagedInfo = {
            wslPath: result.wslPath,
            windowsPath: result.windowsPath,
            bytesCopied: result.bytesCopied,
            mtimeMs: result.mtimeMs,
          };
          deps.router.broadcastEvent({ ...baseEvent, staged });
        },
        () => {
          deps.router.broadcastEvent(baseEvent);
        }
      );
    } else {
      deps.router.broadcastEvent(baseEvent);
    }
  });

  state.watchers.set(repoId, watcher);
  state.repoRoots.set(repoId, repoRoot);

  try {
    await watcher.start();
    deps.router.broadcastEvent({
      type: "snapshot",
      repoId,
      recent: watcher.getRecentChanges(),
    });
  } catch (err) {
    state.watchers.delete(repoId);
    throw err;
  }
}

export async function resetWatchers(state: AgentRuntimeState): Promise<void> {
  if (state.recentFilesStore) {
    await state.recentFilesStore.flush();
  }
  for (const watcher of state.watchers.values()) {
    await watcher.stop();
  }
  state.watchers.clear();
  state.repoRoots.clear();
  state.repoConfigs.clear();
}

export async function shutdown(
  state: AgentRuntimeState,
  deps: AgentRuntimeDeps
): Promise<void> {
  logger.info("Shutting down agent...");
  if (state.recentFilesStore) {
    await state.recentFilesStore.flush();
  }
  for (const watcher of state.watchers.values()) {
    await watcher.stop();
  }
  await deps.server.stop();
  process.exit(0);
}
