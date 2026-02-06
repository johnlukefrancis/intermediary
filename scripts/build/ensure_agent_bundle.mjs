// Path: scripts/build/ensure_agent_bundle.mjs
// Description: Verify bundled host/WSL agent binaries exist before packaging.

import { copyFile, readFile, stat } from "node:fs/promises";
import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const repoRoot = process.cwd();
const bundleDir = path.join(repoRoot, "src-tauri", "resources", "agent_bundle");
const wslBinaryPath = path.join(bundleDir, "im_agent");
const hostBinaryPath = path.join(bundleDir, "im_host_agent.exe");
const versionPath = path.join(bundleDir, "version.json");
const packageJsonPath = path.join(repoRoot, "package.json");
const hostBuildBinaryPath = path.join(
  repoRoot,
  "target",
  "release",
  "im_host_agent.exe"
);

export async function ensureAgentBundle() {
  const pkg = JSON.parse(await readFile(packageJsonPath, "utf8"));
  const version = typeof pkg.version === "string" ? pkg.version.trim() : "";
  if (!version) {
    throw new Error("package.json version is missing");
  }

  const wslBinaryInfo = await readRequiredFileInfo(
    wslBinaryPath,
    "Agent bundle is missing im_agent. See docs/commands/agent_bundle.md."
  );

  if (!wslBinaryInfo.isFile() || wslBinaryInfo.size === 0) {
    throw new Error(
      "Agent bundle im_agent is missing or empty. See docs/commands/agent_bundle.md."
    );
  }

  await ensureHostBinary();

  let bundleVersionRaw;
  try {
    bundleVersionRaw = await readFile(versionPath, "utf8");
  } catch {
    throw new Error(
      "Agent bundle version.json is missing. See docs/commands/agent_bundle.md."
    );
  }

  let bundleVersion;
  try {
    const parsed = JSON.parse(bundleVersionRaw);
    bundleVersion =
      typeof parsed?.version === "string" ? parsed.version.trim() : "";
  } catch (error) {
    throw new Error(
      `Agent bundle version.json is invalid: ${error?.message ?? error}.`
    );
  }

  if (!bundleVersion) {
    throw new Error(
      "Agent bundle version.json has an empty version. See docs/commands/agent_bundle.md."
    );
  }

  if (bundleVersion !== version) {
    throw new Error(
      `Agent bundle version ${bundleVersion} does not match package.json ${version}. See docs/commands/agent_bundle.md.`
    );
  }
}

async function ensureHostBinary() {
  if (process.platform === "win32") {
    await runCommand("cargo", [
      "build",
      "-p",
      "im_host_agent",
      "--bin",
      "im_host_agent",
      "--release",
    ]);
  }

  const builtHostBinary = await tryReadFileInfo(hostBuildBinaryPath);
  if (builtHostBinary && builtHostBinary.isFile() && builtHostBinary.size > 0) {
    // Always refresh bundle from the latest release artifact when available.
    await copyFile(hostBuildBinaryPath, hostBinaryPath);
    return;
  }

  const existingHostBinary = await tryReadFileInfo(hostBinaryPath);
  if (existingHostBinary && existingHostBinary.isFile() && existingHostBinary.size > 0) {
    return;
  }

  throw new Error(
    "Agent bundle is missing im_host_agent.exe. See docs/commands/agent_bundle.md."
  );
}

async function readRequiredFileInfo(filePath, missingMessage) {
  const info = await tryReadFileInfo(filePath);
  if (!info) {
    throw new Error(missingMessage);
  }
  return info;
}

async function tryReadFileInfo(filePath) {
  try {
    return await stat(filePath);
  } catch {
    return null;
  }
}

function runCommand(command, args) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      stdio: "inherit",
      shell: process.platform === "win32",
    });
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

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  ensureAgentBundle().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
