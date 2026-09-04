// Path: app/src/shared/protocol_worktree.ts
// Description: ZIPS-tree worktree action (delete/move/copy/rename) command/result schemas shared with the agent

import { z } from "zod";
import { ConflictPolicySchema } from "./protocol_import.js";

/** UI -> Agent: mutate the repo worktree tree (delete, move, copy, or rename entries). */
export const WorktreeActionSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("delete"),
    /** Repo-relative paths to remove; kept in the repo's quarantine until the next agent start */
    paths: z.array(z.string()).min(1),
  }),
  z.object({
    kind: z.literal("move"),
    paths: z.array(z.string()).min(1),
    /** Repo-relative destination directory; "" means the repo root */
    directory: z.string(),
    onConflict: ConflictPolicySchema,
  }),
  z.object({
    kind: z.literal("copy"),
    paths: z.array(z.string()).min(1),
    directory: z.string(),
    onConflict: ConflictPolicySchema,
  }),
  z.object({
    kind: z.literal("rename"),
    path: z.string(),
    newName: z.string(),
  }),
]);
export type WorktreeAction = z.infer<typeof WorktreeActionSchema>;

export const WorktreeActionCommandSchema = z.object({
  type: z.literal("worktreeAction"),
  repoId: z.string(),
  action: WorktreeActionSchema,
});
export type WorktreeActionCommand = z.infer<typeof WorktreeActionCommandSchema>;

/**
 * Resulting repo-relative paths: removed paths for delete, destinations for move/copy, the new
 * path (singleton) for rename.
 */
export const WorktreeActionResultSchema = z.object({
  type: z.literal("worktreeActionResult"),
  repoId: z.string(),
  kind: z.enum(["delete", "move", "copy", "rename"]),
  entries: z.array(z.string()),
});
export type WorktreeActionResult = z.infer<typeof WorktreeActionResultSchema>;
