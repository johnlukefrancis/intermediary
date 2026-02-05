// Path: app/src/shared/config/repo_root.ts
// Description: Repo root discriminated union schema and path normalization helpers

import { z } from "zod";

const WINDOWS_DRIVE_PATH_REGEX = /^([A-Za-z]):(?:[\\/](.*))?$/;
const WSL_MNT_PATH_REGEX = /^\/mnt\/([A-Za-z])(?:\/(.*))?$/;
const WSL_UNC_PREFIXES = ["\\\\wsl$\\", "\\\\wsl.localhost\\"];

export const RepoRootSchema = z.discriminatedUnion("kind", [
  z.object({
    kind: z.literal("wsl"),
    path: z.string().min(1),
  }),
  z.object({
    kind: z.literal("windows"),
    path: z.string().min(1),
  }),
]);

export type RepoRoot = z.infer<typeof RepoRootSchema>;

export function isWslRepoRoot(
  root: RepoRoot
): root is Extract<RepoRoot, { kind: "wsl" }> {
  return root.kind === "wsl";
}

export function repoRootKey(root: RepoRoot): string {
  if (root.kind === "windows") {
    return `windows:${root.path.toLowerCase()}`;
  }
  return `wsl:${root.path}`;
}

function normalizeWslPath(path: string): string {
  const replaced = path.trim().replace(/\\/g, "/").replace(/\/+/g, "/");
  if (replaced === "/") return replaced;
  return replaced.replace(/\/+$/, "");
}

function normalizeWindowsPath(path: string): string | null {
  const normalized = path.trim().replace(/\//g, "\\");
  const match = WINDOWS_DRIVE_PATH_REGEX.exec(normalized);
  if (!match) return null;

  const drive = match[1]?.toUpperCase();
  if (!drive) return null;

  const rest = (match[2] ?? "").replace(/\\+/g, "\\").replace(/^\\+|\\+$/g, "");
  if (rest.length === 0) return `${drive}:\\`;
  return `${drive}:\\${rest}`;
}

function resolveUncWslPath(path: string): RepoRoot | null {
  const prefix = WSL_UNC_PREFIXES.find((candidate) =>
    path.startsWith(candidate)
  );
  if (!prefix) return null;

  const remainder = path.slice(prefix.length);
  const segments = remainder.split(/[\\/]+/).filter((segment) => segment.length > 0);
  if (segments.length < 1) return null; // Must include distro name

  const pathSegments = segments.slice(1);
  if (pathSegments.length === 0) {
    return { kind: "wsl", path: "/" };
  }

  if (
    pathSegments[0]?.toLowerCase() === "mnt" &&
    pathSegments.length >= 2 &&
    /^[A-Za-z]$/.test(pathSegments[1] ?? "")
  ) {
    const driveSegment = pathSegments[1];
    if (!driveSegment) return null;
    const drive = driveSegment.toUpperCase();
    const rest = pathSegments.slice(2).join("\\");
    return {
      kind: "windows",
      path: rest.length > 0 ? `${drive}:\\${rest}` : `${drive}:\\`,
    };
  }

  return { kind: "wsl", path: `/${pathSegments.join("/")}` };
}

export function repoRootFromInputPath(inputPath: string): RepoRoot | null {
  const trimmed = inputPath.trim();
  if (trimmed.length === 0) return null;

  const uncRoot = resolveUncWslPath(trimmed);
  if (uncRoot) {
    return uncRoot.kind === "wsl"
      ? { kind: "wsl", path: normalizeWslPath(uncRoot.path) }
      : uncRoot;
  }

  if (trimmed.startsWith("/")) {
    const normalizedWsl = normalizeWslPath(trimmed);
    const mntMatch = WSL_MNT_PATH_REGEX.exec(normalizedWsl);
    if (mntMatch) {
      const drive = mntMatch[1]?.toUpperCase();
      if (!drive) return null;
      const rest = (mntMatch[2] ?? "").replace(/\//g, "\\");
      return {
        kind: "windows",
        path: rest.length > 0 ? `${drive}:\\${rest}` : `${drive}:\\`,
      };
    }
    return { kind: "wsl", path: normalizedWsl };
  }

  const windowsPath = normalizeWindowsPath(trimmed);
  if (windowsPath) {
    return { kind: "windows", path: windowsPath };
  }

  return null;
}

export function repoRootFromLegacyPath(legacyPath: string): RepoRoot | null {
  return repoRootFromInputPath(legacyPath);
}
