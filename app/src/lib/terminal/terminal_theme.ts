// Path: app/src/lib/terminal/terminal_theme.ts
// Description: Reads deck colour and font tokens into xterm theme and options; an empty token leaves xterm's default in place

import type { ITerminalOptions, ITheme, IWindowsPty } from "@xterm/xterm";

export const TERMINAL_FONT_SIZE_PX = 14;
export const TERMINAL_SCROLLBACK = 10000;

type ThemeColorKey = Exclude<keyof ITheme, "extendedAnsi">;

const THEME_TOKENS: ReadonlyArray<readonly [ThemeColorKey, string]> = [
  ["background", "--terminal-bg"],
  ["foreground", "--terminal-fg"],
  ["cursor", "--terminal-cursor"],
  ["cursorAccent", "--terminal-cursor-accent"],
  ["selectionBackground", "--terminal-selection"],
  ["scrollbarSliderBackground", "--color-accent"],
  ["scrollbarSliderHoverBackground", "--color-accent"],
  ["scrollbarSliderActiveBackground", "--color-accent"],
  ["black", "--terminal-ansi-black"],
  ["red", "--terminal-ansi-red"],
  ["green", "--terminal-ansi-green"],
  ["yellow", "--terminal-ansi-yellow"],
  ["blue", "--terminal-ansi-blue"],
  ["magenta", "--terminal-ansi-magenta"],
  ["cyan", "--terminal-ansi-cyan"],
  ["white", "--terminal-ansi-white"],
  ["brightBlack", "--terminal-ansi-bright-black"],
  ["brightRed", "--terminal-ansi-bright-red"],
  ["brightGreen", "--terminal-ansi-bright-green"],
  ["brightYellow", "--terminal-ansi-bright-yellow"],
  ["brightBlue", "--terminal-ansi-bright-blue"],
  ["brightMagenta", "--terminal-ansi-bright-magenta"],
  ["brightCyan", "--terminal-ansi-bright-cyan"],
  ["brightWhite", "--terminal-ansi-bright-white"],
];

/** The deck root carries the theme and accent tokens; the document root is the pre-mount fallback */
function tokenRoot(): Element {
  return document.querySelector(".app") ?? document.documentElement;
}

let probe: HTMLSpanElement | null = null;

/**
 * A zero-size element inside the deck root whose `color` resolves a token the way the browser
 * does. xterm's parser takes only hex and comma-form `rgb()`/`rgba()`, while the deck tokens are
 * space-separated with a `calc()` alpha; the computed `color` of the probe is the comma form.
 */
function colorProbe(root: Element): HTMLSpanElement {
  if (probe === null || probe.parentElement !== root) {
    probe?.remove();
    probe = document.createElement("span");
    probe.setAttribute("data-terminal-color-probe", "");
    probe.setAttribute("aria-hidden", "true");
    probe.style.cssText = "position:absolute;width:0;height:0;overflow:hidden;visibility:hidden;";
    root.appendChild(probe);
  }
  return probe;
}

/** Every declared deck slot, resolved to a colour xterm accepts; absent slots keep xterm's default */
export function readTerminalTheme(): ITheme {
  const root = tokenRoot();
  const declared = getComputedStyle(root);
  const resolver = colorProbe(root);
  const theme: ITheme = {};
  for (const [key, token] of THEME_TOKENS) {
    if (declared.getPropertyValue(token).trim() === "") continue;
    resolver.style.color = `var(${token})`;
    const resolved = getComputedStyle(resolver).color.trim();
    if (resolved !== "") theme[key] = resolved;
  }
  return theme;
}

export function readTerminalFontFamily(): string | null {
  const value = getComputedStyle(tokenRoot()).getPropertyValue("--font-mono").trim();
  return value === "" ? null : value;
}

/** ConPTY reflow hint; the build number is only known once the pty has opened */
export function buildWindowsPty(buildNumber: number | null): IWindowsPty {
  return { backend: "conpty", ...(buildNumber === null ? {} : { buildNumber }) };
}

export function buildTerminalOptions(buildNumber: number | null): ITerminalOptions {
  const fontFamily = readTerminalFontFamily();
  return {
    // The unicode11 addon registers through `terminal.unicode`, which is proposed API
    allowProposedApi: true,
    allowTransparency: true,
    cursorBlink: true,
    cursorStyle: "bar",
    cursorWidth: 2,
    cursorInactiveStyle: "outline",
    drawBoldTextInBrightColors: true,
    ...(fontFamily === null ? {} : { fontFamily }),
    fontSize: TERMINAL_FONT_SIZE_PX,
    scrollback: TERMINAL_SCROLLBACK,
    theme: readTerminalTheme(),
    windowsPty: buildWindowsPty(buildNumber),
  };
}
