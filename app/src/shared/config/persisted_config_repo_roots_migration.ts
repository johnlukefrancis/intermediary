// Path: app/src/shared/config/persisted_config_repo_roots_migration.ts
// Description: Repo root migration helpers for persisted config normalization

import { repoRootFromLegacyPath } from "./repo_root.js";
import type { PersistedConfig } from "./persisted_config.js";

export function migrateRepoRoots(config: PersistedConfig): PersistedConfig {
  const repos = config.repos.map((repo) => {
    if (repo.root.kind === "host") {
      const trimmedHostPath = repo.root.path.trim();
      if (trimmedHostPath === repo.root.path) {
        return repo;
      }
      return {
        ...repo,
        root: {
          ...repo.root,
          path: trimmedHostPath,
        },
      };
    }

    const normalizedWsl = repoRootFromLegacyPath(repo.root.path);
    if (!normalizedWsl) {
      return repo;
    }
    if (
      normalizedWsl.kind === repo.root.kind &&
      normalizedWsl.path === repo.root.path
    ) {
      return repo;
    }
    return { ...repo, root: normalizedWsl };
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
      const rootKind = rootRecord.kind;
      const rootPath = rootRecord.path;
      if (typeof rootPath !== "string") {
        return repo;
      }

      if (rootKind === "wsl") {
        const normalizedRoot = repoRootFromLegacyPath(rootPath);
        if (!normalizedRoot) {
          return repo;
        }
        if (
          normalizedRoot.kind !== rootKind ||
          normalizedRoot.path !== rootPath
        ) {
          return {
            ...rawRepo,
            root: normalizedRoot,
          };
        }
      } else if (rootKind === "windows") {
        const normalizedRoot = repoRootFromLegacyPath(rootPath);
        const migratedRoot =
          normalizedRoot?.kind === "host"
            ? normalizedRoot
            : { kind: "host" as const, path: rootPath.trim() };
        return {
          ...rawRepo,
          root: migratedRoot,
        };
      } else if (rootKind === "host") {
        const trimmedHostPath = rootPath.trim();
        if (trimmedHostPath !== rootPath) {
          return {
            ...rawRepo,
            root: { kind: "host", path: trimmedHostPath },
          };
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
