// Path: app/src/lib/agent/activity_support.ts
// Description: Shared minimum agent contract for root-identified activity in Stream and Overview

/** The first agent build that identifies the root on every activity event. */
export const ACTIVITY_MIN_AGENT_VERSION = "0.1.24";

export type ActivitySupport = "supported" | "update-required" | "unknown";

interface SemVerTriple {
  major: number;
  minor: number;
  patch: number;
}

/** Missing components or non-numeric build identities stay unknown. */
function parseVersion(version: string): SemVerTriple | null {
  const match = /^(\d+)\.(\d+)\.(\d+)$/.exec(version);
  if (!match) return null;
  const [, major, minor, patch] = match;
  if (major === undefined || minor === undefined || patch === undefined) return null;
  return { major: Number(major), minor: Number(minor), patch: Number(patch) };
}

function compare(left: SemVerTriple, right: SemVerTriple): number {
  if (left.major !== right.major) return left.major - right.major;
  if (left.minor !== right.minor) return left.minor - right.minor;
  return left.patch - right.patch;
}

/** Before hello or for an unreadable build identity, the panel shows
 * its ordinary waiting states with support still unknown. */
export function activitySupport(agentVersion: string | null): ActivitySupport {
  if (agentVersion === null) return "unknown";
  const actual = parseVersion(agentVersion);
  const required = parseVersion(ACTIVITY_MIN_AGENT_VERSION);
  if (!actual || !required) return "unknown";
  return compare(actual, required) >= 0 ? "supported" : "update-required";
}
