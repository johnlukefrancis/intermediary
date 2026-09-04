// Path: app/src/lib/agent/messages_worktree.ts
// Description: Typed helper for sending the ZIPS-tree worktree action command

import type { AgentClient } from "./agent_client.js";
import type { WorktreeAction, WorktreeActionResult } from "../../shared/protocol.js";

export async function sendWorktreeAction(
  client: AgentClient,
  repoId: string,
  action: WorktreeAction
): Promise<WorktreeActionResult> {
  return client.send<WorktreeActionResult>({
    type: "worktreeAction",
    repoId,
    action,
  });
}
