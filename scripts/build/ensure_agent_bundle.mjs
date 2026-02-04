// Path: scripts/build/ensure_agent_bundle.mjs
// Description: Verify the bundled agent binary exists before packaging.

import { readFile, stat } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const repoRoot = process.cwd();
const bundleDir = path.join(repoRoot, "src-tauri", "resources", "agent_bundle");
const binaryPath = path.join(bundleDir, "im_agent");
const versionPath = path.join(bundleDir, "version.json");
const packageJsonPath = path.join(repoRoot, "package.json");

export async function ensureAgentBundle() {
  const pkg = JSON.parse(await readFile(packageJsonPath, "utf8"));
  const version = typeof pkg.version === "string" ? pkg.version.trim() : "";
  if (!version) {
    throw new Error("package.json version is missing");
  }

  let binaryInfo;
  try {
    binaryInfo = await stat(binaryPath);
  } catch {
    throw new Error(
      "Agent bundle is missing im_agent. See docs/commands/agent_bundle.md."
    );
  }

  if (!binaryInfo.isFile() || binaryInfo.size === 0) {
    throw new Error(
      "Agent bundle im_agent is missing or empty. See docs/commands/agent_bundle.md."
    );
  }

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

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  ensureAgentBundle().catch((error) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}
