// Path: app/src/hooks/bundles/use_tree_drop_import.ts
// Description: Owns the OS drag gesture over the ZIPS tree: hit-testing, dwell-expand, edge auto-scroll, self-drag latch

import { useEffect, useRef, useState, type RefObject } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useAgent } from "../use_agent.js";
import { applyEdgeAutoScroll, resolveDropDir, useDropTargetDwell } from "./tree_drop_targeting.js";

export interface UseTreeDropImportOptions {
  listRef: RefObject<HTMLDivElement>;
  expandedDirs: ReadonlySet<string>;
  expandDirectory: (path: string) => void;
  onImport: (directory: string, sources: string[]) => void;
  importInFlight: boolean;
}

export interface TreeDropImportState {
  dropTargetDir: string | null;
  isDragActive: boolean;
}

function normalizeForCompare(path: string): string {
  return path.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
}

/** Case-insensitive containment after normalising separators; an empty root matches nothing. */
function isUnderRoot(path: string, root: string): boolean {
  const normalizedRoot = normalizeForCompare(root);
  if (normalizedRoot.length === 0) return false;
  const normalizedPath = normalizeForCompare(path);
  return normalizedPath === normalizedRoot || normalizedPath.startsWith(`${normalizedRoot}/`);
}

/**
 * The app's own pointer-driven drag-out (`@crabnebula/tauri-plugin-drag`) re-enters this window
 * as a native OS drag; on Windows its enter/over/drop burst arrives stale after the drag ends, so
 * this hook latches for the whole gesture the moment any staged source is seen on `enter`.
 */
export function useTreeDropImport({
  listRef,
  expandedDirs,
  expandDirectory,
  onImport,
  importInFlight,
}: UseTreeDropImportOptions): TreeDropImportState {
  const { appPaths } = useAgent();
  const { dropTargetDir, setTarget, reset } = useDropTargetDwell({ expandedDirs, expandDirectory });
  const [isDragActive, setIsDragActive] = useState(false);

  const latchedRef = useRef(false);
  const dropTargetRef = useRef<string | null>(null);
  const onImportRef = useRef(onImport);
  const importInFlightRef = useRef(importInFlight);
  const stagingRootRef = useRef(appPaths?.stagingHostRoot);

  useEffect(() => { dropTargetRef.current = dropTargetDir; }, [dropTargetDir]);
  useEffect(() => { onImportRef.current = onImport; }, [onImport]);
  useEffect(() => { importInFlightRef.current = importInFlight; }, [importInFlight]);
  useEffect(() => { stagingRootRef.current = appPaths?.stagingHostRoot; }, [appPaths]);

  useEffect(() => {
    const resetGesture = (): void => {
      latchedRef.current = false;
      reset();
      setIsDragActive(false);
    };

    let unlisten: (() => void) | null = null;
    let cancelled = false;

    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        // WebView2 position is client-area (viewport) physical px; devicePixelRatio (not the
        // webview scale factor) converts to CSS px, since it also reflects webview zoom.
        const scaleFactor = window.devicePixelRatio || 1;

        if (payload.type === "enter") {
          const root = stagingRootRef.current;
          const isSelfDrag =
            root !== undefined && payload.paths.some((path) => isUnderRoot(path, root));
          if (isSelfDrag) {
            latchedRef.current = true;
            return;
          }
          latchedRef.current = false;
          setIsDragActive(true);
          setTarget(resolveDropDir(payload.position.x / scaleFactor, payload.position.y / scaleFactor));
          return;
        }

        if (payload.type === "over") {
          if (latchedRef.current) return;
          const cssX = payload.position.x / scaleFactor;
          const cssY = payload.position.y / scaleFactor;
          setTarget(resolveDropDir(cssX, cssY));
          applyEdgeAutoScroll(listRef, cssY);
          return;
        }

        if (payload.type === "drop") {
          if (latchedRef.current) {
            resetGesture();
            return;
          }
          const cssX = payload.position.x / scaleFactor;
          const cssY = payload.position.y / scaleFactor;
          const target = resolveDropDir(cssX, cssY) ?? dropTargetRef.current;
          if (target !== null && !importInFlightRef.current) {
            onImportRef.current(target, payload.paths);
          }
          resetGesture();
          return;
        }

        // "leave"
        resetGesture();
      })
      .then((fn) => {
        if (cancelled) {
          fn();
          return;
        }
        unlisten = fn;
      })
      .catch((error: unknown) => {
        console.error("[useTreeDropImport] failed to subscribe to drag-drop events", error);
      });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [listRef, reset, setTarget]);

  return { dropTargetDir, isDragActive };
}
