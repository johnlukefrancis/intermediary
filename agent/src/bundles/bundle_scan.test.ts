// Path: agent/src/bundles/bundle_scan.test.ts
// Description: Unit tests for bundle selection validation

import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";
import { scanBundleContents } from "./bundle_scan.js";
import { AgentError } from "../util/errors.js";

async function withTempRepo(
  fn: (repoRoot: string) => Promise<void>
): Promise<void> {
  const repoRoot = await mkdtemp(path.join(tmpdir(), "intermediary-bundle-scan-"));
  try {
    await mkdir(path.join(repoRoot, ".vscode"), { recursive: true });
    await mkdir(path.join(repoRoot, "src"), { recursive: true });
    await fn(repoRoot);
  } finally {
    await rm(repoRoot, { recursive: true, force: true });
  }
}

void test("rejects dot segments in top-level selection", async () => {
  await withTempRepo(async (repoRoot) => {
    await assert.rejects(
      () =>
        scanBundleContents({
          repoRoot,
          includeRoot: false,
          topLevelDirs: ["."],
        }),
      (err: unknown) =>
        err instanceof AgentError && err.code === "INVALID_SELECTION"
    );

    await assert.rejects(
      () =>
        scanBundleContents({
          repoRoot,
          includeRoot: false,
          topLevelDirs: [".."],
        }),
      (err: unknown) =>
        err instanceof AgentError && err.code === "INVALID_SELECTION"
    );
  });
});

void test("allows dot-prefixed directories like .vscode", async () => {
  await withTempRepo(async (repoRoot) => {
    const result = await scanBundleContents({
      repoRoot,
      includeRoot: false,
      topLevelDirs: [".vscode"],
    });

    assert.deepEqual(result.topLevelDirsIncluded, [".vscode"]);
  });
});
