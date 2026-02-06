// Path: app/src/shared/config/persisted_config_code_globs_migration.ts
// Description: Default-only additive migration for expanded code globs coverage.

import {
  DEFAULT_CODE_GLOBS,
  LEGACY_DEFAULT_CODE_GLOBS,
} from "./glob_defaults.js";
import type { PersistedConfig } from "./persisted_config.js";

const LEGACY_DEFAULT_CODE_GLOBS_SET = buildComparableSet(LEGACY_DEFAULT_CODE_GLOBS);
const EXPANDED_DEFAULT_CODE_GLOBS_SET = buildComparableSet(DEFAULT_CODE_GLOBS);
const EXPANDED_DEFAULT_CODE_GLOBS_WITHOUT_INL_SET = buildComparableSet(
  DEFAULT_CODE_GLOBS.filter((glob) => normalizeGlob(glob) !== "**/*.inl")
);
const LEGACY_CODE_GLOBS_VARIANTS = [
  LEGACY_DEFAULT_CODE_GLOBS_SET,
  EXPANDED_DEFAULT_CODE_GLOBS_SET,
  EXPANDED_DEFAULT_CODE_GLOBS_WITHOUT_INL_SET,
];

export function migrateDefaultCodeGlobs(config: PersistedConfig): PersistedConfig {
  const migratedRepos = config.repos.map((repo) => {
    if (!isLegacyDefaultCodeGlobs(repo.codeGlobs)) {
      return repo;
    }
    return {
      ...repo,
      codeGlobs: [...DEFAULT_CODE_GLOBS],
    };
  });

  return {
    ...config,
    repos: migratedRepos,
  };
}

export function normalizeLegacyCodeGlobs(input: unknown): unknown {
  if (!input || typeof input !== "object") return input;
  const raw = input as Record<string, unknown>;
  if (!shouldNormalizeCodeGlobs(raw.configVersion)) {
    return input;
  }
  if (!Array.isArray(raw.repos)) return input;
  const rawRepos = raw.repos as unknown[];

  const migratedRepos: unknown[] = rawRepos.map((repo: unknown): unknown => {
    if (!repo || typeof repo !== "object") return repo;
    const rawRepo = repo as Record<string, unknown>;
    if (!Array.isArray(rawRepo.codeGlobs)) return repo;
    const codeGlobs = rawRepo.codeGlobs.filter(
      (value): value is string => typeof value === "string"
    );
    if (!isLegacyDefaultCodeGlobs(codeGlobs)) {
      return repo;
    }
    return {
      ...rawRepo,
      codeGlobs: [...DEFAULT_CODE_GLOBS],
    };
  });

  return {
    ...raw,
    repos: migratedRepos,
  };
}

function isLegacyDefaultCodeGlobs(globs: string[]): boolean {
  const current = buildComparableSet(globs);
  for (const variant of LEGACY_CODE_GLOBS_VARIANTS) {
    if (current.size !== variant.size) {
      continue;
    }
    let variantMatch = true;
    for (const glob of variant) {
      if (!current.has(glob)) {
        variantMatch = false;
        break;
      }
    }
    if (variantMatch) {
      return true;
    }
  }
  return false;
}

function buildComparableSet(globs: string[]): Set<string> {
  const set = new Set<string>();
  for (const glob of globs) {
    const normalized = normalizeGlob(glob);
    if (normalized.length > 0) {
      set.add(normalized);
    }
  }
  return set;
}

function normalizeGlob(glob: string): string {
  return glob.trim().toLowerCase();
}

function shouldNormalizeCodeGlobs(configVersion: unknown): boolean {
  if (typeof configVersion !== "number" || !Number.isFinite(configVersion)) {
    return true;
  }
  return configVersion < 17;
}
