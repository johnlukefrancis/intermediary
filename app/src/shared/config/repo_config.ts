// Path: app/src/shared/config/repo_config.ts
// Description: RepoConfig schema and type

import { z } from "zod";
import {
  BundlePresetSchema,
  DEFAULT_BUNDLE_PRESET,
} from "./bundle_presets.js";
import { RepoRootSchema } from "./repo_root.js";
import {
  DEFAULT_CODE_GLOBS,
  DEFAULT_DOCS_GLOBS,
  DEFAULT_IGNORE_GLOBS,
} from "./glob_defaults.js";

/**
 * Configuration for a single repository
 */
export const RepoConfigSchema = z.object({
  /** Unique identifier for this repo config */
  repoId: z.string(),
  /** Display name in the UI (shown in dropdown for grouped repos) */
  label: z.string(),
  /** Path-native repo root (WSL paths stay WSL, Windows paths stay Windows) */
  root: RepoRootSchema,
  /** Optional group ID - repos with same groupId share a tab with dropdown */
  groupId: z.string().optional(),
  /** Group display name (shown as tab label for grouped repos) */
  groupLabel: z.string().optional(),
  /** Whether to auto-stage file changes */
  autoStage: z.boolean().default(true),
  /** Globs that classify docs */
  docsGlobs: z.array(z.string()).default(DEFAULT_DOCS_GLOBS),
  /** Globs that classify code */
  codeGlobs: z.array(z.string()).default(DEFAULT_CODE_GLOBS),
  /** Globs that should be ignored by watchers */
  ignoreGlobs: z.array(z.string()).default(DEFAULT_IGNORE_GLOBS),
  /** Bundle presets for this repo */
  bundlePresets: z.array(BundlePresetSchema).default([DEFAULT_BUNDLE_PRESET]),
});

export type RepoConfig = z.infer<typeof RepoConfigSchema>;
