// Path: app/src/shared/protocol_launch.ts
// Description: Tauri project-launch command response schemas (launch candidates and launch outcome)

import { z } from "zod";

export const LaunchKindSchema = z.enum(["unreal", "html"]);
export type LaunchKind = z.infer<typeof LaunchKindSchema>;

/** One launchable top-level file of a repo root, relative to that root. */
export const LaunchCandidateSchema = z.object({
  relativePath: z.string().min(1),
  kind: LaunchKindSchema,
  lastUsedMs: z.number().nullable(),
});
export type LaunchCandidate = z.infer<typeof LaunchCandidateSchema>;

/** `launch_candidates` result: candidates per repoId, ordered best-first by usage evidence. */
export const LaunchCandidatesByRepoSchema = z.record(
  z.string(),
  z.array(LaunchCandidateSchema)
);
export type LaunchCandidatesByRepo = z.infer<typeof LaunchCandidatesByRepoSchema>;

export const LaunchOutcomeSchema = z.enum(["launched", "focused", "starting"]);
export type LaunchOutcome = z.infer<typeof LaunchOutcomeSchema>;
