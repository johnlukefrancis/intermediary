// Path: app/src/lib/agent/messages_import.ts
// Description: Typed helper for sending the drag-and-drop import command

import type { AgentClient } from "./agent_client.js";
import type { ConflictPolicy, ImportFilesResult } from "../../shared/protocol.js";

export async function sendImportFiles(
  client: AgentClient,
  repoId: string,
  directory: string,
  sources: string[],
  onConflict: ConflictPolicy
): Promise<ImportFilesResult> {
  return client.send<ImportFilesResult>({
    type: "importFiles",
    repoId,
    directory,
    sources,
    onConflict,
  });
}
