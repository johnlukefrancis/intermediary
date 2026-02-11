// Path: app/src/lib/window/effective_ui_mode_policy.ts
// Description: Resolves runtime effective UI mode from preferred mode and window state

import type { UiMode } from "../../shared/config.js";

/** Enter standard mode when handset-width transitions to desktop width. */
export const ENTER_STANDARD_WIDTH = 980;
/** Return to handset mode when desktop-width transitions to handset width. */
export const EXIT_HANDSET_WIDTH = 860;

interface ResolveEffectiveUiModeInput {
  width: number;
  maximized: boolean;
  previousEffectiveMode: UiMode;
}

function normalizeWidth(width: number): number {
  if (!Number.isFinite(width) || width <= 0) {
    return 0;
  }
  return Math.round(width);
}

/**
 * Runtime mode follows window geometry with hysteresis:
 * - maximized windows always render standard
 * - width thresholds avoid flapping near breakpoints
 */
export function resolveEffectiveUiMode({
  width,
  maximized,
  previousEffectiveMode,
}: ResolveEffectiveUiModeInput): UiMode {
  if (maximized) {
    return "standard";
  }

  const logicalWidth = normalizeWidth(width);
  if (previousEffectiveMode === "standard") {
    return logicalWidth <= EXIT_HANDSET_WIDTH ? "handset" : "standard";
  }
  return logicalWidth >= ENTER_STANDARD_WIDTH ? "standard" : "handset";
}
