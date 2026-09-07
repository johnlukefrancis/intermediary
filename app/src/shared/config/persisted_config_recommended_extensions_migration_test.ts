// Path: app/src/shared/config/persisted_config_recommended_extensions_migration_test.ts
// Description: Recommended-baseline configs gain newly recommended exclude extensions; trimmed lists stay authoritative

import { test } from "node:test";
import assert from "node:assert/strict";
import { GLOBAL_EXCLUDE_SCENE_EXTENSIONS } from "../global_excludes.js";
import { parsePersistedConfig, PersistedConfigSchema } from "./persisted_config.js";
import { CONFIG_VERSION } from "./version.js";

function baselineAt(version: number) {
  const config = PersistedConfigSchema.parse({});
  return { ...config, configVersion: version };
}

void test("a v26 recommended baseline gains the scene extensions", () => {
  const stored = baselineAt(26);
  stored.globalExcludes.extensions = stored.globalExcludes.extensions.filter(
    (value) => !GLOBAL_EXCLUDE_SCENE_EXTENSIONS.includes(value)
  );

  const migrated = parsePersistedConfig(stored);

  assert.equal(migrated.configVersion, CONFIG_VERSION);
  for (const extension of GLOBAL_EXCLUDE_SCENE_EXTENSIONS) {
    assert.ok(migrated.globalExcludes.extensions.includes(extension), extension);
  }
});

void test("a trimmed v26 exclude list is left as the user wrote it", () => {
  const stored = baselineAt(26);
  stored.globalExcludes.extensions = stored.globalExcludes.extensions.filter(
    (value) => !GLOBAL_EXCLUDE_SCENE_EXTENSIONS.includes(value) && value !== ".bin"
  );
  const before = [...stored.globalExcludes.extensions];

  const migrated = parsePersistedConfig(stored);

  assert.deepEqual(migrated.globalExcludes.extensions, before);
});

void test("a v7 baseline gains both the v8 and the v27 additions", () => {
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
