// Path: scripts/build/build_agent_bundle.mjs
// Description: Build the bundled WSL agent runtime and copy im_bundle_cli into Tauri resources

import { build } from "esbuild";
import { mkdir, readFile, writeFile, copyFile, stat } from "node:fs/promises";
import path from "node:path";

const repoRoot = process.cwd();
const outputDir = path.join(repoRoot, "src-tauri", "resources", "agent_bundle");
const entryPoint = path.join(repoRoot, "agent", "src", "main.ts");
const outputFile = path.join(outputDir, "agent_main.cjs");

const cliCandidates = [
  path.join(repoRoot, "target", "release", "im_bundle_cli"),
  path.join(repoRoot, "target", "debug", "im_bundle_cli"),
];

async function resolveCliPath() {
  for (const candidate of cliCandidates) {
    try {
      const info = await stat(candidate);
      if (info.isFile()) {
        return candidate;
      }
    } catch {
      // Try next candidate.
    }
  }
  return null;
}

async function loadVersion() {
  const pkgPath = path.join(repoRoot, "package.json");
  const pkg = JSON.parse(await readFile(pkgPath, "utf8"));
  const version = typeof pkg.version === "string" ? pkg.version.trim() : "";
  if (!version) {
    throw new Error("package.json version is missing");
  }
  return version;
}

async function main() {
  await mkdir(outputDir, { recursive: true });

  await build({
    entryPoints: [entryPoint],
    outfile: outputFile,
    bundle: true,
    platform: "node",
    format: "cjs",
    target: "node18",
    sourcemap: false,
  });

  const cliPath = await resolveCliPath();
  if (!cliPath) {
    throw new Error(
      "im_bundle_cli not found. Build the Rust CLI before bundling the agent."
    );
  }

  await copyFile(cliPath, path.join(outputDir, "im_bundle_cli"));

  const version = await loadVersion();
  await writeFile(
    path.join(outputDir, "version.json"),
    JSON.stringify({ version }, null, 2) + "\n",
    "utf8"
  );
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
