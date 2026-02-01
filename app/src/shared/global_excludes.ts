// Path: app/src/shared/global_excludes.ts
// Description: Global bundle exclude schema and UI options

import { z } from "zod";

export const GlobalExcludesSchema = z.object({
  /** Directory names to exclude (exact match) */
  dirNames: z.array(z.string()).default([]),
  /** Directory name suffixes to exclude (e.g. ".egg-info") */
  dirSuffixes: z.array(z.string()).default([]),
  /** File names to exclude (exact match) */
  fileNames: z.array(z.string()).default([]),
  /** File extensions to exclude (e.g. ".safetensors", ".ckpt") */
  extensions: z.array(z.string()).default([]),
  /** Path segments to exclude (e.g. "models", "checkpoints") */
  patterns: z.array(z.string()).default([]),
});

export type GlobalExcludes = z.infer<typeof GlobalExcludesSchema>;

export interface GlobalExcludeExtensionOption {
  value: string;
  label: string;
}

export interface GlobalExcludeDirOption {
  value: string;
  label: string;
}

export interface GlobalExcludeFileOption {
  value: string;
  label: string;
}

export interface GlobalExcludeSuffixOption {
  value: string;
  label: string;
}

export const GLOBAL_EXCLUDE_DIR_OPTIONS: GlobalExcludeDirOption[] = [
  { value: ".cache", label: ".cache/" },
  { value: ".git", label: ".git/" },
  { value: ".mypy_cache", label: ".mypy_cache/" },
  { value: ".next", label: ".next/" },
  { value: ".nyc_output", label: ".nyc_output/" },
  { value: ".nuxt", label: ".nuxt/" },
  { value: ".parcel-cache", label: ".parcel-cache/" },
  { value: ".pytest_cache", label: ".pytest_cache/" },
  { value: ".ruff_cache", label: ".ruff_cache/" },
  { value: ".svelte-kit", label: ".svelte-kit/" },
  { value: ".tox", label: ".tox/" },
  { value: ".turbo", label: ".turbo/" },
  { value: ".venv", label: ".venv/" },
  { value: ".gradle", label: ".gradle/" },
  { value: ".hypothesis", label: ".hypothesis/" },
  { value: ".nox", label: ".nox/" },
  { value: "__pycache__", label: "__pycache__/" },
  { value: "build", label: "build/" },
  { value: "coverage", label: "coverage/" },
  { value: "dist", label: "dist/" },
  { value: "env", label: "env/" },
  { value: "logs", label: "logs/" },
  { value: "node_modules", label: "node_modules/" },
  { value: "out", label: "out/" },
  { value: "target", label: "target/" },
  { value: "venv", label: "venv/" },
];

export const GLOBAL_EXCLUDE_DIR_SUFFIX_OPTIONS: GlobalExcludeSuffixOption[] = [
  { value: ".egg-info", label: ".egg-info/" },
];

export const GLOBAL_EXCLUDE_FILE_OPTIONS: GlobalExcludeFileOption[] = [
  { value: ".ds_store", label: ".DS_Store" },
  { value: ".coverage", label: ".coverage" },
  { value: ".env", label: ".env" },
  { value: ".env.local", label: ".env.local" },
  { value: ".eslintcache", label: ".eslintcache" },
  { value: "thumbs.db", label: "Thumbs.db" },
];

export const GLOBAL_EXCLUDE_SUFFIX_OPTIONS: GlobalExcludeSuffixOption[] = [
  { value: ".bak", label: ".bak" },
  { value: ".log", label: ".log" },
  { value: ".old", label: ".old" },
  { value: ".orig", label: ".orig" },
  { value: ".pyc", label: ".pyc" },
  { value: ".pyd", label: ".pyd" },
  { value: ".pyo", label: ".pyo" },
  { value: ".swo", label: ".swo" },
  { value: ".swp", label: ".swp" },
  { value: ".tmp", label: ".tmp" },
  { value: "~", label: "~" },
];

export const GLOBAL_EXCLUDE_EXTENSION_OPTIONS: GlobalExcludeExtensionOption[] = [
  { value: ".safetensors", label: ".safetensors" },
  { value: ".ckpt", label: ".ckpt" },
  { value: ".pt", label: ".pt" },
  { value: ".pth", label: ".pth" },
  { value: ".bin", label: ".bin" },
  { value: ".onnx", label: ".onnx" },
  { value: ".pb", label: ".pb" },
  { value: ".h5", label: ".h5" },
  { value: ".keras", label: ".keras" },
];

export interface GlobalExcludePatternOption {
  value: string;
  label: string;
}

export const GLOBAL_EXCLUDE_PATTERN_OPTIONS: GlobalExcludePatternOption[] = [
  { value: "models", label: "models/" },
  { value: "checkpoints", label: "checkpoints/" },
  { value: "weights", label: "weights/" },
  { value: ".huggingface", label: ".huggingface/" },
  { value: "huggingface_hub", label: "huggingface_hub/" },
  { value: "wandb", label: "wandb/" },
  { value: "mlruns", label: "mlruns/" },
  { value: "lightning_logs", label: "lightning_logs/" },
];

export const GLOBAL_EXCLUDE_RECOMMENDED_EXTENSIONS = GLOBAL_EXCLUDE_EXTENSION_OPTIONS.map(
  (option) => option.value
);

export const GLOBAL_EXCLUDE_RECOMMENDED_PATTERNS = GLOBAL_EXCLUDE_PATTERN_OPTIONS.map(
  (option) => option.value
);

export const GLOBAL_EXCLUDE_RECOMMENDED_DIRS = GLOBAL_EXCLUDE_DIR_OPTIONS.map(
  (option) => option.value
);

export const GLOBAL_EXCLUDE_RECOMMENDED_DIR_SUFFIXES = GLOBAL_EXCLUDE_DIR_SUFFIX_OPTIONS.map(
  (option) => option.value
);

export const GLOBAL_EXCLUDE_RECOMMENDED_FILES = GLOBAL_EXCLUDE_FILE_OPTIONS.map(
  (option) => option.value
);

export const GLOBAL_EXCLUDE_RECOMMENDED_FILE_SUFFIXES = GLOBAL_EXCLUDE_SUFFIX_OPTIONS.map(
  (option) => option.value
);
