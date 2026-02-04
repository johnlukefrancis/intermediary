// Path: app/src/shared/config.ts
// Description: Shared config barrel exports

export { GlobalExcludesSchema } from "./global_excludes.js";
export type { GlobalExcludes } from "./global_excludes.js";

export {
  BundlePresetSchema,
  DEFAULT_BUNDLE_PRESET,
} from "./config/bundle_presets.js";
export type { BundlePreset } from "./config/bundle_presets.js";

export {
  DEFAULT_CODE_GLOBS,
  DEFAULT_DOCS_GLOBS,
  DEFAULT_IGNORE_GLOBS,
} from "./config/glob_defaults.js";

export { RepoConfigSchema } from "./config/repo_config.js";
export type { RepoConfig } from "./config/repo_config.js";

export {
  AppConfigSchema,
  DEFAULT_APP_CONFIG,
  extractAppConfig,
  getDefaultConfig,
  parseAppConfig,
} from "./config/app_config.js";
export type { AppConfig } from "./config/app_config.js";

export { CONFIG_VERSION } from "./config/version.js";

export {
  BundleSelectionSchema,
  BundleSelectionsSchema,
  PersistedConfigSchema,
  StarredFilesEntrySchema,
  StarredFilesSchema,
  TabThemeSchema,
  ThemeModeSchema,
  UiStateSchema,
  getDefaultPersistedConfig,
  parsePersistedConfig,
} from "./config/persisted_config.js";
export type {
  BundleSelection,
  BundleSelections,
  LoadConfigResult,
  PersistedConfig,
  StarredFiles,
  StarredFilesEntry,
  TabTheme,
  ThemeMode,
  UiState,
} from "./config/persisted_config.js";
