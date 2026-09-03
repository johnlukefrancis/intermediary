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

/**
 * Size and mtime of a worktree file when the status was read. A discard carries the stamp it
 * displayed so the agent can refuse when the file changed between the read and the action.
 */
export const SourceControlStampSchema = z.object({
  bytes: z.number().int().nonnegative(),
  mtimeMs: z.number(),
  /** Sub-second nanoseconds (0..1e9); both fields must match on discard, not mtimeMs alone */
  mtimeNanos: z.number().int().gte(0).lt(1_000_000_000),
});
export type SourceControlStamp = z.infer<typeof SourceControlStampSchema>;

/** One changed path in one area; a path changed in both areas appears once per area. */
export const SourceControlEntrySchema = z.object({
  /** Repo-root-relative slash path (same contract as readTextFile) */
  path: z.string(),
  originalPath: z.string().optional(),
  area: SourceControlEntryAreaSchema,
  change: SourceControlChangeSchema,
  /** Worktree and conflict entries whose file exists on disk; absent for index-only entries */
  worktreeStamp: SourceControlStampSchema.optional(),
  /** True when a worktree/conflict entry's file is absent on disk (a deleted row) */
  worktreeMissing: z.literal(true).optional(),
});
export type SourceControlEntry = z.infer<typeof SourceControlEntrySchema>;

export const SourceControlOmittedSchema = z.object({
  /** Staged paths above the configured root (the root is a subdirectory of the Git top level) */
  stagedOutsideRoot: z.number().int().nonnegative(),
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
  /** Tree `git write-tree` would produce from this index; a commit's reviewed-state precondition */
  indexTreeSha: z.string(),
  /** The physical mutation lock for this repo was held while the status was read */
  mutationInProgress: z.boolean(),
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

/**
 * One file a discard acts on, with the state the UI reviewed. A deleted row sends
 * `expectedMissing: true` instead of a stamp: it is only restored, never removed, and the agent
 * refuses when the path exists again by the time the discard runs.
 */
export const SourceControlDiscardTargetSchema = z.object({
  path: z.string(),
  /** Absent when the file did not exist at read time */
  expectedStamp: SourceControlStampSchema.optional(),
  /** The row was a deleted/missing entry at read time; mutually exclusive with expectedStamp */
  expectedMissing: z.literal(true).optional(),
});
export type SourceControlDiscardTarget = z.infer<typeof SourceControlDiscardTargetSchema>;

export const SourceControlActionSchema = z.discriminatedUnion("kind", [
  z.object({ kind: z.literal("stage"), scope: SourceControlScopeSchema }),
  z.object({ kind: z.literal("unstage"), scope: SourceControlScopeSchema }),
  z.object({
    kind: z.literal("discard"),
    targets: z.array(SourceControlDiscardTargetSchema).min(1),
  }),
  z.object({
    kind: z.literal("commit"),
    message: z.string().min(1),
    /** The index the user reviewed; the agent refuses the commit when the index moved since */
    expectedIndexTreeSha: z.string(),
    /** The HEAD the user reviewed (status.headSha); null on an unborn branch */
    expectedHeadSha: z.string().nullable(),
  }),
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

/** The agent refused a mutation because the repository moved under the reviewed snapshot */
export const SOURCE_CONTROL_STATE_CHANGED_CODE = "SOURCE_CONTROL_STATE_CHANGED";
/** The agent is shutting down and accepts no new mutations */
export const AGENT_DRAINING_CODE = "AGENT_DRAINING";

/**
 * Effect certainty every mutation error carries in `details.effect`. `notApplied` is the agent's
 * proof that nothing happened; anything else (including a missing field) leaves the UI reconciling.
 */
export const SourceControlEffectSchema = z.enum(["notApplied", "unknown"]);
export type SourceControlEffect = z.infer<typeof SourceControlEffectSchema>;

/** The mutation-error `details` payload the UI reads; other keys are ignored. */
export const SourceControlErrorDetailsSchema = z.object({
  effect: SourceControlEffectSchema,
});
export type SourceControlErrorDetails = z.infer<typeof SourceControlErrorDetailsSchema>;

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
  /** A commit hook (e.g. lint-staged) re-staged these reviewed-root paths; not an error */
  hookChangedPaths: z.array(z.string()).optional(),
});
export type SourceControlActionResult = z.infer<typeof SourceControlActionResultSchema>;
