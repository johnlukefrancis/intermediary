// Path: agent/src/util/categorizer.ts
// Description: File kind classification (docs/code/other) based on path patterns

import type { FileKind } from "../../../app/src/shared/protocol.js";

/** Extensions considered documentation */
const DOC_EXTENSIONS = new Set([
  ".md",
  ".txt",
  ".rst",
  ".adoc",
  ".asciidoc",
  ".wiki",
]);

/** Directories that indicate documentation */
const DOC_DIRS = new Set([
  "docs",
  "doc",
  "documentation",
  "wiki",
]);

/** Extensions considered code */
const CODE_EXTENSIONS = new Set([
  ".ts",
  ".tsx",
  ".js",
  ".jsx",
  ".mjs",
  ".cjs",
  ".json",
  ".rs",
  ".toml",
  ".css",
  ".scss",
  ".html",
  ".svelte",
  ".vue",
  ".py",
  ".go",
  ".yaml",
  ".yml",
]);

/**
 * Classify a file path into docs, code, or other.
 */
export function categorizeFile(relativePath: string): FileKind {
  const lower = relativePath.toLowerCase();
  const parts = lower.split("/");
  const fileName = parts[parts.length - 1] ?? "";

  // Check if in a docs directory
  for (const part of parts.slice(0, -1)) {
    if (DOC_DIRS.has(part)) {
      return "docs";
    }
  }

  // Check file extension
  const extMatch = /\.[^./]+$/.exec(fileName);
  const ext = extMatch?.[0] ?? "";

  if (DOC_EXTENSIONS.has(ext)) {
    return "docs";
  }

  if (CODE_EXTENSIONS.has(ext)) {
    return "code";
  }

  // Special files
  if (fileName === "readme" || fileName.startsWith("readme.")) {
    return "docs";
  }

  return "other";
}

/**
 * Returns true if the file kind should be auto-staged.
 */
export function shouldAutoStage(kind: FileKind): boolean {
  return kind === "docs" || kind === "code";
}
