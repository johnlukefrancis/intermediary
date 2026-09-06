// Path: app/src/lib/stream/stream_agent_support.ts
// Description: Whether the connected agent build publishes fileDelta events the Stream panel can render

/**
 * The first agent build that publishes `fileDelta`. PLACEHOLDER: the closeout rung of the
 * Stream ladder sets this to the version the release flow actually assigns.
 */
export const STREAM_MIN_AGENT_VERSION = "0.1.23";

export type StreamSupport = "supported" | "update-required" | "unknown";

interface SemVerTriple {
  major: number;
  minor: number;
  patch: number;
}

/**
 * Parse a leading `major.minor.patch`. Anything else (a hash, a channel suffix that
 * replaces a number, a missing component) is unparsable and stays unknown.
 */
function parseVersion(version: string): SemVerTriple | null {
  const match = /^(\d+)\.(\d+)\.(\d+)/.exec(version.trim());
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

/**
 * `unknown` is the honest answer before a hello lands and for a build we cannot read;
 * the panel treats it as "not yet proven old" and shows the ordinary waiting states.
 */
export function streamSupport(agentVersion: string | null): StreamSupport {
  if (agentVersion === null) return "unknown";
  const actual = parseVersion(agentVersion);
  const required = parseVersion(STREAM_MIN_AGENT_VERSION);
  if (!actual || !required) return "unknown";
  return compare(actual, required) >= 0 ? "supported" : "update-required";
}
