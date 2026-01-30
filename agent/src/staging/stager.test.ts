// Path: agent/src/staging/stager.test.ts
// Description: Unit tests for staging path validation

import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";
import { createStager } from "./stager.js";
import { AgentError } from "../util/errors.js";

void test("rejects empty and dot relative paths", async () => {
  const repoRoot = await mkdtemp(path.join(tmpdir(), "intermediary-stager-"));
  const stagingRoot = path.join(repoRoot, "staging");
  try {
    const stager = createStager({
      stagingWslRoot: stagingRoot,
      stagingWinRoot: "C:\\staging",
    });

    await assert.rejects(
      () => stager.stageFile("repo", repoRoot, ""),
      (err: unknown) => err instanceof AgentError && err.code === "INVALID_PATH"
    );

    await assert.rejects(
      () => stager.stageFile("repo", repoRoot, "."),
      (err: unknown) => err instanceof AgentError && err.code === "INVALID_PATH"
    );
  } finally {
    await rm(repoRoot, { recursive: true, force: true });
  }
});
