// Path: app/src/lib/theme/accent_utils.ts
// Description: Convert hex accent color to CSS variable values for runtime theming

/** CSS variable values derived from an accent color */
export interface AccentCssVars {
  "--accent-primary": string;
  "--accent-soft": string;
  "--accent-glow": string;
  "--color-accent": string;
  "--color-accent-soft": string;
  "--color-accent-glow": string;
  "--deck-grid-dot": string;
  "--deck-vignette-tint": string;
}

/** Default accent color (dusty rose) */
export const DEFAULT_ACCENT_HEX = "#c4688a";

/** Accent palette for auto-assigned tab themes */
export const ACCENT_PALETTE = [
  "#6fbf9e",
  "#7aa2f7",
  "#d1a04f",
  "#c4688a",
  "#a98bdc",
  "#d0776b",
  "#5fb3b3",
];

interface RgbColor {
  r: number;
  g: number;
  b: number;
}

/**
 * Parse hex color to RGB components.
 * @throws Error if hex format is invalid
 */
function hexToRgb(hex: string): RgbColor {
  const match = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex);
  if (!match) {
    throw new Error(`Invalid hex color: ${hex}`);
  }
  // Regex guarantees exactly 3 capture groups when matched, but TS needs explicit narrowing
  const rHex = match[1];
  const gHex = match[2];
  const bHex = match[3];
  if (!rHex || !gHex || !bHex) {
    throw new Error(`Invalid hex color: ${hex}`);
  }
  return {
    r: parseInt(rHex, 16),
    g: parseInt(gHex, 16),
    b: parseInt(bHex, 16),
  };
}

/**
 * Convert hex accent color to all required CSS variable values.
 *
 * Derived values use fixed opacity levels matching the design system:
 * - soft: 15% opacity (subtle backgrounds)
 * - glow: 40% opacity (highlights, focus rings)
 * - deck-grid-dot: 2.5% opacity (substrate grid)
 * - deck-vignette-tint: 3% opacity (edge vignette)
 */
export function hexToAccentCssVars(hex: string): AccentCssVars {
  const { r, g, b } = hexToRgb(hex);

  const soft = `rgba(${r}, ${g}, ${b}, 0.15)`;
  const glow = `rgba(${r}, ${g}, ${b}, 0.4)`;
  const gridDot = `rgba(${r}, ${g}, ${b}, 0.025)`;
  const vignetteTint = `rgba(${r}, ${g}, ${b}, 0.03)`;

  return {
    "--accent-primary": hex,
    "--accent-soft": soft,
    "--accent-glow": glow,
    "--color-accent": hex,
    "--color-accent-soft": soft,
    "--color-accent-glow": glow,
    "--deck-grid-dot": gridDot,
    "--deck-vignette-tint": vignetteTint,
  };
}
