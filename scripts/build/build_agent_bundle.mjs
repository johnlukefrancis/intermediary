// Path: scripts/build/build_agent_bundle.mjs
// Description: Build the bundled WSL agent binary and sync it into Tauri resources.

import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";

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

async function main() {
  ensureLinux();
  const repoRoot = process.cwd();
  const scriptPath = path.join(
    repoRoot,
    "scripts",
    "build",
    "build_agent_bundle.sh"
  );
  await runCommand("bash", [scriptPath], { cwd: repoRoot });
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
