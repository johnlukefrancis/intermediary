// Path: app/src/shared/config/glob_defaults.ts
// Description: Default glob patterns for docs, code, and ignores

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

export const DEFAULT_CODE_GLOBS = [
  "src/**",
  "app/**",
  "crates/**",
  "src-tauri/**",
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
  "**/*.c",
  "**/*.h",
  "**/*.hpp",
  "**/*.cc",
  "**/*.cpp",
  "**/*.cxx",
  "**/*.cs",
  "**/*.java",
  "**/*.kt",
  "**/*.swift",
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
