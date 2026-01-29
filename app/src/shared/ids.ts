// Path: app/src/shared/ids.ts
// Description: Shared identifiers for tabs and worktrees

import { z } from "zod";

export const TabIdSchema = z.enum([
  "texture-portal",
  "triangle-rain",
  "intermediary",
]);
export type TabId = z.infer<typeof TabIdSchema>;

export const WorktreeIdSchema = z.enum(["tr-engine"]);
export type WorktreeId = z.infer<typeof WorktreeIdSchema>;
