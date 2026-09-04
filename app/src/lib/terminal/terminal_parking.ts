// Path: app/src/lib/terminal/terminal_parking.ts
// Description: Off-screen parking host for terminal elements that are alive but not shown; sized so xterm open() and fit() still measure

let host: HTMLDivElement | null = null;

/**
 * Off-screen (not merely hidden) so xterm's IntersectionObserver pauses rendering, with a
 * definite size so a session created while parked measures real cell dimensions.
 */
const PARKING_STYLE =
  "position:fixed;left:-10000px;top:0;width:960px;height:600px;overflow:hidden;" +
  "pointer-events:none;contain:strict;";

export function terminalParkingHost(): HTMLDivElement {
  if (host !== null && host.isConnected) return host;
  const created = document.createElement("div");
  created.setAttribute("data-terminal-parking", "");
  created.setAttribute("aria-hidden", "true");
  created.inert = true;
  created.style.cssText = PARKING_STYLE;
  document.body.appendChild(created);
  host = created;
  return created;
}

/** Moves the element into the parking host (no-op when it is already there) */
export function parkElement(element: HTMLElement): void {
  const parking = terminalParkingHost();
  if (element.parentElement !== parking) parking.appendChild(element);
}
