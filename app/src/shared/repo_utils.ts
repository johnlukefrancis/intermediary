// Path: app/src/shared/repo_utils.ts
// Description: Utility functions for repo ID generation and path handling

/**
 * Slugify a folder name for use as repoId.
 * Converts to lowercase and replaces non-alphanumeric chars with hyphens.
 */
export function slugifyFolderName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

/**
 * Generate a unique repoId, appending -2, -3 etc. if the base slug already exists.
 */
export function generateUniqueRepoId(
  baseName: string,
  existingIds: Set<string>
): string {
  const baseSlug = slugifyFolderName(baseName) || "repo";
  if (!existingIds.has(baseSlug)) return baseSlug;

  let counter = 2;
  while (existingIds.has(`${baseSlug}-${counter}`)) {
    counter++;
  }
  return `${baseSlug}-${counter}`;
}

/**
 * Extract folder basename from a Windows or Unix path.
 */
export function extractFolderName(path: string): string {
  // Normalize slashes and remove trailing slashes
  const normalized = path.replace(/\\/g, "/").replace(/\/+$/, "");
  const parts = normalized.split("/");
  return parts[parts.length - 1] || "repo";
}
