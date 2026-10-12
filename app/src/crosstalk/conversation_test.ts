// Path: app/src/crosstalk/conversation_test.ts
// Description: Reply context must belong to the latest exact participant pair and stay bounded

import { test } from "node:test";
import assert from "node:assert/strict";
import { receive, type Conversation } from "./conversation.js";
import type { Message } from "./identity.js";

void test("recall uses only a disappeared recent message from the exact reversed pair", () => {
  const first: Message = { id: "a", from: "codex@repo#1111", to: "claude@home#2222", text: "question", sent: 1, cwd: null };
  const reply = { ...first, id: "b", from: first.to, to: first.from, text: "answer" };
  const held = new Set<string>();
  const arrived = receive({ cards: [], recent: [] }, first, held, 0);
  const expired = { ...arrived, cards: [] };
  assert.equal(receive(arrived, reply, held, 20_000).cards.at(-1)?.echo, undefined);
  assert.equal(receive(expired, reply, held, 20_000).cards[0]?.echo?.id, "a");
  assert.equal(receive(expired, { ...reply, from: "claude@home#9999" }, held, 20_000).cards[0]?.echo, undefined);
  assert.equal(receive(expired, { ...first, id: "c" }, held, 20_000).cards[0]?.echo, undefined);
  assert.equal(receive(expired, reply, held, 300_000).cards[0]?.echo, undefined);
  let state: Conversation = expired;
  for (let i = 0; i < 30; i++) state = receive(state, { ...first, id: `pair-${i}`, to: `claude@home#${i}` }, held, 1);
  assert.equal(state.cards.length, 3);
  assert.equal(state.recent.length, 16);
  const allHeld = new Set(state.cards.map(item => item.id));
  assert.equal(receive(state, reply, allHeld, 2), state);
});
