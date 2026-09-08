// Path: app/src/lib/bundles/bundle_selection_visibility_test.ts
// Description: Tree file rows mirror the scanner: a globally excluded file is neither enabled nor included

import { test } from "node:test";
import assert from "node:assert/strict";
import { normalizeGlobalExcludes } from "../../shared/global_exclude_rules.js";
import type { BundleSelection } from "../../shared/protocol.js";
import { isFileEnabled, isFileGloballyExcluded, isFileIncluded } from "./bundle_selection_visibility.js";

const selection: BundleSelection = {
  includeRoot: true,
  topLevelDirs: ["witness"],
  includedSubdirs: ["witness/blorb_creation/iterations/round_06"],
  excludedSubdirs: [],
  excludedFiles: ["witness/blorb_creation/iterations/round_06/beach_ring.png"],
};
const rules = normalizeGlobalExcludes({
  dirNames: ["target"], dirSuffixes: [], fileNames: [], extensions: [".blend", ".blend1"], patterns: [],
});
const blend = "witness/blorb_creation/iterations/round_06/blorb_world.blend";
const png = "witness/blorb_creation/iterations/round_06/crown_glacier.png";

void test("a Blender file under an explicitly included folder is dropped by the rules, so the row is disabled", () => {
  assert.equal(isFileGloballyExcluded(blend, rules), true);
  assert.equal(isFileEnabled(blend, selection, rules), false);
  assert.equal(isFileIncluded(blend, selection, rules), false);
});

void test("a sibling image keeps the selection-driven state", () => {
  assert.equal(isFileGloballyExcluded(png, rules), false);
  assert.equal(isFileEnabled(png, selection, rules), true);
  assert.equal(isFileIncluded(png, selection, rules), true);
  assert.equal(isFileIncluded("witness/blorb_creation/iterations/round_06/beach_ring.png", selection, rules), false);
});
