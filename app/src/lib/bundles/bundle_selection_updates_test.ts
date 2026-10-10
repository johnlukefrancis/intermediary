// Path: app/src/lib/bundles/bundle_selection_updates_test.ts
// Description: Regression coverage for selective inclusion beneath excluded bundle ancestors

import { test } from "node:test";
import assert from "node:assert/strict";
import { BundleSelectionSchema } from "../../shared/protocol_bundles.js";
import { BundleSelectionSchema as SavedSelectionSchema } from "../../shared/config/persisted_config.js";
import { normalizeGlobalExcludes } from "../../shared/global_exclude_rules.js";
import { createPresetState } from "../../hooks/bundles/bundle_selection_defaults.js";
import { toggleDirectory, toggleFile } from "./bundle_selection_updates.js";
import {
  directoryHasInclusions, isDirectoryIncluded, isFileIncluded,
} from "./bundle_selection_visibility.js";

const rules = normalizeGlobalExcludes({
  dirNames: ["Saved", "target"], dirSuffixes: [], fileNames: ["secret.txt"],
  extensions: [".bin"], patterns: ["private"],
});
const empty = BundleSelectionSchema.parse({ includeRoot: false, topLevelDirs: [] });

void test("excluded parents permit independent subtrees and files across persistence", () => {
  let selection = toggleDirectory("Saved/Keep", empty, rules);
  selection = toggleFile("Saved/Logs/one.txt", selection, rules);
  selection = toggleFile("README.md", selection, rules);
  const persisted = SavedSelectionSchema.parse(JSON.parse(JSON.stringify(selection)));
  const preset = { presetId: "context", presetName: "Context", includeRoot: true, topLevelDirs: [] };
  selection = createPresetState(preset, ["Saved"], persisted, ["Saved"], {}).selection;
  assert.deepEqual(selection.topLevelDirs, []);
  assert.equal(selection.includeRoot, false);
  assert.equal(isDirectoryIncluded("Saved", selection, rules), false);
  assert.equal(directoryHasInclusions("Saved", selection, rules), true);
  for (const path of ["Saved/Keep/a.txt", "Saved/Keep/deeper/b.txt", "Saved/Logs/one.txt", "README.md"]) {
    assert.equal(isFileIncluded(path, selection, rules), true, path);
  }
  for (const path of ["Saved/other.txt", "Saved/Logs/two.txt", "other.md", "Saved/Keep/secret.txt"]) {
    assert.equal(isFileIncluded(path, selection, rules), false, path);
  }
});

void test("closest directory rule wins and disabling a subtree clears its positive overrides", () => {
  let selection = toggleDirectory("Saved", empty, rules);
  selection = toggleDirectory("Saved/Keep", selection, rules);
  selection = toggleDirectory("Saved/Keep/Nested", selection, rules);
  selection = toggleFile("Saved/Keep/one.txt", selection, rules);
  assert.equal(isFileIncluded("Saved/Keep/Nested/a.txt", selection, rules), true);
  assert.equal(isFileIncluded("Saved/Keep/one.txt", selection, rules), true);
  assert.equal(isFileIncluded("Saved/Keep/two.txt", selection, rules), false);
  selection = toggleFile("Saved/Keep/one.txt", selection, rules);
  assert.equal(isFileIncluded("Saved/Keep/one.txt", selection, rules), false);
  selection = toggleDirectory("Saved", selection, rules);
  assert.equal(directoryHasInclusions("Saved", selection, rules), false);
  assert.equal(isFileIncluded("Saved/Keep/Nested/a.txt", selection, rules), false);
});

void test("explicit descendants cross name defaults but never hard global file or path rules", () => {
  const selection = toggleDirectory("Saved/target/source", empty, rules);
  assert.equal(isFileIncluded("Saved/target/source/a.txt", selection, rules), true);
  assert.equal(isFileIncluded("Saved/target/other.txt", selection, rules), false);
  assert.equal(isFileIncluded("Saved/target/source/target/cache.txt", selection, rules), false);
  for (const path of ["Saved/target/source/secret.txt", "Saved/blob.bin", "Saved/private/a.txt"]) {
    assert.equal(toggleFile(path, selection, rules), selection);
    assert.equal(isFileIncluded(path, selection, rules), false);
  }
  assert.equal(toggleDirectory("Saved/private", selection, rules), selection);
});
