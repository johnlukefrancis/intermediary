// Path: scripts/release/version_contract.mjs
// Description: Shared version-contract helpers for Intermediary release automation.

import { readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

export const VERSION_TARGETS = [
  { kind: "json", path: "package.json", label: "package.json" },
  { kind: "json", path: "src-tauri/tauri.conf.json", label: "src-tauri/tauri.conf.json" },
  { kind: "toml", path: "src-tauri/Cargo.toml", label: "src-tauri/Cargo.toml" },
  { kind: "toml", path: "crates/im_agent/Cargo.toml", label: "crates/im_agent/Cargo.toml" },
  {
    kind: "toml",
    path: "crates/im_host_agent/Cargo.toml",
    label: "crates/im_host_agent/Cargo.toml",
  },
  { kind: "toml", path: "crates/im_bundle/Cargo.toml", label: "crates/im_bundle/Cargo.toml" },
  {
    kind: "json",
    path: "src-tauri/resources/agent_bundle/version.json",
    label: "src-tauri/resources/agent_bundle/version.json",
  },
];

const VERSION_PATTERN = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;
const TOML_VERSION_LINE = /^version = "([^"]+)"$/m;

export function normalizeVersion(rawVersion) {
  if (typeof rawVersion !== "string") {
    throw new Error("Version must be a string.");
  }

  const normalized = rawVersion.trim().replace(/^v/, "");
  if (!VERSION_PATTERN.test(normalized)) {
    throw new Error(
      `Version "${rawVersion}" is invalid. Expected semver-like value such as 0.1.0 or 0.1.0-rc.1.`
    );
  }

  return normalized;
}

export async function readVersionEntries(repoRoot = process.cwd()) {
  const entries = [];

  for (const target of VERSION_TARGETS) {
    const absolutePath = path.join(repoRoot, target.path);
    const contents = await readFile(absolutePath, "utf8");
    const version =
      target.kind === "json"
        ? readJsonVersion(contents, target.label)
        : readTomlVersion(contents, target.label);

    entries.push({
      ...target,
      absolutePath,
      version,
    });
  }

  return entries;
}

export async function assertConsistentVersions(repoRoot = process.cwd()) {
  const entries = await readVersionEntries(repoRoot);
  const versions = new Set(entries.map((entry) => entry.version));

  if (versions.size !== 1) {
    throw new Error(
      [
        "Version contract mismatch:",
        ...entries.map((entry) => `- ${entry.label}: ${entry.version}`),
      ].join("\n")
    );
  }

  return {
    version: entries[0].version,
    entries,
  };
}

export async function writeVersionToTargets(nextVersion, repoRoot = process.cwd()) {
  const normalized = normalizeVersion(nextVersion);
  const writes = [];

  for (const target of VERSION_TARGETS) {
    const absolutePath = path.join(repoRoot, target.path);
    const contents = await readFile(absolutePath, "utf8");
    const updatedContents =
      target.kind === "json"
        ? writeJsonVersion(contents, normalized, target.label)
        : writeTomlVersion(contents, normalized, target.label);

    if (updatedContents !== contents) {
      await writeFile(absolutePath, updatedContents, "utf8");
    }

    writes.push(target.path);
  }

  return writes;
}

function readJsonVersion(contents, label) {
  let parsed;
  try {
    parsed = JSON.parse(contents);
  } catch (error) {
    throw new Error(`${label} is not valid JSON: ${error instanceof Error ? error.message : error}`);
  }

  const version = typeof parsed?.version === "string" ? parsed.version.trim() : "";
  if (!version) {
    throw new Error(`${label} is missing a non-empty version field.`);
  }

  return version;
}

function writeJsonVersion(contents, nextVersion, label) {
  let parsed;
  try {
    parsed = JSON.parse(contents);
  } catch (error) {
    throw new Error(`${label} is not valid JSON: ${error instanceof Error ? error.message : error}`);
  }

  parsed.version = nextVersion;
  return `${JSON.stringify(parsed, null, 2)}\n`;
}

function readTomlVersion(contents, label) {
  const match = contents.match(TOML_VERSION_LINE);
  if (!match) {
    throw new Error(`${label} is missing a top-level package version.`);
  }

  const version = match[1].trim();
  if (!version) {
    throw new Error(`${label} has an empty package version.`);
  }

  return version;
}

function writeTomlVersion(contents, nextVersion, label) {
  if (!TOML_VERSION_LINE.test(contents)) {
    throw new Error(`${label} is missing a top-level package version.`);
  }

  return contents.replace(TOML_VERSION_LINE, `version = "${nextVersion}"`);
}
