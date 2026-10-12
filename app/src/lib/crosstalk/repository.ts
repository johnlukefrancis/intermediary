// Path: app/src/lib/crosstalk/repository.ts
// Description: Exact normalized root-prefix attribution for Crosstalk sender paths

import { repoRootFromInputPath, repoRootKey } from "../../shared/config/repo_root.js";

export function crosstalkPathKey(path: string): string | null {
  const root = repoRootFromInputPath(path);
  return root ? repoRootKey(root).replace(/\\/g, "/").replace(/\/$/, "") : null;
}

export function containingRepository<T extends { path: string }>(cwd: string, repos: readonly T[]): T | undefined {
  const key = crosstalkPathKey(cwd);
  if (key === null) return undefined;
  let match: T | undefined;
  let length = -1;
  for (const repo of repos) {
    const root = crosstalkPathKey(repo.path);
    if (root !== null && root.length > length && (key === root || key.startsWith(`${root}/`))) {
      match = repo;
      length = root.length;
    }
  }
  return match;
}
