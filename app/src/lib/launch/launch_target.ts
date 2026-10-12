// Path: app/src/lib/launch/launch_target.ts
// Description: Pure launch-target resolution: persisted choice while still a candidate, else the top-ranked candidate

import type { LaunchCandidate } from "../../shared/protocol_launch.js";
import type { RepoConfig } from "../../shared/config.js";

export function effectiveLaunchTarget(
  candidates: readonly LaunchCandidate[],
  persistedTarget: string | undefined
): LaunchCandidate | null {
  const persisted = persistedTarget === undefined
    ? undefined
    : candidates.find((c) => c.target === persistedTarget);
  return persisted ?? candidates[0] ?? null;
}

export function demoTargetFor(
  repos: readonly RepoConfig[],
  targets: Readonly<Record<string, string>>,
  repoId: string
): string | null {
  const repo = repos.find((candidate) => candidate.repoId === repoId);
  return repo ? targets[repo.groupId ?? repo.repoId] ?? null : null;
}

/** Human-readable file or page name for launch controls. */
export function launchFileName(target: string): string {
  const namePath = /^https?:\/\//i.test(target)
    ? target.replace(/[?#].*$/, "").replace(/\/+$/, "")
    : target;
  const segments = namePath.split(/[\\/]/);
  return segments[segments.length - 1] || target;
}
