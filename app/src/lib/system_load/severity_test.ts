// Path: app/src/lib/system_load/severity_test.ts
// Description: Memory severity and the three-sample paging rule from the system-load design

import { test } from "node:test";
import assert from "node:assert/strict";
import type { MemoryLoad } from "../../shared/protocol_system_load.js";
import { memorySeverity, nextPagingStreak, percentSeverity } from "./severity.js";

const GB = 1024 ** 3;

function memory(inUsePercent: number, hardFaultsPerSec: number, commitPercent = 50): MemoryLoad {
  const totalBytes = 32 * GB;
  return {
    totalBytes,
    availableBytes: totalBytes * (1 - inUsePercent / 100),
    commitBytes: commitPercent * GB,
    commitLimitBytes: 100 * GB,
    hardFaultsPerSec,
  };
}

void test("percent bands split at 60 and 85", () => {
  assert.equal(percentSeverity(59.9), "calm");
  assert.equal(percentSeverity(60), "busy");
  assert.equal(percentSeverity(85), "hot");
});

void test("memory bands use in-use percent and commit pressure", () => {
  assert.equal(memorySeverity(memory(74, 0), 0), "calm");
  assert.equal(memorySeverity(memory(75, 0), 0), "busy");
  assert.equal(memorySeverity(memory(90, 0), 0), "hot");
  assert.equal(memorySeverity(memory(40, 0, 92), 0), "hot");
});

void test("paging needs three consecutive high-fault samples at 85% in use", () => {
  const paging = memory(88, 250);
  let streak = 0;
  streak = nextPagingStreak(streak, paging);
  streak = nextPagingStreak(streak, paging);
  assert.equal(memorySeverity(paging, streak), "busy");
  streak = nextPagingStreak(streak, paging);
  assert.equal(memorySeverity(paging, streak), "paging");
  assert.equal(nextPagingStreak(streak, memory(84, 250)), 0);
  assert.equal(nextPagingStreak(streak, memory(88, 99)), 0);
});
