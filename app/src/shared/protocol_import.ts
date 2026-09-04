// Path: app/src/shared/protocol_import.ts
// Description: Drag-and-drop file import command/result schemas shared with the agent

import { z } from "zod";

/**
 * UI -> Agent conflict authorization: "refuse" fails on any collision; `{ replace }` authorizes
 * overwriting exactly the listed repo-relative paths (the full conflict set a prior refusal
 * returned in `details.conflicts`) — never an unscoped "replace everything".
 */
export const ConflictPolicySchema = z.union([
  z.literal("refuse"),
  z.object({ replace: z.array(z.string()) }),
]);
export type ConflictPolicy = z.infer<typeof ConflictPolicySchema>;

/** UI -> Agent: copy OS files/folders into a repo-relative directory ("" = root). */
export const ImportFilesCommandSchema = z.object({
  type: z.literal("importFiles"),
  repoId: z.string(),
  /** Repo-relative target directory; "" means the repo root */
  directory: z.string(),
  /** Absolute OS paths exactly as Tauri delivered them */
  sources: z.array(z.string()),
  onConflict: ConflictPolicySchema,
});
export type ImportFilesCommand = z.infer<typeof ImportFilesCommandSchema>;

export const ImportedFileSchema = z.object({
  path: z.string(),
  bytes: z.number().int().nonnegative(),
});
export type ImportedFile = z.infer<typeof ImportedFileSchema>;

export const ImportFilesResultSchema = z.object({
  type: z.literal("importFilesResult"),
  repoId: z.string(),
  directory: z.string(),
  imported: z.array(ImportedFileSchema),
});
export type ImportFilesResult = z.infer<typeof ImportFilesResultSchema>;
