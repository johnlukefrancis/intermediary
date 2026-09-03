// Path: app/src/shared/protocol_source_control.ts
// Description: Source-control status, diff, and action command/result schemas shared with the agents

import { z } from "zod";

/** Which side of the index a diff refers to: HEAD->index (staged) or index->worktree. */
export const SourceControlAreaSchema = z.enum(["index", "worktree"]);
export type SourceControlArea = z.infer<typeof SourceControlAreaSchema>;

export const SourceControlEntryAreaSchema = z.enum(["index", "worktree", "conflict"]);
export type SourceControlEntryArea = z.infer<typeof SourceControlEntryAreaSchema>;

export const SourceControlChangeSchema = z.enum([
  "added",
  "modified",
  "deleted",
  "renamed",
  "copied",
  "typeChanged",
  "untracked",
  "unmerged",
]);
export type SourceControlChange = z.infer<typeof SourceControlChangeSchema>;

/** One changed path in one area; a path changed in both areas appears once per area. */
export const SourceControlEntrySchema = z.object({
  /** Repo-root-relative slash path (same contract as readTextFile) */
  path: z.string(),
  originalPath: z.string().optional(),
  area: SourceControlEntryAreaSchema,
  change: SourceControlChangeSchema,
});
export type SourceControlEntry = z.infer<typeof SourceControlEntrySchema>;

export const SourceControlOmittedSchema = z.object({
  /** Staged paths above the configured root (the root is a subdirectory of the Git top level) */
  stagedOutsideRoot: z.number().int().nonnegative(),
  /** Unmerged paths above the configured root: unlisted, but they block the commit and drive the alert */
  unmergedOutsideRoot: z.number().int().nonnegative(),
  unrepresentablePath: z.number().int().nonnegative(),
});
export type SourceControlOmitted = z.infer<typeof SourceControlOmittedSchema>;

export const SourceControlStatusSchema = z.object({
  branch: z.string().nullable(),
  headSha: z.string().nullable(),
  detached: z.boolean(),
  upstream: z.string().nullable(),
  ahead: z.number().int().nonnegative().nullable(),
  behind: z.number().int().nonnegative().nullable(),
  index: z.array(SourceControlEntrySchema),
  worktree: z.array(SourceControlEntrySchema),
  conflicts: z.array(SourceControlEntrySchema),
  omitted: SourceControlOmittedSchema,
  /** Git's own answer: the index differs from HEAD, or a merge is in progress */
  committable: z.boolean(),
  /** True when Git's status output overran its budget; lists are incomplete */
  truncated: z.boolean(),
  capturedAtIso: z.string(),
});
export type SourceControlStatus = z.infer<typeof SourceControlStatusSchema>;

/** Pathspec scope: "all" means everything under the repo root, never the whole repository. */
export const SourceControlScopeSchema = z.discriminatedUnion("mode", [
  z.object({ mode: z.literal("all") }),
  z.object({ mode: z.literal("paths"), paths: z.array(z.string()).min(1) }),
]);
export type SourceControlScope = z.infer<typeof SourceControlScopeSchema>;

export const SourceControlActionSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("stage"), scope: SourceControlScopeSchema }),
  z.object({ kind: z.literal("unstage"), scope: SourceControlScopeSchema }),
  z.object({ kind: z.literal("discard"), paths: z.array(z.string()).min(1) }),
  z.object({ kind: z.literal("commit"), message: z.string().min(1) }),
  z.object({ kind: z.literal("push") }),
  z.object({ kind: z.literal("pull") }),
]);
export type SourceControlAction = z.infer<typeof SourceControlActionSchema>;

export const SourceControlActionKindSchema = z.enum([
  "stage",
  "unstage",
  "discard",
  "commit",
  "push",
  "pull",
]);
export type SourceControlActionKind = z.infer<typeof SourceControlActionKindSchema>;

export const SourceControlStatusCommandSchema = z.object({
  type: z.literal("sourceControlStatus"),
  repoId: z.string(),
});
export type SourceControlStatusCommand = z.infer<typeof SourceControlStatusCommandSchema>;

export const SourceControlDiffCommandSchema = z.object({
  type: z.literal("sourceControlDiff"),
  repoId: z.string(),
  path: z.string(),
  /** Rename source for renamed/copied entries so the diff can pair both paths */
  originalPath: z.string().optional(),
  area: SourceControlAreaSchema,
});
export type SourceControlDiffCommand = z.infer<typeof SourceControlDiffCommandSchema>;

export const SourceControlActionCommandSchema = z.object({
  type: z.literal("sourceControlAction"),
  repoId: z.string(),
  action: SourceControlActionSchema,
});
export type SourceControlActionCommand = z.infer<typeof SourceControlActionCommandSchema>;

export const SourceControlStatusResultSchema = z.object({
  type: z.literal("sourceControlStatusResult"),
  repoId: z.string(),
  status: SourceControlStatusSchema,
});
export type SourceControlStatusResult = z.infer<typeof SourceControlStatusResultSchema>;

export const SourceControlDiffResultSchema = z.object({
  type: z.literal("sourceControlDiffResult"),
  repoId: z.string(),
  path: z.string(),
  area: SourceControlAreaSchema,
  patch: z.string(),
  truncated: z.boolean(),
  binary: z.boolean(),
});
export type SourceControlDiffResult = z.infer<typeof SourceControlDiffResultSchema>;

export const SourceControlActionResultSchema = z.object({
  type: z.literal("sourceControlActionResult"),
  repoId: z.string(),
  kind: SourceControlActionKindSchema,
  status: SourceControlStatusSchema,
  commitSha: z.string().optional(),
});
export type SourceControlActionResult = z.infer<typeof SourceControlActionResultSchema>;

/** Which snapshot of a changed image a pane shows; the agent picks the pair from the Git state. */
export const ImageDiffSourceSchema = z.enum(["head", "index", "worktree", "ours", "theirs"]);
export type ImageDiffSource = z.infer<typeof ImageDiffSourceSchema>;

/** One side of an image diff; `truncated` carries an empty payload past the per-side bound. */
export const ImageDiffSideSchema = z.object({
  source: ImageDiffSourceSchema,
  dataBase64: z.string(),
  mimeType: z.string(),
  bytes: z.number().int().nonnegative(),
  truncated: z.boolean(),
});
export type ImageDiffSide = z.infer<typeof ImageDiffSideSchema>;

export const SourceControlImageDiffCommandSchema = z.object({
  type: z.literal("sourceControlImageDiff"),
  repoId: z.string(),
  path: z.string(),
  /** Rename source for renamed/copied entries so the HEAD side can be located */
  originalPath: z.string().optional(),
  area: SourceControlAreaSchema,
});
export type SourceControlImageDiffCommand = z.infer<typeof SourceControlImageDiffCommandSchema>;

/** A side that does not exist (added, deleted, unborn HEAD) is null, never an error. */
export const SourceControlImageDiffResultSchema = z.object({
  type: z.literal("sourceControlImageDiffResult"),
  repoId: z.string(),
  path: z.string(),
  area: SourceControlAreaSchema,
  before: ImageDiffSideSchema.nullable(),
  after: ImageDiffSideSchema.nullable(),
});
export type SourceControlImageDiffResult = z.infer<typeof SourceControlImageDiffResultSchema>;
