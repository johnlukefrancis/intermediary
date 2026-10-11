// Path: app/src/lib/agent/agent_client_inspection_test.ts
// Description: Remote hover cancellation and settled-request cleanup at the WebSocket boundary

import assert from "node:assert/strict";
import { test } from "node:test";
import { createAgentClient } from "./agent_client.js";
import { RequestEnvelopeSchema, type UiCommand } from "../../shared/protocol.js";

class TestSocket {
  static OPEN = 1;
  static CONNECTING = 0;
  static current: TestSocket;
  readyState = 1;
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  onmessage: ((event: { data: string }) => void) | null = null;
  constructor() { TestSocket.current = this; }
  send(data: string): void { this.sent.push(data); }
  close(): void { this.readyState = 3; }
}

const command: Extract<UiCommand, { type: "inspectBundleEntry" }> = {
  type: "inspectBundleEntry", repoId: "repo", path: "Saved",
  selection: { includeRoot: false, topLevelDirs: [], includedSubdirs: [], includedFiles: [], excludedSubdirs: [], excludedFiles: [] },
  globalExcludes: { dirNames: [], dirSuffixes: [], fileNames: [], extensions: [], patterns: [] },
};

void test("hover abort cancels only the pending request and ignores its late response", async (t) => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "WebSocket");
  Object.defineProperty(globalThis, "WebSocket", { configurable: true, value: TestSocket });
  t.after(() => { if (descriptor) Object.defineProperty(globalThis, "WebSocket", descriptor); });
  const client = createAgentClient({ host: "localhost", port: 3141, authToken: "test", onConnectionChange: () => {}, onEvent: () => {} });
  t.after(() => { client.disconnect(); });
  client.connect();
  const socket = TestSocket.current;
  const controller = new AbortController();
  const pending = client.send(command, controller.signal);
  const request = RequestEnvelopeSchema.parse(JSON.parse(socket.sent[0] ?? ""));
  controller.abort();
  await assert.rejects(pending, { name: "AbortError" });
  assert.equal(socket.sent[1], JSON.stringify({ kind: "cancel", requestId: request.requestId }));
  const response = (requestId: string): void => {
    socket.onmessage?.({ data: JSON.stringify({
      kind: "response", requestId, status: "ok", payload: {
        type: "inspectBundleEntryResult", repoId: "repo", path: "Saved",
        inspection: { isDirectory: true, bytes: 20, fileCount: 2, includedBytes: 10, includedFileCount: 1,
          largestIncludedFiles: [{ path: "Saved/keep.txt", bytes: 10 }], modifiedMs: null, complete: true, unreadableEntries: 0 },
      },
    }) });
  };
  response(request.requestId);
  const secondController = new AbortController();
  const second = client.send(command, secondController.signal);
  const secondRequest = RequestEnvelopeSchema.parse(JSON.parse(socket.sent[2] ?? ""));
  response(secondRequest.requestId);
  assert.equal((await second).type, "inspectBundleEntryResult");
  secondController.abort();
  assert.equal(socket.sent.length, 3, "completed inspection must release its abort listener");
  await assert.rejects(client.send(command, controller.signal), { name: "AbortError" });
  assert.equal(socket.sent.length, 3, "already aborted hover must not send a request");
});
