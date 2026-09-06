// Path: app/src/lib/stream/stream_ring_apply_support.ts
// Description: Shared reducer plumbing: reduce state seed, id allocation, card replacement, notices, path lookup

import { RING_SIZE } from "./stream_bounds.js";
import { INITIAL_BURST_DETECT } from "./stream_burst_detect.js";
import { EMPTY_RING, pushNotice } from "./stream_ring.js";
import type {
  StreamBurstCard,
  StreamFileCard,
  StreamImageStripCard,
  StreamNoticeTone,
  StreamPendingCard,
  StreamReduceState,
  StreamRingCard,
} from "./stream_types.js";

export interface ApplyOptions {
  outsideSelection: (path: string) => boolean;
}

export function initialReduceState(): StreamReduceState {
  return { ring: EMPTY_RING, pending: [], settling: [], burstDetect: INITIAL_BURST_DETECT, nextId: 1 };
}

export function takeId(state: StreamReduceState): [number, StreamReduceState] {
  return [state.nextId, { ...state, nextId: state.nextId + 1 }];
}

export function withoutSettling(state: StreamReduceState, path: string): StreamReduceState {
  if (!state.settling.some((entry) => entry.path === path)) return state;
  return { ...state, settling: state.settling.filter((entry) => entry.path !== path) };
}

export function notice(
  state: StreamReduceState,
  key: string,
  tone: StreamNoticeTone,
  count: number,
  textFor: (count: number) => string,
  now: number
): StreamReduceState {
  const [id, next] = takeId(state);
  return { ...next, ring: pushNotice(next.ring, { key, tone, count, textFor }, now, id) };
}

export type CardPlace = "pending" | "ring";

export function replaceCard(state: StreamReduceState, where: CardPlace, card: StreamPendingCard): StreamReduceState {
  if (where === "pending") {
    return { ...state, pending: state.pending.map((entry) => (entry.id === card.id ? card : entry)) };
  }
  const cards = state.ring.cards.map((entry) => (entry.id === card.id ? card : entry));
  return { ...state, ring: { ...state.ring, cards } };
}

export function updateBurst(
  state: StreamReduceState,
  id: number,
  update: (card: StreamBurstCard) => StreamBurstCard
): StreamReduceState {
  const map = (card: StreamPendingCard): StreamPendingCard =>
    card.kind === "burst" && card.id === id ? update(card) : card;
  const pending = state.pending.map(map);
  const cards = state.ring.cards.map((card) => (card.kind === "history" ? card : map(card)));
  return { ...state, pending, ring: { ...state.ring, cards } };
}

/** The newest card matching `pick` among the last `budget` entries, skipping exiting cards */
function newestWhere<T extends StreamPendingCard>(
  cards: readonly StreamRingCard[],
  budget: number,
  pick: (card: StreamPendingCard) => card is T
): T | null {
  const floor = Math.max(0, cards.length - budget);
  for (let index = cards.length - 1; index >= floor; index -= 1) {
    const card = cards[index];
    if (card !== undefined && card.kind !== "history" && !card.exiting && pick(card)) return card;
  }
  return null;
}

/** Pending FIFO first (newest at its tail), then the ring, looking back at most RING_SIZE cards in total */
function newestCard<T extends StreamPendingCard>(
  state: StreamReduceState,
  pick: (card: StreamPendingCard) => card is T
): { where: CardPlace; card: T } | null {
  const pending = newestWhere(state.pending, RING_SIZE, pick);
  if (pending !== null) return { where: "pending", card: pending };
  const ring = newestWhere(state.ring.cards, RING_SIZE - Math.min(RING_SIZE, state.pending.length), pick);
  return ring === null ? null : { where: "ring", card: ring };
}

/** The newest live file card of `path` in admission order */
export function newestCardOfPath(
  state: StreamReduceState,
  path: string
): { where: CardPlace; card: StreamFileCard } | null {
  return newestCard(state, (card): card is StreamFileCard => card.kind === "file" && card.path === path);
}

/** Whether `id` is the feed's tail entry: the last non-exiting pending card, else the last non-exiting ring card */
export function isTailCard(state: StreamReduceState, id: number): boolean {
  const tail = (cards: readonly StreamRingCard[]): StreamRingCard | undefined => {
    for (let index = cards.length - 1; index >= 0; index -= 1) {
      const card = cards[index];
      if (card !== undefined && !card.exiting) return card;
    }
    return undefined;
  };
  const last = tail(state.pending) ?? tail(state.ring.cards);
  return last?.id === id;
}

/** The newest live image strip in admission order; the only strip that can still accept tiles */
export function newestStrip(state: StreamReduceState): { where: CardPlace; card: StreamImageStripCard } | null {
  return newestCard(state, (card): card is StreamImageStripCard => card.kind === "images");
}
