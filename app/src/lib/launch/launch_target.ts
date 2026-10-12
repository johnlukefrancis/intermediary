// Path: app/src/lib/launch/launch_target.ts
// Description: Pure launch-target resolution: persisted choice while still a candidate, else the top-ranked candidate

import type { LaunchCandidate } from "../../shared/protocol_launch.js";

export function effectiveLaunchTarget(
  candidates: readonly LaunchCandidate[],
  persistedRelativePath: string | undefined
): LaunchCandidate | null {
  const persisted = persistedRelativePath === undefined
    ? undefined
    : candidates.find((c) => c.relativePath === persistedRelativePath);
  return persisted ?? candidates[0] ?? null;
}

/** Last path segment of a root-relative path, for labels and tooltips. */
export function launchFileName(relativePath: string): string {
  const segments = relativePath.split(/[\\/]/);
  return segments[segments.length - 1] || relativePath;
}
