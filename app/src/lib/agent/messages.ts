// Path: app/src/lib/agent/messages.ts
// Description: Typed helper functions for sending agent commands

import type { AgentClient } from "./agent_client.js";
import type { AppConfig, GlobalExcludes } from "../../shared/config.js";
import type {
  ClientHelloResult,
  StageFileResult,
  SetOptionsResult,
  GetRepoTopLevelResult,
  RefreshResult,
  BuildBundleResult,
  ListBundlesResult,
  BundleSelection,
} from "../../shared/protocol.js";

function shouldRetryWithLegacyHelloConfig(error: unknown): boolean {
  if (!(error instanceof Error)) {
    return false;
  }
  const message = error.message.toLowerCase();
  return message.includes("invalid_config") || message.includes("invalid config");
}

function hasHostRepoRoots(config: AppConfig): boolean {
  return config.repos.some((repo) => repo.root.kind === "host");
}

function toLegacyHelloConfig(config: AppConfig): AppConfig {
  const legacyRepos = config.repos.map((repo) => {
    if (repo.root.kind !== "host") {
      return repo;
    }
    return {
      ...repo,
      root: {
        kind: "windows",
        path: repo.root.path,
      },
    };
  });

  return {
    ...config,
    // TODO(ts-precision): clientHello config is a mixed-version wire seam.
    repos: legacyRepos as unknown as AppConfig["repos"],
  };
}

export async function sendClientHello(
  client: AgentClient,
  config: AppConfig,
  stagingHostRoot: string,
  stagingWslRoot: string | undefined,
  autoStageOnChange: boolean
): Promise<ClientHelloResult> {
  const effectiveStagingWslRoot = stagingWslRoot ?? stagingHostRoot;
  const baseCommand = {
    type: "clientHello",
    stagingHostRoot,
    // Legacy compatibility: current in-repo agent still expects stagingWinRoot.
    stagingWinRoot: stagingHostRoot,
    // Legacy compatibility: current in-repo agent requires stagingWslRoot.
    stagingWslRoot: effectiveStagingWslRoot,
    autoStageOnChange,
  } as const;

  try {
    return await client.send<ClientHelloResult>({
      ...baseCommand,
      config,
    });
  } catch (error) {
    if (!hasHostRepoRoots(config) || !shouldRetryWithLegacyHelloConfig(error)) {
      throw error;
    }
    return client.send<ClientHelloResult>({
      ...baseCommand,
      config: toLegacyHelloConfig(config),
    });
  }
}

export async function sendStageFile(
  client: AgentClient,
  repoId: string,
  path: string
): Promise<StageFileResult> {
  return client.send<StageFileResult>({
    type: "stageFile",
    repoId,
    path,
  });
}

export async function sendSetOptions(
  client: AgentClient,
  autoStageOnChange: boolean
): Promise<SetOptionsResult> {
  return client.send<SetOptionsResult>({
    type: "setOptions",
    autoStageOnChange,
  });
}

export async function sendGetRepoTopLevel(
  client: AgentClient,
  repoId: string
): Promise<GetRepoTopLevelResult> {
  return client.send<GetRepoTopLevelResult>({
    type: "getRepoTopLevel",
    repoId,
  });
}

export async function sendRefresh(
  client: AgentClient,
  repoId: string
): Promise<RefreshResult> {
  return client.send<RefreshResult>({
    type: "refresh",
    repoId,
  });
}

export async function sendBuildBundle(
  client: AgentClient,
  repoId: string,
  presetId: string,
  selection: BundleSelection,
  globalExcludes?: GlobalExcludes
): Promise<BuildBundleResult> {
  return client.send<BuildBundleResult>({
    type: "buildBundle",
    repoId,
    presetId,
    selection,
    globalExcludes,
  });
}

export async function sendListBundles(
  client: AgentClient,
  repoId: string,
  presetId: string
): Promise<ListBundlesResult> {
  return client.send<ListBundlesResult>({
    type: "listBundles",
    repoId,
    presetId,
  });
}
