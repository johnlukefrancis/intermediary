// Path: app/src/lib/agent/transient_wsl_error_test.ts
// Description: Repo request retry classification treats UI request timeouts as transient

import assert from "node:assert/strict";
import { test } from "node:test";
import { isTransientRepoRequestError } from "./transient_wsl_error.js";

void test("request timeouts and WSL transport failures are transient; other errors are terminal", () => {
  assert.equal(isTransientRepoRequestError(new Error("Request timeout: watchRepo")), true);
  assert.equal(isTransientRepoRequestError(new Error("WSL_BACKEND_UNAVAILABLE: offline")), true);
  assert.equal(isTransientRepoRequestError(new Error("Unknown repo: x")), false);
  assert.equal(isTransientRepoRequestError("timeout"), false);
});
