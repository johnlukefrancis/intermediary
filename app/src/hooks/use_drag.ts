// Path: app/src/hooks/use_drag.ts
// Description: Drag-out logic with on-demand staging

import { useCallback, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { useAgent } from "./use_agent.js";
import { sendStageFile } from "../lib/agent/messages.js";
import { isStagingNotConfiguredError } from "../lib/agent/error_codes.js";
import type { StagedInfo } from "../shared/protocol.js";

export interface FileForDrag {
  path: string;
  stagedInfo: StagedInfo | undefined;
}

export interface DragState {
  isDragging: boolean;
  isStaging: boolean;
  error: string | null;
}

const INITIAL_DRAG_STATE: DragState = {
  isDragging: false,
  isStaging: false,
  error: null,
};

export interface UseDragOptions {
  onStaged?: (relativePath: string, stagedInfo: StagedInfo) => void;
}

export interface UseDragResult {
  dragState: DragState;
  handleDragStart: (
    repoId: string,
    relativePath: string,
    stagedInfo: StagedInfo | undefined
  ) => Promise<void>;
  handleMultiDragStart: (
    repoId: string,
    files: FileForDrag[]
  ) => Promise<void>;
  clearError: () => void;
}

export function useDrag(options?: UseDragOptions): UseDragResult {
  const { client, appPaths, helloState, resyncClientHello } = useAgent();
  const [dragState, setDragState] = useState<DragState>(INITIAL_DRAG_STATE);
  const onStaged = options?.onStaged;

  const clearError = useCallback(() => {
    setDragState((prev) => ({ ...prev, error: null }));
  }, []);

  const handleDragStart = useCallback(
    async (
      repoId: string,
      relativePath: string,
      stagedInfo: StagedInfo | undefined
    ): Promise<void> => {
      if (!client || !appPaths) {
        setDragState({ isDragging: false, isStaging: false, error: "Not connected" });
        return;
      }
      if (helloState.status !== "ok") {
        setDragState({
          isDragging: false,
          isStaging: false,
          error: "Agent session initializing; retry in a moment.",
        });
        return;
      }

      setDragState({ isDragging: true, isStaging: false, error: null });

      try {
        let hostPath: string;

        if (stagedInfo?.hostPath) {
          hostPath = stagedInfo.hostPath;
        } else {
          // Need to stage the file first
          setDragState({ isDragging: true, isStaging: true, error: null });
          let result;
          try {
            result = await sendStageFile(client, repoId, relativePath);
          } catch (err) {
            if (!isStagingNotConfiguredError(err)) {
              throw err;
            }
            const resynced = await resyncClientHello();
            if (!resynced) {
              throw err;
            }
            result = await sendStageFile(client, repoId, relativePath);
          }
          const stagedResult: StagedInfo = {
            hostPath: result.hostPath,
            wslPath: result.wslPath,
            bytesCopied: result.bytesCopied,
            mtimeMs: result.mtimeMs,
          };
          onStaged?.(relativePath, stagedResult);
          hostPath = stagedResult.hostPath;
        }

        await startDrag({
          item: [hostPath],
          icon: appPaths.dragIconHostPath,
        });

        setDragState(INITIAL_DRAG_STATE);
      } catch (err) {
        const message = err instanceof Error ? err.message : "Drag failed";
        setDragState({ isDragging: false, isStaging: false, error: message });
      }
    },
    [appPaths, client, helloState.status, onStaged, resyncClientHello]
  );

  const handleMultiDragStart = useCallback(
    async (repoId: string, files: FileForDrag[]): Promise<void> => {
      if (!client || !appPaths) {
        setDragState({ isDragging: false, isStaging: false, error: "Not connected" });
        return;
      }
      if (helloState.status !== "ok") {
        setDragState({
          isDragging: false,
          isStaging: false,
          error: "Agent session initializing; retry in a moment.",
        });
        return;
      }
      if (files.length === 0) return;

      setDragState({ isDragging: true, isStaging: true, error: null });

      try {
        const results = await Promise.allSettled(
          files.map(async (f) => {
            if (f.stagedInfo?.hostPath) return { path: f.path, hostPath: f.stagedInfo.hostPath };
            let result;
            try {
              result = await sendStageFile(client, repoId, f.path);
            } catch (err) {
              if (!isStagingNotConfiguredError(err)) {
                throw err;
              }
              const resynced = await resyncClientHello();
              if (!resynced) {
                throw err;
              }
              result = await sendStageFile(client, repoId, f.path);
            }
            const stagedResult: StagedInfo = {
              hostPath: result.hostPath,
              wslPath: result.wslPath,
              bytesCopied: result.bytesCopied,
              mtimeMs: result.mtimeMs,
            };
            onStaged?.(f.path, stagedResult);
            return { path: f.path, hostPath: stagedResult.hostPath };
          })
        );

        const hostPaths: string[] = [];
        for (const r of results) {
          if (r.status === "fulfilled") {
            hostPaths.push(r.value.hostPath);
          } else {
            console.warn("[useDrag] staging failed for a file:", r.reason);
          }
        }

        if (hostPaths.length === 0) {
          setDragState({ isDragging: false, isStaging: false, error: "All files failed to stage" });
          return;
        }

        await startDrag({
          item: hostPaths,
          icon: appPaths.dragIconHostPath,
        });

        setDragState(INITIAL_DRAG_STATE);
      } catch (err) {
        const message = err instanceof Error ? err.message : "Multi-drag failed";
        setDragState({ isDragging: false, isStaging: false, error: message });
      }
    },
    [appPaths, client, helloState.status, onStaged, resyncClientHello]
  );

  return {
    dragState,
    handleDragStart,
    handleMultiDragStart,
    clearError,
  };
}
