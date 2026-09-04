// Path: app/src/lib/terminal/terminal_output_scan.ts
// Description: Tells whether a pty output chunk paints anything (text or a line break) once escape sequences are skipped

const ESC = 0x1b;
const BEL = 0x07;
const CR = 0x0d;
const LF = 0x0a;

/**
 * conhost's first bytes are queries and mode changes (the cursor-position request, win32-input
 * mode) that never reach the screen; the STARTING notice stays up until something does.
 * CSI sequences end at 0x40..0x7e, OSC/DCS/APC strings at BEL or ESC \\, and any other ESC
 * takes one following byte.
 */
export function hasVisibleOutput(bytes: Uint8Array): boolean {
  let index = 0;
  while (index < bytes.length) {
    const byte = bytes[index] ?? 0;
    if (byte !== ESC) {
      if (byte >= 0x20 || byte === CR || byte === LF) return true;
      index += 1;
      continue;
    }
    const kind = bytes[index + 1];
    if (kind === undefined) return false;
    if (kind === 0x5b) {
      // CSI: parameters and intermediates until a final byte
      index += 2;
      while (index < bytes.length && (bytes[index] ?? 0) < 0x40) index += 1;
      index += 1;
      continue;
    }
    if (kind === 0x5d || kind === 0x50 || kind === 0x5f) {
      // OSC / DCS / APC: string until BEL or ESC \\
      index += 2;
      while (index < bytes.length) {
        const current = bytes[index] ?? 0;
        if (current === BEL) { index += 1; break; }
        if (current === ESC && bytes[index + 1] === 0x5c) { index += 2; break; }
        index += 1;
      }
      continue;
    }
    index += 2;
  }
  return false;
}
