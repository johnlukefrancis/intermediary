// Path: app/src/lib/stream/stream_store_test.ts
// Description: Conductor rules under injected timers: flush, idle wake, cadence, away admits, hidden pause and collapse, intake cap, burst close order, idle notice expiry

import { test } from "node:test";
import assert from "node:assert/strict";
import { BURST_CLOSE_MS, BURST_THRESHOLD, CADENCE_BASE_MS, FLUSH_MS, INTAKE_CAP, NOTICE_TTL_MS, STATIC_AFTER_MS } from "./stream_bounds.js";
import { createStreamStore } from "./stream_store.js";
import { snapshotHasEntries } from "./stream_store_support.js";
import { changed, imageDelta, resetSeq, textDelta } from "./testing/stream_fixtures.js";
import type { StreamStore, StreamStoreDeps, StreamTransport } from "./stream_types.js";

interface FakeClock extends StreamStoreDeps {
  advance(ms: number): void;
  live(): number;
}

function fakeClock(): FakeClock {
  let now = 10_000;
  let nextHandle = 1;
  const timers = new Map<number, { due: number; callback: () => void }>();
  return {
    now: () => now,
    setTimer(callback, ms) {
      const handle = nextHandle;
      nextHandle += 1;
      timers.set(handle, { due: now + ms, callback });
      return handle;
    },
    clearTimer(handle) { timers.delete(handle); },
    advance(ms) {
      const target = now + ms;
      for (;;) {
        const next = [...timers.entries()].sort((a, b) => a[1].due - b[1].due)[0];
        if (next === undefined || next[1].due > target) break;
        timers.delete(next[0]);
        now = next[1].due;
        next[1].callback();
      }
      now = target;
    },
    live: () => timers.size,
  };
}

const ONLINE: StreamTransport = { connected: true, helloOk: true, agentVersion: "0.1.23", repoRootKind: "host", wslOnline: true };

function cardsOf(store: StreamStore): number {
  return store.getSnapshot().ring.cards.length;
}

void test("a delta flushes once per FLUSH_MS and an idle stream admits it immediately", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  const before = store.getSnapshot();
  store.intake(textDelta("a.ts"));
  assert.equal(store.getSnapshot(), before);
  clock.advance(FLUSH_MS);
  assert.equal(cardsOf(store), 1);
  assert.equal(store.getSnapshot().pending, 0);
  store.dispose();
});

void test("back-to-back arrivals admit one per cadence tick and turn static after STATIC_AFTER_MS", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  store.intake(textDelta("a.ts"));
  clock.advance(FLUSH_MS);
  store.intake(textDelta("b.ts"));
  store.intake(textDelta("c.ts"));
  clock.advance(FLUSH_MS);
  assert.equal(cardsOf(store), 1);
  assert.equal(store.getSnapshot().pending, 2);
  clock.advance(CADENCE_BASE_MS);
  assert.equal(cardsOf(store), 2);
  clock.advance(CADENCE_BASE_MS);
  assert.equal(cardsOf(store), 3);
  assert.equal(store.getSnapshot().ring.cards.every((card) => card.kind === "file" && !card.static), true);
  clock.advance(STATIC_AFTER_MS * 2);
  assert.equal(store.getSnapshot().ring.cards.every((card) => card.kind === "file" && card.static), true);
  assert.equal(clock.live(), 0);
  store.dispose();
});

void test("not visible: cards land at once and the return announces them", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.intake(textDelta("a.ts"));
  store.intake(textDelta("b.ts"));
  clock.advance(FLUSH_MS);
  assert.equal(cardsOf(store), 2);
  assert.equal(store.getSnapshot().admittedWhileAway, 2);
  store.setVisible(true);
  const snapshot = store.getSnapshot();
  assert.equal(snapshot.admittedWhileAway, 0);
  assert.equal(snapshot.ring.notices.at(-1)?.text, "2 CHANGES WHILE AWAY");
  store.dispose();
});

void test("document hidden pauses admission; a deep backlog collapses into one burst card on show", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  store.setDocumentHidden(true);
  for (let index = 0; index < 3; index += 1) store.intake(textDelta(`f${String(index)}.ts`));
  clock.advance(FLUSH_MS + CADENCE_BASE_MS * 3);
  assert.equal(cardsOf(store), 0);
  assert.equal(store.getSnapshot().pending, 3);
  for (let index = 3; index < BURST_THRESHOLD; index += 1) store.intake(textDelta(`f${String(index)}.ts`));
  clock.advance(FLUSH_MS);
  store.setDocumentHidden(false);
  clock.advance(CADENCE_BASE_MS);
  assert.equal(cardsOf(store), 1);
  assert.equal(store.getSnapshot().ring.cards[0]?.kind, "burst");
  store.dispose();
});

void test("the intake buffer is capped: the oldest fileChanged drop first and the next flush prints them as dropped", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  for (let index = 0; index < INTAKE_CAP; index += 1) store.intake(changed("same.ts"));
  store.intake(textDelta("a.ts"));
  store.intake(textDelta("b.ts"));
  store.intake(textDelta("c.ts"));
  clock.advance(FLUSH_MS);
  const snapshot = store.getSnapshot();
  assert.equal(snapshot.ring.notices.at(-1)?.text, "3 EDITS DROPPED");
  assert.equal(snapshot.ring.notices.at(-1)?.tone, "error");
  // Every delta survived: only fileChanged events were let go
  assert.equal(cardsOf(store) + snapshot.pending, 3);
  store.dispose();
});

void test("a member delta that shares the flush with the burst's close is absorbed, not printed", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  for (let index = 0; index < BURST_THRESHOLD; index += 1) store.intake(changed(`src/f${String(index)}.ts`));
  clock.advance(FLUSH_MS);
  assert.notEqual(store.getSnapshot().ring.burstOpen, null);
  // The close is due before this flush runs; the delta must still land on the burst card
  clock.advance(BURST_CLOSE_MS - FLUSH_MS / 2);
  store.intake(textDelta("src/f3.ts"));
  clock.advance(FLUSH_MS);
  const snapshot = store.getSnapshot();
  assert.equal(snapshot.ring.cards.length, 1);
  assert.equal(snapshot.pending, 0);
  const burst = snapshot.ring.cards[0];
  assert.equal(burst?.kind, "burst");
  assert.equal(burst.resolved, 1);
  store.dispose();
});

void test("a card that waited in the FIFO is not static at admit: the static clock starts at admission", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  store.setDocumentHidden(true);
  assert.equal(store.getSnapshot().documentHidden, true);
  store.intake(textDelta("a.ts"));
  clock.advance(FLUSH_MS);
  assert.equal(store.getSnapshot().pending, 1);
  clock.advance(STATIC_AFTER_MS + 500);
  store.setDocumentHidden(false);
  assert.equal(store.getSnapshot().documentHidden, false);
  assert.equal(cardsOf(store), 1);
  const card = store.getSnapshot().ring.cards[0];
  assert.equal(card?.kind, "file");
  assert.equal(card.static, false);
  assert.equal(card.admittedAtMs, clock.now());
  clock.advance(STATIC_AFTER_MS);
  const aged = store.getSnapshot().ring.cards[0];
  assert.equal(aged?.kind, "file");
  assert.equal(aged.static, true);
  store.dispose();
});

void test("a counters event prints the withheld notice, which expires after NOTICE_TTL_MS on its own timer", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  store.intake({ type: "fileDeltaCounters", repoId: "r", seq: 1, withheld: 7, dropped: 0 });
  clock.advance(FLUSH_MS);
  assert.equal(store.getSnapshot().ring.notices[0]?.text, "7 EDITS WITHHELD · BURST");
  assert.equal(cardsOf(store), 0);
  clock.advance(NOTICE_TTL_MS - 1);
  assert.equal(store.getSnapshot().ring.notices.length, 1);
  clock.advance(1);
  assert.equal(store.getSnapshot().ring.notices.length, 0);
  assert.equal(clock.live(), 0);
  store.dispose();
});

void test("snapshot identity is stable until something changes; transport drives offline and held", () => {
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  const first = store.getSnapshot();
  assert.equal(store.getSnapshot(), first);
  assert.equal(first.offline, true);
  store.setVisible(false);
  assert.equal(store.getSnapshot(), first);
  store.setTransport(ONLINE);
  assert.equal(store.getSnapshot().offline, false);
  assert.equal(store.getSnapshot().support, "supported");
  store.setTransport({ ...ONLINE, repoRootKind: "wsl", wslOnline: false });
  store.setTransport({ ...ONLINE, repoRootKind: "wsl", wslOnline: false });
  const held = store.getSnapshot();
  assert.equal(held.held, true);
  assert.equal(held.ring.notices.length, 1);
  store.markRehydrated();
  assert.equal(store.getSnapshot().ring.notices.at(-1)?.text, "RECONNECTED — RESUMING");
  assert.equal(store.getSnapshot().ring.lastSeq, null);
  store.dispose();
  assert.equal(clock.live(), 0);
});

void test("the selection filter remaps every strip tile as well as every file card", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  store.intake(imageDelta("in/a.png"));
  store.intake(imageDelta("out/b.png"));
  store.intake(textDelta("out/c.ts"));
  clock.advance(FLUSH_MS + CADENCE_BASE_MS);
  assert.equal(cardsOf(store), 2);
  store.setSelectionFilter((path) => path.startsWith("in/"));
  const [strip, file] = store.getSnapshot().ring.cards;
  assert.equal(strip?.kind, "images");
  assert.deepEqual(strip.tiles.map((tile) => tile.outsideSelection), [false, true]);
  assert.equal(file?.kind, "file");
  assert.equal(file.outsideSelection, true);
  store.setSelectionFilter(null);
  const cleared = store.getSnapshot().ring.cards[0];
  assert.equal(cleared?.kind, "images");
  assert.deepEqual(cleared.tiles.map((tile) => tile.outsideSelection), [false, false]);
  store.dispose();
});

void test("a notice or the settling line counts as an entry while the ring holds no card", () => {
  resetSeq();
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  assert.equal(snapshotHasEntries(store.getSnapshot()), false);
  store.intake({ type: "fileDeltaCounters", repoId: "r", seq: 1, withheld: 5, dropped: 0 });
  clock.advance(FLUSH_MS);
  const withNotice = store.getSnapshot();
  assert.equal(withNotice.ring.cards.length, 0);
  assert.equal(withNotice.ring.notices.at(-1)?.text, "5 EDITS WITHHELD · BURST");
  assert.equal(snapshotHasEntries(withNotice), true);
  clock.advance(NOTICE_TTL_MS);
  assert.equal(snapshotHasEntries(store.getSnapshot()), false);
  store.intake(changed("a.ts"));
  clock.advance(FLUSH_MS);
  const settling = store.getSnapshot();
  assert.equal(settling.ring.cards.length, 0);
  assert.deepEqual(settling.settling, ["a.ts"]);
  assert.equal(snapshotHasEntries(settling), true);
  store.dispose();
});

void test("a notice pushed on an idle store (held, then reconnected) still expires after NOTICE_TTL_MS", () => {
  const clock = fakeClock();
  const store = createStreamStore("r", clock);
  store.setVisible(true);
  assert.equal(clock.live(), 0);
  store.setTransport({ ...ONLINE, repoRootKind: "wsl", wslOnline: false });
  assert.equal(store.getSnapshot().ring.notices.length, 1);
  assert.equal(clock.live(), 1);
  clock.advance(NOTICE_TTL_MS);
  assert.equal(store.getSnapshot().ring.notices.length, 0);
  store.markRehydrated();
  assert.equal(store.getSnapshot().ring.notices.at(-1)?.text, "RECONNECTED — RESUMING");
  clock.advance(NOTICE_TTL_MS);
  assert.equal(store.getSnapshot().ring.notices.length, 0);
  assert.equal(clock.live(), 0);
  store.dispose();
});
