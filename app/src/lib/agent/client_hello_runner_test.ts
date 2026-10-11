// Path: app/src/lib/agent/client_hello_runner_test.ts
// Description: Single-flight coalescing, retry backoff, WSL generation re-hello and disposal of the hello runner

import assert from "node:assert/strict";
import { test } from "node:test";
import {
  ClientHelloRunner,
  computeHelloRetryDelayMs,
  type HelloOutcome,
} from "./client_hello_runner.js";
import type { ClientHelloResult } from "../../shared/protocol.js";

const RESULT: ClientHelloResult = { type: "clientHelloResult", watchedRepoIds: ["a"], agentVersion: "1.0.0" };

interface Pending { resolve: (r: ClientHelloResult) => void; reject: (e: Error) => void }

function harness() {
  const sends: Pending[] = [];
  const outcomes: HelloOutcome[] = [];
  const timers: Array<{ run: () => void; delayMs: number; cancelled: boolean }> = [];
  const runner = new ClientHelloRunner({
    send: () => new Promise<ClientHelloResult>((resolve, reject) => { sends.push({ resolve, reject }); }),
    onStart: () => {},
    onOutcome: (outcome) => { outcomes.push(outcome); },
    schedule: (run, delayMs) => {
      const timer = { run, delayMs, cancelled: false };
      timers.push(timer);
      return () => { timer.cancelled = true; };
    },
  });
  return { runner, sends, outcomes, timers };
}

function nth<T>(items: T[], index: number): T {
  const item = items[index];
  if (item === undefined) throw new Error(`missing item ${String(index)}`);
  return item;
}

const flush = (): Promise<void> => new Promise((resolve) => setImmediate(resolve));

void test("retry delay doubles from 1 s and caps at 30 s", () => {
  assert.deepEqual([0, 1, 2, 3, 4, 5, 9].map(computeHelloRetryDelayMs), [1000, 2000, 4000, 8000, 16000, 30000, 30000]);
});

void test("requests during an in-flight hello coalesce into one trailing hello", async () => {
  const { runner, sends } = harness();
  const first = runner.request();
  const second = runner.request();
  const third = runner.request();
  assert.equal(sends.length, 1);
  nth(sends, 0).resolve(RESULT);
  assert.equal(await first, true);
  await flush();
  assert.equal(sends.length, 2);
  nth(sends, 1).resolve(RESULT);
  assert.deepEqual(await Promise.all([second, third]), [true, true]);
});

void test("failures retry with growing backoff and success resets it", async () => {
  const { runner, sends, timers, outcomes } = harness();
  const first = runner.request();
  nth(sends, 0).reject(new Error("Request timeout: clientHello"));
  assert.equal(await first, false);
  assert.deepEqual(outcomes[0], { kind: "error", message: "Request timeout: clientHello" });
  assert.equal(nth(timers, 0).delayMs, 1000);
  nth(timers, 0).run();
  nth(sends, 1).reject(new Error("again"));
  await flush();
  assert.equal(nth(timers, 1).delayMs, 2000);
  nth(timers, 1).run();
  nth(sends, 2).resolve(RESULT);
  await flush();
  assert.equal(timers.length, 2);
  void runner.request();
  nth(sends, 3).reject(new Error("later"));
  await flush();
  assert.equal(nth(timers, 2).delayMs, 1000);
});

void test("an explicit request cancels the pending retry and sends immediately", async () => {
  const { runner, sends, timers } = harness();
  void runner.request();
  nth(sends, 0).reject(new Error("down"));
  await flush();
  const resync = runner.request();
  assert.equal(nth(timers, 0).cancelled, true);
  assert.equal(sends.length, 2);
  nth(sends, 1).resolve(RESULT);
  assert.equal(await resync, true);
});

void test("WSL online re-hellos once per generation", async () => {
  const { runner, sends } = harness();
  runner.noteWslOnline(1);
  runner.noteWslOnline(1);
  assert.equal(sends.length, 1);
  nth(sends, 0).resolve(RESULT);
  await flush();
  runner.noteWslOnline(1);
  runner.noteWslOnline(2);
  assert.equal(sends.length, 2);
});

void test("dispose resolves waiters false, cancels retries and ignores late results", async () => {
  const { runner, sends, timers, outcomes } = harness();
  const inFlight = runner.request();
  const queued = runner.request();
  runner.dispose();
  assert.equal(await queued, false);
  nth(sends, 0).resolve(RESULT);
  assert.equal(await inFlight, false);
  assert.equal(outcomes.length, 0);
  assert.equal(await runner.request(), false);
  assert.equal(timers.length, 0);
});
