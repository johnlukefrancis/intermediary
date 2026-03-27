// Path: scripts/release/stage_windows_release_assets.mjs
// Description: Collect Windows bundle outputs into a release artifact directory with sha256 files.

import { createHash } from "node:crypto";
import {
  access,
  copyFile,
  mkdir,
  readdir,
  readFile,
  rm,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const outputRoot = path.join(repoRoot, "artifacts", "windows-release");
const allowedExtensions = new Set([".exe", ".msi", ".zip", ".msix", ".appx"]);
const bundleRootCandidates = [
  path.join(repoRoot, "target", "release", "bundle"),
  path.join(repoRoot, "src-tauri", "target", "release", "bundle"),
];

async function main() {
  const bundleRoot = await resolveBundleRoot();
  const sourceFiles = await collectReleaseFiles(bundleRoot);
  if (sourceFiles.length === 0) {
    throw new Error(
      `No Windows release artifacts were found under ${bundleRoot}.`
    );
  }

  await rm(outputRoot, { recursive: true, force: true });

  const manifest = [];
  for (const sourcePath of sourceFiles) {
    const relativePath = path.relative(bundleRoot, sourcePath);
    const stagedPath = path.join(outputRoot, relativePath);

    await mkdir(path.dirname(stagedPath), { recursive: true });
    await copyFile(sourcePath, stagedPath);

    const sha256 = await hashFile(stagedPath);
    await writeFile(
      `${stagedPath}.sha256`,
      `${sha256}  ${path.basename(stagedPath)}\n`,
      "utf8"
    );

    manifest.push({
      path: relativePath.split(path.sep).join("/"),
      sha256,
    });
  }

  await writeFile(
    path.join(outputRoot, "release_assets.json"),
    `${JSON.stringify({ assets: manifest }, null, 2)}\n`,
    "utf8"
  );

  console.log(`Staged ${manifest.length} Windows release artifact(s) in ${outputRoot}`);
  for (const asset of manifest) {
    console.log(`- ${asset.path}`);
  }
}

async function resolveBundleRoot() {
  for (const candidate of bundleRootCandidates) {
    try {
      await access(candidate);
      return candidate;
    } catch {
      // Try the next known Tauri bundle location.
    }
  }

  throw new Error(
    [
      "No Windows release bundle directory was found.",
      "Checked:",
      ...bundleRootCandidates.map((candidate) => `- ${candidate}`),
    ].join("\n")
  );
}

async function collectReleaseFiles(rootDir) {
  const files = [];
  await walk(rootDir, files);
  return files.sort();
}

async function walk(currentDir, files) {
  const entries = await readdir(currentDir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(currentDir, entry.name);
    if (entry.isDirectory()) {
      await walk(fullPath, files);
      continue;
    }

    if (!allowedExtensions.has(path.extname(entry.name).toLowerCase())) {
      continue;
    }

    files.push(fullPath);
  }
}

async function hashFile(filePath) {
  const contents = await readFile(filePath);
  return createHash("sha256").update(contents).digest("hex");
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
