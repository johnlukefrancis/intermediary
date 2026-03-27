// Path: scripts/release/bump_version.mjs
// Description: Update all release-facing version files to a single Intermediary version.

import process from "node:process";
import {
  normalizeVersion,
  readVersionEntries,
  writeVersionToTargets,
} from "./version_contract.mjs";

async function main() {
  const rawVersion = process.argv[2];
  if (!rawVersion || rawVersion === "--help" || rawVersion === "-h") {
    printUsage();
    process.exitCode = rawVersion ? 0 : 1;
    return;
  }

  const nextVersion = normalizeVersion(rawVersion);
  const currentEntries = await readVersionEntries();
  const currentVersions = [...new Set(currentEntries.map((entry) => entry.version))];

  if (currentVersions.length === 1 && currentVersions[0] === nextVersion) {
    console.log(`Version contract already at ${nextVersion}.`);
    return;
  }

  const updatedPaths = await writeVersionToTargets(nextVersion);
  if (currentVersions.length === 1) {
    console.log(
      `Updated Intermediary version contract: ${currentVersions[0]} -> ${nextVersion}`
    );
  } else {
    console.log(`Repaired version contract drift and set all targets to ${nextVersion}`);
    for (const entry of currentEntries) {
      console.log(`  was ${entry.version} in ${entry.label}`);
    }
  }
  for (const updatedPath of updatedPaths) {
    console.log(`- ${updatedPath}`);
  }
}

function printUsage() {
  console.log("Usage: node scripts/release/bump_version.mjs <version>");
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
