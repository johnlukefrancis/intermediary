// Path: app/src/shared/config/persisted_config_migrations.ts
// Description: Persisted config migrations and legacy normalization

import {
  migrateLegacyModelDirPatterns,
  migrateRecommendedExtensions,
  normalizeLegacyGlobalExcludes,
} from "./persisted_config_global_excludes_migration.js";
import {
  migrateRepoRoots,
  normalizeLegacyRepoRoots,
} from "./persisted_config_repo_roots_migration.js";
import {
  migrateDefaultCodeGlobs,
  normalizeLegacyCodeGlobs,
} from "./persisted_config_code_globs_migration.js";
import { CONFIG_VERSION } from "./version.js";
import type { PersistedConfig } from "./persisted_config.js";

/**
 * Apply migrations to bring config to current version
 */
export function migrateConfig(config: PersistedConfig): PersistedConfig {
  // Migration: v1 -> v2: Add excludedSubdirs to bundle selections
  // Zod schema's .default([]) already ensures excludedSubdirs exists after parsing.

  // Migration: v2 -> v3: Remove worktree fields, use repoId for tab identity
  // Old lastActiveTabId values (e.g. "texture-portal") won't match repoIds,
  // so app.tsx validation will gracefully fall back to first repo.

  // Migration: v3 -> v4: Add globalExcludes
  // Zod schema defaults handle missing globalExcludes fields.

  // Migration: v4 -> v5: Add globalExcludes presets (deprecated in v6).
  // Migration: v5 -> v6: Drop presets in favor of explicit extension/pattern lists.
  // Migration: v6 -> v7: Add explicit dir/file exclude lists.
  // Migration: v9 -> v10: Add per-tab texture id (optional).

  let next = { ...config };

  // Migration: v7 -> v8: Add new recommended binary/model-weight extensions.
  if (config.configVersion < 8) {
    next = migrateRecommendedExtensions(next);
  }

  // Migration: v11 -> v12: Normalize localhost agent host to loopback IP.
  if (config.configVersion < 12) {
    next = migrateLoopbackAgentHost(next);
  }
  // Migration: v12 -> v13: Add agent auto-start + distro override fields.
  if (config.configVersion < 13) {
    next = migrateAgentDefaults(next);
  }
  // Migration: v14 -> v15: Add uiState.lastActiveGroupRepoIds.
  // Migration: v15 -> v16: Replace repo.wslPath with path-native repo.root.
  // Migration: v17 -> v18: Rename repo root authority kind windows -> host.
  // Migration: v18 -> v19: Add uiMode (schema default handles missing field).
  // Migration: v19 -> v20: Add uiState.windowBoundsByMode (schema default handles missing field).
  // Migration: v20 -> v21: Remove compact uiMode and fold compact bounds into standard.
  // Migration: v21 -> v22: Add windowOpacityPercent (schema default handles missing field).
  // Migration: v22 -> v23: Add textureIntensityPercent (schema default handles missing field).
  // Migration: v23 -> v24: Remove legacy model-dir path excludes from the recommended baseline.
  if (config.configVersion < 18) {
    next = migrateRepoRoots(next);
  }
  // Migration: v16 -> v17: Expand default codeGlobs coverage to broad language support.
  if (config.configVersion < 17) {
    next = migrateDefaultCodeGlobs(next);
  }
  if (config.configVersion < 24) {
    next = migrateLegacyModelDirPatterns(next);
  }

  return { ...next, configVersion: CONFIG_VERSION };
}

/**
 * Normalize legacy compact mode from raw input before schema parsing.
 * - uiMode "compact" becomes "standard"
 * - uiState.windowBoundsByMode.compact is folded into standard when standard is absent
 * - compact bounds key is removed
 */
export function normalizeLegacyUiModeAndBounds(input: unknown): unknown {
  if (!input || typeof input !== "object" || Array.isArray(input)) {
    return input;
  }

  const config = { ...(input as Record<string, unknown>) };
  if (config.uiMode === "compact") {
    config.uiMode = "standard";
  }

  const uiStateRaw = config.uiState;
  if (!uiStateRaw || typeof uiStateRaw !== "object" || Array.isArray(uiStateRaw)) {
    return config;
  }

  const uiState = { ...(uiStateRaw as Record<string, unknown>) };
  const boundsRaw = uiState.windowBoundsByMode;
  if (!boundsRaw || typeof boundsRaw !== "object" || Array.isArray(boundsRaw)) {
    config.uiState = uiState;
    return config;
  }

  const bounds = { ...(boundsRaw as Record<string, unknown>) };
  if (bounds.standard === undefined && bounds.compact !== undefined) {
    bounds.standard = bounds.compact;
  }
  if ("compact" in bounds) {
    delete bounds.compact;
  }

  uiState.windowBoundsByMode = bounds;
  config.uiState = uiState;
  return config;
}

function migrateLoopbackAgentHost(config: PersistedConfig): PersistedConfig {
  if (config.agentHost !== "localhost") {
    return config;
  }

  return {
    ...config,
    agentHost: "127.0.0.1",
  };
}

function migrateAgentDefaults(config: PersistedConfig): PersistedConfig {
  const trimmedDistro = config.agentDistro?.trim() ?? "";
  return {
    ...config,
    agentAutoStart: config.agentAutoStart,
    agentDistro: trimmedDistro.length > 0 ? trimmedDistro : null,
  };
}

export {
  normalizeLegacyCodeGlobs,
  normalizeLegacyGlobalExcludes,
  normalizeLegacyRepoRoots,
};
