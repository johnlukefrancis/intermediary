// Path: app/src/lib/window/foreground.ts
// Description: Shared predicate for whether this window is truly foreground (visible + focused)

/**
 * True only when the window is genuinely in the foreground: not hidden, its
 * document is visible, and it currently holds focus. A window that is on-screen
 * but sitting behind another app (unfocused) is NOT foreground.
 *
 * Used by the motion governor to pause animation and by the resume detector to
 * gate sleep/wake handling, so both share one definition of "foreground".
 */
export function isForegroundWindow(): boolean {
  return (
    !document.hidden &&
    document.visibilityState === "visible" &&
    document.hasFocus()
  );
}
