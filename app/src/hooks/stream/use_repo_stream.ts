// Path: app/src/hooks/stream/use_repo_stream.ts
// Description: Binds the active repo's Stream store to React: visibility, selection filter, card and tile actions

import { useCallback, useEffect, useMemo, useSyncExternalStore } from "react";
import { isPreviewImagePath } from "../repo_workspace_types.js";
import { isFileIncluded } from "../../lib/bundles/bundle_selection_visibility.js";
import { streamStoreRegistry } from "../../lib/stream/stream_store_registry.js";
import type { StreamRingCard, StreamSnapshot, StreamStripTile } from "../../lib/stream/stream_types.js";
import type { BundleSelection, SourceControlChange, SourceControlEntry } from "../../shared/protocol.js";

export interface RepoStreamOptions {
  /** Stream mode, no workspace open, and (on the handset) the FILES section */
  visible: boolean;
  bundleSelection: BundleSelection | null;
  openFile: (path: string) => void;
  openDiff: (entry: SourceControlEntry) => void;
  onDragStart: (path: string) => void | Promise<void>;
}

/** What a double-click, Enter, or OPEN DIFF names: a ring card, or one tile of an image strip */
export type StreamOpenTarget = StreamRingCard | { kind: "tile"; tile: StreamStripTile };

export interface RepoStream {
  snapshot: StreamSnapshot;
  expand: (id: number) => void;
  /** The diff workspace for tracked edits with content, the file otherwise; a strip itself opens nothing */
  openCard: (target: StreamOpenTarget) => void;
  dragCard: (path: string) => void;
}

function changeFor(op: "modify" | "remove" | "rename"): SourceControlChange {
  switch (op) {
    case "modify":
      return "modified";
    case "remove":
      return "deleted";
    case "rename":
      return "renamed";
  }
}

export function useRepoStream(repoId: string, options: RepoStreamOptions): RepoStream {
  const { visible, bundleSelection, openFile, openDiff, onDragStart } = options;
  const store = useMemo(() => streamStoreRegistry.getOrCreate(repoId), [repoId]);
  const snapshot = useSyncExternalStore(store.subscribe, store.getSnapshot);

  useEffect(() => {
    streamStoreRegistry.setVisibleRepo(repoId);
    return () => {
      store.setVisible(false);
      streamStoreRegistry.setVisibleRepo(null);
    };
  }, [repoId, store]);

  useEffect(() => { store.setVisible(visible); }, [store, visible]);

  useEffect(() => {
    store.setSelectionFilter(bundleSelection === null ? null : (path) => isFileIncluded(path, bundleSelection));
  }, [bundleSelection, store]);

  const expand = useCallback((id: number) => { store.expand(id); }, [store]);

  /** A tile opens the viewer, or the image diff for a modified tracked preview image */
  const openTile = useCallback(
    (tile: StreamStripTile) => {
      const diffable = tile.tracked !== false && tile.body.status !== "gone" && isPreviewImagePath(tile.path);
      if (tile.op === "add" || !diffable) {
        openFile(tile.path);
        return;
      }
      openDiff({ path: tile.path, area: "worktree", change: changeFor(tile.op) });
    },
    [openDiff, openFile]
  );

  const openCard = useCallback(
    (target: StreamOpenTarget) => {
      if (target.kind === "burst" || target.kind === "images") return;
      if (target.kind === "tile") {
        openTile(target.tile);
        return;
      }
      if (target.kind === "history") {
        openFile(target.path);
        return;
      }
      // A diff needs a tracked baseline and content on at least one side
      const diffable = target.tracked !== false && target.body.status !== "opaque" && target.body.status !== "gone";
      if (target.op === "add" || !diffable) {
        openFile(target.path);
        return;
      }
      const entry: SourceControlEntry = { path: target.path, area: "worktree", change: changeFor(target.op) };
      openDiff(target.fromPath === null ? entry : { ...entry, originalPath: target.fromPath });
    },
    [openDiff, openFile, openTile]
  );

  const dragCard = useCallback((path: string) => { void onDragStart(path); }, [onDragStart]);

  return { snapshot, expand, openCard, dragCard };
}
