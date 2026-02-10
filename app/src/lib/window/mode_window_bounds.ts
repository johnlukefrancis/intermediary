// Path: app/src/lib/window/mode_window_bounds.ts
// Description: Shared per-mode window bounds defaults, clamping, and resolution helpers

import type {
  UiMode,
  UiWindowBounds,
  UiWindowBoundsByMode,
} from "../../shared/config.js";

export const MIN_WINDOW_WIDTH = 360;
export const MIN_WINDOW_HEIGHT = 500;
export const MAX_WINDOW_WIDTH = 8192;
export const MAX_WINDOW_HEIGHT = 8192;

export const MODE_WINDOW_DEFAULT_BOUNDS: Readonly<Record<UiMode, UiWindowBounds>> = {
  standard: { width: 1200, height: 800 },
  compact: { width: 1200, height: 800 },
  handset: { width: 420, height: 660 },
};

export function clampWindowBounds(bounds: UiWindowBounds): UiWindowBounds {
  return {
    width: Math.max(
      MIN_WINDOW_WIDTH,
      Math.min(MAX_WINDOW_WIDTH, Math.round(bounds.width))
    ),
    height: Math.max(
      MIN_WINDOW_HEIGHT,
      Math.min(MAX_WINDOW_HEIGHT, Math.round(bounds.height))
    ),
  };
}

export function resolveModeWindowBounds(
  mode: UiMode,
  byMode: UiWindowBoundsByMode
): UiWindowBounds {
  const bounds = byMode[mode] ?? MODE_WINDOW_DEFAULT_BOUNDS[mode];
  return clampWindowBounds(bounds);
}

export function areWindowBoundsEqual(
  left: UiWindowBounds,
  right: UiWindowBounds
): boolean {
  return left.width === right.width && left.height === right.height;
}
