// Path: scripts/build/build_agent_bundle.mjs
// Description: Build the bundled WSL agent binary and sync it into Tauri resources.

import { chmod, copyFile, mkdir, readFile, rename, rm, stat, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";

const repoRoot = process.cwd();
const outputDir = path.join(repoRoot, "src-tauri", "resources", "agent_bundle");
const tempDir = `${outputDir}.tmp`;
const binaryName = "im_agent";
const binaryPath = path.join(repoRoot, "target", "release", binaryName);

function ensureLinux() {
  if (process.platform !== "linux") {
    throw new Error(
      "build_agent_bundle.mjs must run in WSL/Linux so the Linux agent binary can be produced."
    );
  }
}

function runCommand(command, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: "inherit", ...options });
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(" ")} failed with exit code ${code}`));
    });
  });
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

async function buildAgentBinary() {
  await runCommand("cargo", ["build", "-p", "im_agent", "--bin", "im_agent", "--release"], {
    cwd: repoRoot,
  });

  const info = await stat(binaryPath).catch(() => null);
  if (!info?.isFile()) {
    throw new Error("im_agent binary not found after build");
  }
}

async function writeBundle(version) {
  await rm(tempDir, { recursive: true, force: true });
  await mkdir(tempDir, { recursive: true });

  const binaryDest = path.join(tempDir, binaryName);
  await copyFile(binaryPath, binaryDest);
  await chmod(binaryDest, 0o755);

  await writeFile(
    path.join(tempDir, "version.json"),
    JSON.stringify({ version }, null, 2) + "\n",
    "utf8"
  );

  await rm(outputDir, { recursive: true, force: true });
  await rename(tempDir, outputDir);
}

async function main() {
  ensureLinux();

  const version = await loadVersion();
  await buildAgentBinary();
  await writeBundle(version);
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
