// Path: scripts/release/check_versions.mjs
// Description: Validate that all public Intermediary version files stay in lockstep.

import process from "node:process";
import { assertConsistentVersions, normalizeVersion } from "./version_contract.mjs";

async function main() {
  const args = process.argv.slice(2);
  let expectedVersion = null;

  for (let i = 0; i < args.length; i += 1) {
    const arg = args[i];
    if (arg === "--expect-version") {
      const rawVersion = args[i + 1];
      if (!rawVersion) {
        throw new Error("--expect-version requires a value.");
      }
      expectedVersion = normalizeVersion(rawVersion);
      i += 1;
      continue;
    }

    if (arg === "--help" || arg === "-h") {
      printUsage();
      return;
    }

    throw new Error(`Unknown argument: ${arg}`);
  }

  const { version, entries } = await assertConsistentVersions();

  if (expectedVersion && version !== expectedVersion) {
    throw new Error(
      `Version contract is ${version}, but expected ${expectedVersion}.`
    );
  }

  console.log(`Version contract OK: ${version}`);
  for (const entry of entries) {
    console.log(`- ${entry.label}`);
  }
}

function printUsage() {
  console.log("Usage: node scripts/release/check_versions.mjs [--expect-version <version>]");
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
