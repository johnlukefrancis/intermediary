// Path: app/src/shared/config/persisted_config_repo_roots_migration.ts
// Description: Repo root migration helpers for persisted config normalization

import { repoRootFromLegacyPath } from "./repo_root.js";
import type { PersistedConfig } from "./persisted_config.js";

export function migrateRepoRoots(config: PersistedConfig): PersistedConfig {
  const repos = config.repos.map((repo) => {
    const resolved = repoRootFromLegacyPath(repo.root.path);
    if (!resolved) {
      return repo;
    }
    if (resolved.kind === repo.root.kind && resolved.path === repo.root.path) {
      return repo;
    }
    return { ...repo, root: resolved };
  });

  const hasChanged = repos.some((repo, index) => repo !== config.repos[index]);
  if (!hasChanged) {
    return config;
  }

  return { ...config, repos };
}

export function normalizeLegacyRepoRoots(input: unknown): unknown {
  if (!input || typeof input !== "object") return input;

  const raw = input as Record<string, unknown>;
  if (!Array.isArray(raw.repos)) {
    return input;
  }

  const rawRepos = raw.repos as unknown[];
  const migratedRepos: unknown[] = rawRepos.map((repo: unknown): unknown => {
    if (!repo || typeof repo !== "object") return repo;

    const rawRepo = repo as Record<string, unknown>;
    if ("root" in rawRepo) {
      const rawRoot = rawRepo.root;
      if (!rawRoot || typeof rawRoot !== "object") {
        return repo;
      }
      const rootRecord = rawRoot as Record<string, unknown>;
      if (
        (rootRecord.kind === "wsl" || rootRecord.kind === "windows") &&
        typeof rootRecord.path === "string"
      ) {
        const normalizedRoot = repoRootFromLegacyPath(rootRecord.path);
        if (normalizedRoot) {
          if (
            normalizedRoot.kind !== rootRecord.kind ||
            normalizedRoot.path !== rootRecord.path
          ) {
            return {
              ...rawRepo,
              root: normalizedRoot,
            };
          }
        }
      }
      return repo;
    }

    const legacyWslPath = rawRepo.wslPath;
    if (typeof legacyWslPath === "string" && legacyWslPath.trim().length > 0) {
      const root = repoRootFromLegacyPath(legacyWslPath);
      if (root) {
        const { wslPath: _legacyWslPath, ...rest } = rawRepo;
        return { ...rest, root };
      }
    }

    return repo;
  });

  const hasChanged = migratedRepos.some((repo, index) => repo !== rawRepos[index]);
  if (!hasChanged) {
    return input;
  }

  return {
    ...raw,
    repos: migratedRepos,
  };
}
