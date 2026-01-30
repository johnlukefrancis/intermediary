// Path: app/src/hooks/use_drag.ts
// Description: Drag-out logic with on-demand staging

import { useCallback, useState } from "react";
import { startDrag } from "@crabnebula/tauri-plugin-drag";
import { useAgent } from "./use_agent.js";
import { sendStageFile } from "../lib/agent/messages.js";
import type { StagedInfo } from "../shared/protocol.js";

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
  clearError: () => void;
}

export function useDrag(options?: UseDragOptions): UseDragResult {
  const { client, appPaths } = useAgent();
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

      setDragState({ isDragging: true, isStaging: false, error: null });

      try {
        let windowsPath: string;

        if (stagedInfo?.windowsPath) {
          windowsPath = stagedInfo.windowsPath;
        } else {
          // Need to stage the file first
          setDragState({ isDragging: true, isStaging: true, error: null });
          const result = await sendStageFile(client, repoId, relativePath);
          const stagedResult: StagedInfo = {
            wslPath: result.wslPath,
            windowsPath: result.windowsPath,
            bytesCopied: result.bytesCopied,
            mtimeMs: result.mtimeMs,
          };
          onStaged?.(relativePath, stagedResult);
          windowsPath = stagedResult.windowsPath;
        }

        await startDrag({
          item: [windowsPath],
          icon: appPaths.dragIconWindowsPath,
        });

        setDragState(INITIAL_DRAG_STATE);
      } catch (err) {
        const message = err instanceof Error ? err.message : "Drag failed";
        setDragState({ isDragging: false, isStaging: false, error: message });
      }
    },
    [client, appPaths, onStaged]
  );

  return {
    dragState,
    handleDragStart,
    clearError,
  };
}
