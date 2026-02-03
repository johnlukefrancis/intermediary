// Path: agent/src/commands/client_hello.ts
// Description: Handles clientHello command with watcher-safe idempotency

import * as path from "node:path";
import type { UiCommand, UiResponse } from "../../../app/src/shared/protocol.js";
import { createBundleBuilder } from "../bundles/bundle_builder.js";
import { createRecentFilesStore } from "../repos/recent_files_store.js";
import { isValidRepoRoot } from "../repos/repo_top_level.js";
import { createStager } from "../staging/stager.js";
import { type PathBridgeConfig } from "../staging/path_bridge.js";
import { computeConfigFingerprint } from "../util/config_fingerprint.js";
import { logger } from "../util/logger.js";
import {
  type AgentRuntimeState,
  resetWatchers,
  shouldResetWatchers,
  startWatcher,
} from "../agent_runtime.js";
import { type Router } from "../server/router.js";

type ClientHelloCommand = Extract<UiCommand, { type: "clientHello" }>;

export async function handleClientHello(
  state: AgentRuntimeState,
  router: Router,
  command: ClientHelloCommand,
  agentVersion: string
): Promise<UiResponse> {
  const autoStageResolved = command.autoStageOnChange ?? command.config.autoStageGlobal;
  const newFingerprint = computeConfigFingerprint({
    config: command.config,
    stagingWslRoot: command.stagingWslRoot,
  });

  const needsReset = shouldResetWatchers(state, newFingerprint);
  if (needsReset) {
    logger.info("clientHello: config changed, resetting watchers", {
      isFirstHello: state.configFingerprint === null,
    });
    await resetWatchers(state);
  } else {
    logger.info("clientHello: config unchanged, keeping watchers");
  }

  state.configFingerprint = newFingerprint;

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
  state.recentFilesLimit = command.config.recentFilesLimit;

  const stateDir = path.posix.join(command.stagingWslRoot, "state");
  state.recentFilesStore = createRecentFilesStore(stateDir);

  const watchedRepoIds: string[] = [];
  for (const repo of command.config.repos) {
    state.repoRoots.set(repo.repoId, repo.wslPath);
    state.repoConfigs.set(repo.repoId, repo);

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
    agentVersion,
    watchedRepoIds,
  };
}
