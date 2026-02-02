// Path: agent/src/repos/mru_index.ts
// Description: MRU (Most Recently Used) index for recent file changes with unique-by-path semantics

import type { FileEntry } from "../../../app/src/shared/protocol.js";

/**
 * MRU index that maintains unique entries by path with newest-first ordering.
 * Uses Map's insertion order to track recency - delete+insert moves to end.
 */
export interface MruIndex {
  /** Add or update entry; moves to front (most recent) */
  upsert(entry: FileEntry): void;
  /** Remove entry by path; returns true if found */
  remove(path: string): boolean;
  /** Get all entries newest-first, trimmed to capacity */
  entries(): FileEntry[];
  /** Seed the index from persisted entries (oldest-first order expected) */
  loadFrom(entries: FileEntry[]): void;
  /** Current entry count */
  readonly size: number;
  /** Clear all entries */
  clear(): void;
}

/**
 * Create an MRU index with bounded capacity.
 * Entries are unique by path; updating an entry moves it to the front.
 */
export function createMruIndex(capacity: number): MruIndex {
  if (capacity <= 0) {
    throw new Error("MruIndex capacity must be positive");
  }

  // Map maintains insertion order; we treat end as "newest"
  const items = new Map<string, FileEntry>();

  function upsert(entry: FileEntry): void {
    // Delete first to update insertion order (move to end = newest)
    items.delete(entry.path);
    items.set(entry.path, entry);

    // Trim oldest entries if over capacity
    while (items.size > capacity) {
      const firstKey = items.keys().next().value as string;
      items.delete(firstKey);
    }
  }

  function remove(path: string): boolean {
    return items.delete(path);
  }

  function entries(): FileEntry[] {
    // Convert to array and reverse so newest is first
    return Array.from(items.values()).reverse();
  }

  function loadFrom(persistedEntries: FileEntry[]): void {
    items.clear();
    // Persisted entries should be newest-first, so iterate in reverse
    // to add oldest first (so newest ends up at end of Map = front of output)
    for (let i = persistedEntries.length - 1; i >= 0; i--) {
      const entry = persistedEntries[i];
      if (entry && !items.has(entry.path)) {
        items.set(entry.path, entry);
      }
    }
    // Trim to capacity
    while (items.size > capacity) {
      const firstKey = items.keys().next().value as string;
      items.delete(firstKey);
    }
  }

  function clear(): void {
    items.clear();
  }

  return {
    upsert,
    remove,
    entries,
    loadFrom,
    get size() {
      return items.size;
    },
    clear,
  };
}
