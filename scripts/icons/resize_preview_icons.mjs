#!/usr/bin/env node
// Path: scripts/icons/resize_preview_icons.mjs
// Description: Resize preview geometry icons from raw assets to display sizes. Outputs 40px (1x) and 80px (2x retina) versions.

/**
 * Resize preview geometry icons from raw assets to display sizes.
 * Outputs 40px (1x) and 80px (2x retina) versions.
 */

import sharp from "sharp";
import { mkdir } from "fs/promises";
import { dirname, join } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, "..");

const ICONS = [
  {
    name: "sphere",
    src: "assets/raw/20260112_1338_Image Generation_simple_compose_01kes6tfvnefztwf0mzjn0ryaq.png",
  },
  {
    name: "cube",
    src: "assets/raw/20260112_1338_Image Generation_simple_compose_01kes6tkr9f80tegmrq71hgnzz.png",
  },
  {
    name: "plane",
    src: "assets/raw/20260112_1340_Image Generation_simple_compose_01kes6xtmzfk193seeg47fmkmp.png",
  },
];

const OUTPUT_DIR = join(ROOT, "assets/public/icons");
const SIZE_1X = 40;
const SIZE_2X = 80;

async function resizeIcon(icon) {
  const srcPath = join(ROOT, icon.src);

  // 1x version
  await sharp(srcPath)
    .resize(SIZE_1X, SIZE_1X, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toFile(join(OUTPUT_DIR, `${icon.name}.png`));

  // 2x version
  await sharp(srcPath)
    .resize(SIZE_2X, SIZE_2X, { fit: "contain", background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toFile(join(OUTPUT_DIR, `${icon.name}@2x.png`));

  console.log(`✓ ${icon.name}: ${SIZE_1X}px + ${SIZE_2X}px`);
}

async function main() {
  await mkdir(OUTPUT_DIR, { recursive: true });
  console.log(`Output: ${OUTPUT_DIR}\n`);

  for (const icon of ICONS) {
    await resizeIcon(icon);
  }

  console.log("\nDone!");
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
