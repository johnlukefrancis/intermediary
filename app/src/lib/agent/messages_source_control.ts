// Path: app/src/lib/agent/messages_source_control.ts
// Description: Typed helpers for sending source-control status, diff, and action commands

import type { AgentClient } from "./agent_client.js";
import type {
  SourceControlAction,
  SourceControlActionResult,
  SourceControlArea,
  SourceControlDiffResult,
  SourceControlImageDiffResult,
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

/** Both snapshots of a changed image in one bounded read; a missing side comes back null. */
export async function sendSourceControlImageDiff(
  client: AgentClient,
  repoId: string,
  path: string,
  area: SourceControlArea,
  originalPath?: string
): Promise<SourceControlImageDiffResult> {
  return client.send<SourceControlImageDiffResult>({
    type: "sourceControlImageDiff",
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
