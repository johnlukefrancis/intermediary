// Path: app/src/shared/protocol_repo_topology.ts
// Description: Repo topology and lazy directory listing protocol schemas

import { z } from "zod";

/** Request top-level directory listing for a repo */
export const GetRepoTopLevelCommandSchema = z.object({
  type: z.literal("getRepoTopLevel"),
  repoId: z.string(),
});

/** Request direct children for a repo-relative directory path */
export const ListRepoDirectoryCommandSchema = z.object({
  type: z.literal("listRepoDirectory"),
  repoId: z.string(),
  path: z.string(),
});

/** Top-level directories and files for a repo */
export const GetRepoTopLevelResultSchema = z.object({
  type: z.literal("getRepoTopLevelResult"),
  repoId: z.string(),
  dirs: z.array(z.string()),
  files: z.array(z.string()),
  /** Nested subdirectory paths within each top-level dir, up to repo depth 4 */
  subdirs: z.record(z.string(), z.array(z.string())).optional(),
  /** Dir names that are excluded by default (e.g. node_modules, .git, target) */
  defaultExcluded: z.array(z.string()).default([]),
});

/** Direct child directories and files for a repo-relative directory path */
export const ListRepoDirectoryResultSchema = z.object({
  type: z.literal("listRepoDirectoryResult"),
  repoId: z.string(),
  path: z.string(),
  dirs: z.array(z.string()),
  files: z.array(z.string()),
});

export type GetRepoTopLevelCommand = z.infer<typeof GetRepoTopLevelCommandSchema>;
export type ListRepoDirectoryCommand = z.infer<typeof ListRepoDirectoryCommandSchema>;
export type GetRepoTopLevelResult = z.infer<typeof GetRepoTopLevelResultSchema>;
export type ListRepoDirectoryResult = z.infer<typeof ListRepoDirectoryResultSchema>;
