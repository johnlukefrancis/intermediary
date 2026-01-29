#!/usr/bin/env node
// Path: scripts/icons/generate_icons.mjs
// Description: Generate all icon sizes from a source PNG. Usage: node scripts/generate_icons.mjs [source.png] Default source: app/as...

/**
 * Generate all icon sizes from a source PNG.
 * Usage: node scripts/generate_icons.mjs [source.png]
 * Default source: app/assets/icon.png
 */

import sharp from 'sharp';
import { mkdir, writeFile } from 'fs/promises';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '../..');

const SOURCE = process.argv[2] || join(ROOT, 'app/assets/icon.png');
const OUT_DIR = join(ROOT, 'src-tauri/icons');

// All sizes needed for Tauri (Windows + macOS + Linux)
const SIZES = [
  { name: '32x32.png', size: 32 },
  { name: '128x128.png', size: 128 },
  { name: '128x128@2x.png', size: 256 },
  { name: 'icon.png', size: 512 },
  // Windows Store logos
  { name: 'Square30x30Logo.png', size: 30 },
  { name: 'Square44x44Logo.png', size: 44 },
  { name: 'Square71x71Logo.png', size: 71 },
  { name: 'Square89x89Logo.png', size: 89 },
  { name: 'Square107x107Logo.png', size: 107 },
  { name: 'Square142x142Logo.png', size: 142 },
  { name: 'Square150x150Logo.png', size: 150 },
  { name: 'Square284x284Logo.png', size: 284 },
  { name: 'Square310x310Logo.png', size: 310 },
  { name: 'StoreLogo.png', size: 50 },
];

// ICO sizes (Windows)
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];

async function generatePngs(sourceBuffer) {
  console.log(`Output: ${OUT_DIR}\n`);

  await mkdir(OUT_DIR, { recursive: true });

  for (const { name, size } of SIZES) {
    const out = join(OUT_DIR, name);
    await sharp(sourceBuffer)
      .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
      .png()
      .toFile(out);
    console.log(`  ${name} (${size}x${size})`);
  }
}

async function generateIco(sourceBuffer) {
  const images = await Promise.all(
    ICO_SIZES.map(async (size) => {
      const buf = await sharp(sourceBuffer)
        .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
        .png()
        .toBuffer();
      return { size, data: buf };
    })
  );

  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);

  const dirEntries = Buffer.alloc(16 * images.length);
  let offset = 6 + 16 * images.length;

  images.forEach((img, i) => {
    const entry = dirEntries.subarray(i * 16, (i + 1) * 16);
    entry.writeUInt8(img.size >= 256 ? 0 : img.size, 0);
    entry.writeUInt8(img.size >= 256 ? 0 : img.size, 1);
    entry.writeUInt8(0, 2);
    entry.writeUInt8(0, 3);
    entry.writeUInt16LE(1, 4);
    entry.writeUInt16LE(32, 6);
    entry.writeUInt32LE(img.data.length, 8);
    entry.writeUInt32LE(offset, 12);
    offset += img.data.length;
  });

  const ico = Buffer.concat([header, dirEntries, ...images.map((img) => img.data)]);
  await writeFile(join(OUT_DIR, 'icon.ico'), ico);
  console.log(`  icon.ico (${ICO_SIZES.join(', ')})`);
}

async function generateIcns(sourceBuffer) {
  const icnsTypes = [
    { type: 'icp4', size: 16 },
    { type: 'icp5', size: 32 },
    { type: 'icp6', size: 64 },
    { type: 'ic07', size: 128 },
    { type: 'ic08', size: 256 },
    { type: 'ic09', size: 512 },
    { type: 'ic10', size: 1024 },
  ];

  const images = await Promise.all(
    icnsTypes.map(async ({ type, size }) => {
      const data = await sharp(sourceBuffer)
        .resize(size, size, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
        .png()
        .toBuffer();
      return { type, data };
    })
  );

  let totalSize = 8;
  for (const img of images) {
    totalSize += 8 + img.data.length;
  }

  const icns = Buffer.alloc(totalSize);
  icns.write('icns', 0);
  icns.writeUInt32BE(totalSize, 4);

  let offset = 8;
  for (const img of images) {
    icns.write(img.type, offset);
    icns.writeUInt32BE(8 + img.data.length, offset + 4);
    img.data.copy(icns, offset + 8);
    offset += 8 + img.data.length;
  }

  await writeFile(join(OUT_DIR, 'icon.icns'), icns);
  console.log(`  icon.icns (${icnsTypes.map((t) => t.size).join(', ')})`);
}

async function main() {
  console.log('Generating icons...\n');
  console.log(`Source: ${SOURCE}`);

  const source = await sharp(SOURCE).png().toBuffer();
  console.log('');

  await generatePngs(source);
  await generateIco(source);
  await generateIcns(source);

  console.log('\nDone!');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
