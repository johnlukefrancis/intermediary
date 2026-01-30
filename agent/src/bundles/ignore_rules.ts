// Path: agent/src/bundles/ignore_rules.ts
// Description: Centralized ignore patterns for bundle building

/**
 * Directories that are always excluded from bundles
 */
export const ALWAYS_IGNORE_DIRS = new Set([
  "node_modules",
  ".git",
  "dist",
  "build",
  "target",
  ".next",
  ".cache",
  "logs",
  ".turbo",
  ".vercel",
  "__pycache__",
  ".mypy_cache",
  ".pytest_cache",
  "coverage",
]);

/**
 * File patterns that are always excluded from bundles
 */
export const ALWAYS_IGNORE_FILES = new Set([
  ".DS_Store",
  "Thumbs.db",
  ".env",
  ".env.local",
]);

/**
 * Check if an entry should be ignored during bundle building
 */
export function shouldIgnoreEntry(name: string, isDirectory: boolean): boolean {
  // Skip hidden files/dirs (starting with .)
  if (name.startsWith(".") && name !== ".") {
    // Allow certain dotfiles if needed in future
    return true;
  }

  if (isDirectory) {
    return ALWAYS_IGNORE_DIRS.has(name);
  }

  return ALWAYS_IGNORE_FILES.has(name);
}
