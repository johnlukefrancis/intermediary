// Path: app/src/lib/agent/messages_source_control.ts
// Description: Typed helpers for sending source-control status, diff, and action commands

import type { AgentClient } from "./agent_client.js";
import type {
  SourceControlAction,
  SourceControlActionResult,
  SourceControlArea,
  SourceControlDiffResult,
  SourceControlStatusResult,
} from "../../shared/protocol.js";

export async function sendSourceControlStatus(
  client: AgentClient,
  repoId: string
): Promise<SourceControlStatusResult> {
  return client.send<SourceControlStatusResult>({
    type: "sourceControlStatus",
    repoId,
  });
}

export async function sendSourceControlDiff(
  client: AgentClient,
  repoId: string,
  path: string,
  area: SourceControlArea,
  originalPath?: string
): Promise<SourceControlDiffResult> {
  return client.send<SourceControlDiffResult>({
    type: "sourceControlDiff",
    repoId,
    path,
    ...(originalPath === undefined ? {} : { originalPath }),
    area,
  });
}

export async function sendSourceControlAction(
  client: AgentClient,
  repoId: string,
  action: SourceControlAction
): Promise<SourceControlActionResult> {
  return client.send<SourceControlActionResult>({
    type: "sourceControlAction",
    repoId,
    action,
  });
}
