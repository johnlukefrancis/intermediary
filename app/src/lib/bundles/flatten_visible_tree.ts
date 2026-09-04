// Path: app/src/lib/bundles/flatten_visible_tree.ts
// Description: Flattens the lazily-loaded ZIPS tree into the exact visible row order the DOM renders

import type { DirectoryListingState } from "../../hooks/bundles/use_directory_listings.js";
import { parentPath } from "./bundle_selection_visibility.js";

export interface VisibleRow {
  path: string;
  kind: "dir" | "file";
  parent: string;
}

/**
 * Order must match the DOM exactly, and the DOM is asymmetric by design: at the root, directories
 * come before files (`bundle_explorer_tree.tsx`); inside an expanded directory, files come before
 * subdirectories (`bundle_explorer_directory.tsx`). Loading/error placeholder rows are not real
 * tree entries, so they are skipped here.
 */
export function flattenVisibleTree(
  topLevelDirs: readonly string[],
  topLevelFiles: readonly string[],
  expandedDirs: ReadonlySet<string>,
  listings: ReadonlyMap<string, DirectoryListingState>
): VisibleRow[] {
  const rows: VisibleRow[] = [];

  function pushDirectory(path: string): void {
    const parent = parentPath(path);
    rows.push({ path, kind: "dir", parent });
    if (!expandedDirs.has(path)) return;
    const listing = listings.get(path);
    if (!listing || listing.status !== "ready") return;
    for (const filePath of listing.files) {
      rows.push({ path: filePath, kind: "file", parent: path });
    }
    for (const dirPath of listing.dirs) {
      pushDirectory(dirPath);
    }
  }

  for (const dirPath of topLevelDirs) {
    pushDirectory(dirPath);
  }
  for (const filePath of topLevelFiles) {
    rows.push({ path: filePath, kind: "file", parent: "" });
  }

  return rows;
}
