// Path: app/src/lib/agent/messages.ts
// Description: Typed helper functions for sending agent commands

import type { AgentClient } from "./agent_client.js";
import type { AppConfig } from "../../shared/config.js";
import type {
  ClientHelloResult,
  StageFileResult,
  SetOptionsResult,
  GetRepoTopLevelResult,
  RefreshResult,
} from "../../shared/protocol.js";

export async function sendClientHello(
  client: AgentClient,
  config: AppConfig,
  stagingWslRoot: string,
  stagingWinRoot: string,
  autoStageOnChange: boolean
): Promise<ClientHelloResult> {
  return client.send<ClientHelloResult>({
    type: "clientHello",
    config,
    stagingWslRoot,
    stagingWinRoot,
    autoStageOnChange,
  });
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
