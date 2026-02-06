// Path: app/src/shared/config/glob_defaults.ts
// Description: Default glob patterns for docs, code, and ignores

import { GENERATED_CODE_EXTENSION_GLOBS } from "./generated_code_globs.js";

export const DEFAULT_DOCS_GLOBS = [
  "docs/**",
  "**/*.md",
  "**/*.mdx",
  "**/*.txt",
  "**/*.rst",
  "**/*.adoc",
  "**/*.asciidoc",
  "**/*.wiki",
  "**/README*",
];

const CODE_ROOT_GLOBS = [
  "src/**",
  "app/**",
  "crates/**",
  "src-tauri/**",
];

export const LEGACY_DEFAULT_CODE_GLOBS = [
  ...CODE_ROOT_GLOBS,
  "**/*.ts",
  "**/*.tsx",
  "**/*.js",
  "**/*.jsx",
  "**/*.mjs",
  "**/*.cjs",
  "**/*.rs",
  "**/*.toml",
  "**/*.json",
  "**/*.yaml",
  "**/*.yml",
  "**/*.py",
  "**/*.go",
];

export const DEFAULT_CODE_GLOBS = [
  ...CODE_ROOT_GLOBS,
  ...GENERATED_CODE_EXTENSION_GLOBS,
];

export const DEFAULT_IGNORE_GLOBS = [
  "**/node_modules/**",
  "**/.git/**",
  "**/dist/**",
  "**/build/**",
  "**/target/**",
  "**/.cache/**",
  "**/logs/**",
  "**/scripts/zip/output/**",
  "**/scripts/zip/Output/**",
  "**/Scripts/Zip/Output/**",
];
