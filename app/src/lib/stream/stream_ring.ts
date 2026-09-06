// Path: app/src/lib/stream/stream_ring.ts
// Description: Pure ring operations: admit and evict, expand, notices, burst cards, history seed, static sweep

import {
  HISTORY_ROWS,
  MAX_EXPANDED,
  NOTICE_MAX,
  NOTICE_MERGE_MS,
  NOTICE_TTL_MS,
  RING_SIZE,
  STATIC_AFTER_MS,
} from "./stream_bounds.js";
import { countDir } from "./stream_burst_card.js";
import type {
  StreamBurstCard,
  StreamExpandableCard,
  StreamHistorySeed,
  StreamNoticeRow,
  StreamNoticeTone,
  StreamRing,
  StreamRingCard,
} from "./stream_types.js";

export const EMPTY_RING: StreamRing = { cards: [], notices: [], lastSeq: null, burstOpen: null };

export function spliceExited(ring: StreamRing): StreamRing {
  if (!ring.cards.some((card) => card.exiting)) return ring;
  return { ...ring, cards: ring.cards.filter((card) => !card.exiting) };
}

/** File cards and image strips expand in place; both are eviction-exempt while expanded */
export function isExpandable(card: StreamRingCard): card is StreamExpandableCard {
  return card.kind === "file" || card.kind === "images";
}

function evictable(card: StreamRingCard): boolean {
  if (card.exiting) return false;
  return !isExpandable(card) || !card.expanded;
}

/** Oldest evictable card, history rows first: index or -1 */
function victimIndex(cards: readonly StreamRingCard[]): number {
  const history = cards.findIndex((card) => card.kind === "history" && !card.exiting);
  if (history !== -1) return history;
  return cards.findIndex(evictable);
}

/**
 * Append at the bottom; already-exiting cards are spliced first. Over RING_SIZE the oldest
 * evictable card is flagged exiting so the motion sheet can play it out before the next admit.
 */
export function admit(ring: StreamRing, card: StreamRingCard): StreamRing {
  const cards = [...spliceExited(ring).cards, card];
  const live = cards.filter((entry) => !entry.exiting).length;
  if (live > RING_SIZE) {
    const index = victimIndex(cards);
    if (index !== -1) {
      const victim = cards[index];
      if (victim !== undefined) cards[index] = { ...victim, exiting: true };
    }
  }
  return { ...ring, cards };
}

/** Toggle one file card or strip; when a third opens, the oldest expanded card collapses first */
export function expand(ring: StreamRing, id: number): StreamRing {
  const target = ring.cards.find((card) => card.id === id);
  if (target === undefined || !isExpandable(target)) return ring;
  const opening = !target.expanded;
  let cards = ring.cards.map((card) =>
    isExpandable(card) && card.id === id ? { ...card, expanded: opening } : card
  );
  if (opening) {
    let open = cards.filter((card) => isExpandable(card) && card.expanded).length;
    cards = cards.map((card) => {
      if (open <= MAX_EXPANDED || !isExpandable(card) || !card.expanded || card.id === id) return card;
      open -= 1;
      return { ...card, expanded: false };
    });
  }
  return { ...ring, cards };
}

export function collapseAll(ring: StreamRing): StreamRing {
  return {
    ...ring,
    cards: ring.cards.map((card) =>
      isExpandable(card) && card.expanded ? { ...card, expanded: false } : card
    ),
  };
}

export interface NoticeInput {
  key: string;
  tone: StreamNoticeTone;
  count: number;
  textFor: (count: number) => string;
}

/** Bounded at NOTICE_MAX; a fresh notice with the same key accumulates its count in place */
export function pushNotice(ring: StreamRing, notice: NoticeInput, now: number, id: number): StreamRing {
  const fresh = ring.notices.find(
    (entry) => entry.key === notice.key && now - entry.arrivedAtMs < NOTICE_MERGE_MS
  );
  if (fresh !== undefined) {
    const count = fresh.count + notice.count;
    const merged: StreamNoticeRow = { ...fresh, count, text: notice.textFor(count) };
    return { ...ring, notices: ring.notices.map((entry) => (entry === fresh ? merged : entry)) };
  }
  const row: StreamNoticeRow = {
    kind: "notice",
    id,
    key: notice.key,
    arrivedAtMs: now,
    tone: notice.tone,
    count: notice.count,
    text: notice.textFor(notice.count),
  };
  const notices = [...ring.notices, row];
  return { ...ring, notices: notices.slice(Math.max(0, notices.length - NOTICE_MAX)) };
}

/** Notices older than NOTICE_TTL_MS leave; the same ring comes back when none aged */
export function expireNotices(ring: StreamRing, now: number): StreamRing {
  if (!ring.notices.some((entry) => now - entry.arrivedAtMs >= NOTICE_TTL_MS)) return ring;
  return { ...ring, notices: ring.notices.filter((entry) => now - entry.arrivedAtMs < NOTICE_TTL_MS) };
}

/** Milliseconds until the oldest notice expires, or null with no notices; never negative */
export function nextNoticeExpiryMs(ring: StreamRing, now: number): number | null {
  const oldest = ring.notices[0];
  return oldest === undefined ? null : Math.max(0, oldest.arrivedAtMs + NOTICE_TTL_MS - now);
}

export function openBurst(ring: StreamRing, id: number, untilMs: number): StreamRing {
  return { ...ring, burstOpen: { id, untilMs, paths: new Set(), dirCounts: new Map() } };
}

/** Records the path in the open burst; returns the ring and whether the path was new to it */
export function absorbIntoBurst(
  ring: StreamRing,
  path: string,
  untilMs: number
): { ring: StreamRing; newPath: boolean } {
  if (ring.burstOpen === null) return { ring, newPath: false };
  const newPath = !ring.burstOpen.paths.has(path);
  const paths = new Set(ring.burstOpen.paths);
  paths.add(path);
  const dirCounts = newPath ? countDir(ring.burstOpen.dirCounts, path) : ring.burstOpen.dirCounts;
  return { ring: { ...ring, burstOpen: { ...ring.burstOpen, paths, dirCounts, untilMs } }, newPath };
}

export function closeBurst(ring: StreamRing): StreamRing {
  return ring.burstOpen === null ? ring : { ...ring, burstOpen: null };
}

export function updateBurstCard(
  ring: StreamRing,
  id: number,
  update: (card: StreamBurstCard) => StreamBurstCard
): StreamRing {
  const cards = ring.cards.map((card) => (card.kind === "burst" && card.id === id ? update(card) : card));
  return { ...ring, cards };
}

/** Scrollback rows only ever seed an empty ring; oldest first so the newest sits at the tail */
export function seedHistory(ring: StreamRing, entries: readonly StreamHistorySeed[], firstId: number): StreamRing {
  if (ring.cards.length > 0 || entries.length === 0) return ring;
  const sorted = [...entries]
    .sort((a, b) => Date.parse(b.lastSeenAtIso) - Date.parse(a.lastSeenAtIso))
    .slice(0, HISTORY_ROWS)
    .reverse();
  const cards: StreamRingCard[] = sorted.map((entry, index) => ({
    kind: "history",
    id: firstId + index,
    path: entry.path,
    fileKind: entry.fileKind,
    lastSeenAtIso: entry.lastSeenAtIso,
    exiting: false,
  }));
  return { ...ring, cards };
}

/** The static clock starts at admission, not arrival: a card that waited in the FIFO still prints */
function agedOut(card: StreamRingCard, now: number): boolean {
  return card.kind !== "history" && !card.static && now - card.admittedAtMs >= STATIC_AFTER_MS;
}

/** Cards admitted longer than STATIC_AFTER_MS ago never replay their arrival; same ring when nothing aged */
export function markStatic(ring: StreamRing, now: number): StreamRing {
  if (!ring.cards.some((card) => agedOut(card, now))) return ring;
  return { ...ring, cards: ring.cards.map((card) => (agedOut(card, now) ? { ...card, static: true } : card)) };
}

/** True while a settle tick still has work: an exiting card or a card that will turn static */
export function needsSettle(ring: StreamRing): boolean {
  return ring.cards.some((card) => card.exiting || (card.kind !== "history" && !card.static));
}
