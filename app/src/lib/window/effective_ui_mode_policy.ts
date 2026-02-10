// Path: app/src/lib/window/effective_ui_mode_policy.ts
// Description: Resolves runtime effective UI mode from preferred mode and window state

import type { UiMode } from "../../shared/config.js";

/** Enter standard mode when handset-preferred width reaches this threshold. */
export const ENTER_STANDARD_WIDTH = 980;
/** Return to handset mode when width contracts to this threshold. */
export const EXIT_HANDSET_WIDTH = 860;

interface ResolveEffectiveUiModeInput {
  preferredMode: UiMode;
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
 * For handset preference, maximize or wide windows temporarily render standard mode.
 * Hysteresis avoids mode flapping near the breakpoint.
 */
export function resolveEffectiveUiMode({
  preferredMode,
  width,
  maximized,
  previousEffectiveMode,
}: ResolveEffectiveUiModeInput): UiMode {
  if (preferredMode === "standard") {
    return "standard";
  }

  if (maximized) {
    return "standard";
  }

  const logicalWidth = normalizeWidth(width);
  if (previousEffectiveMode === "standard") {
    return logicalWidth <= EXIT_HANDSET_WIDTH ? "handset" : "standard";
  }
  return logicalWidth >= ENTER_STANDARD_WIDTH ? "standard" : "handset";
}
