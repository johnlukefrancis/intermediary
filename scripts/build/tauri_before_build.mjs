// Path: scripts/build/tauri_before_build.mjs
// Description: Validate the agent bundle before running the Tauri frontend build.

import { spawn } from "node:child_process";
import process from "node:process";
import { ensureAgentBundle } from "./ensure_agent_bundle.mjs";

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

async function main() {
  await ensureAgentBundle();
  await runCommand("pnpm", ["build"]);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
