// Path: app/src/shared/config/persisted_config_recommended_extensions_migration_test.ts
// Description: Every config is seeded once with the scene extensions; baseline-gated additions still respect trimmed lists

import { test } from "node:test";
import assert from "node:assert/strict";
import { GLOBAL_EXCLUDE_SCENE_EXTENSIONS } from "../global_excludes.js";
import { parsePersistedConfig, PersistedConfigSchema } from "./persisted_config.js";
import { CONFIG_VERSION } from "./version.js";

function baselineAt(version: number) {
  const config = PersistedConfigSchema.parse({});
  return { ...config, configVersion: version };
}

void test("a v27 recommended baseline gains the scene extensions", () => {
  const stored = baselineAt(27);
  stored.globalExcludes.extensions = stored.globalExcludes.extensions.filter(
    (value) => !GLOBAL_EXCLUDE_SCENE_EXTENSIONS.includes(value)
  );

  const migrated = parsePersistedConfig(stored);

  assert.equal(migrated.configVersion, CONFIG_VERSION);
  for (const extension of GLOBAL_EXCLUDE_SCENE_EXTENSIONS) {
    assert.ok(migrated.globalExcludes.extensions.includes(extension), extension);
  }
});

void test("a trimmed v27 config is seeded with the scene extensions and nothing else", () => {
  const stored = baselineAt(27);
  stored.globalExcludes = {
    dirNames: ["target"],
    dirSuffixes: [],
    fileNames: [],
    extensions: [],
    patterns: [],
  };

  const migrated = parsePersistedConfig(stored);

  assert.deepEqual(migrated.globalExcludes.extensions, [...GLOBAL_EXCLUDE_SCENE_EXTENSIONS]);
  assert.deepEqual(migrated.globalExcludes.dirNames, ["target"]);
});

void test("a trimmed v7 config does not regain the baseline-gated v8 additions", () => {
  const stored = baselineAt(7);
  stored.globalExcludes.extensions = stored.globalExcludes.extensions.filter(
    (value) =>
      !GLOBAL_EXCLUDE_SCENE_EXTENSIONS.includes(value) && ![".exe", ".bin"].includes(value)
  );

  const migrated = parsePersistedConfig(stored);

  assert.ok(!migrated.globalExcludes.extensions.includes(".exe"));
  assert.ok(!migrated.globalExcludes.extensions.includes(".bin"));
  assert.ok(migrated.globalExcludes.extensions.includes(".blend"));
});

void test("a v7 baseline gains both the v8 and the v28 additions", () => {
  const stored = baselineAt(7);
  stored.globalExcludes.extensions = stored.globalExcludes.extensions.filter(
    (value) =>
      !GLOBAL_EXCLUDE_SCENE_EXTENSIONS.includes(value) &&
      ![".exe", ".dll", ".so", ".dylib", ".pdb", ".lib", ".a", ".gguf"].includes(value)
  );

  const migrated = parsePersistedConfig(stored);

  for (const extension of [".exe", ".gguf", ...GLOBAL_EXCLUDE_SCENE_EXTENSIONS]) {
    assert.ok(migrated.globalExcludes.extensions.includes(extension), extension);
  }
});
