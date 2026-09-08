// Path: app/src/hooks/stream/use_stream_follow.ts
// Description: Follow-scroll for the Stream scroller: tail pin, unread count while unpinned, freeze, resume

import { useCallback, useLayoutEffect, useRef, useState, type RefObject } from "react";
import { FOLLOW_EPSILON_PX } from "../../lib/stream/stream_bounds.js";
import type { StreamSnapshot } from "../../lib/stream/stream_types.js";

export interface StreamFollow {
  pinned: boolean;
  /** Pinned but held still by a focused or expanded card: arrivals land unseen below */
  frozen: boolean;
  /** Cards admitted since the reader left the tail */
  unread: number;
  onScroll: () => void;
  /** Jump to the tail and pin again */
  resume: () => void;
}

function newestCardId(snapshot: StreamSnapshot): number {
  let max = 0;
  for (const card of snapshot.ring.cards) {
    if (card.kind !== "history" && card.id > max) max = card.id;
  }
  return max;
}

function isAtTail(element: HTMLElement): boolean {
  return element.scrollHeight - element.scrollTop - element.clientHeight <= FOLLOW_EPSILON_PX;
}

/**
 * While pinned and not frozen, every snapshot change lands the tail in a layout effect (no smooth
 * scroll, so a card never arrives mid-glide). A focused or expanded card freezes the pin so
 * reading is never yanked; `resume` releases it. The freeze protects a reading position, and a
 * fresh scroller (repo tab switch, workspace closed, STREAM mode returning) has none: its first
 * landing always takes the tail, however many cards the store still holds expanded.
 */
export function useStreamFollow(
  scrollerRef: RefObject<HTMLDivElement | null>,
  snapshot: StreamSnapshot,
  frozen: boolean
): StreamFollow {
  const [pinned, setPinned] = useState(true);
  const [unread, setUnread] = useState(0);
  const pinnedRef = useRef(true);
  const seenIdRef = useRef(0);
  /** False until this scroller has landed its first tail; a freeze only holds after that */
  const landedRef = useRef(false);

  const pinTo = useCallback((next: boolean) => {
    if (pinnedRef.current === next) return;
    pinnedRef.current = next;
    setPinned(next);
  }, []);

  const onScroll = useCallback(() => {
    const element = scrollerRef.current;
    if (!element) return;
    const atTail = isAtTail(element);
    pinTo(atTail);
    if (atTail) {
      seenIdRef.current = newestCardId(snapshot);
      setUnread(0);
    }
  }, [pinTo, scrollerRef, snapshot]);

  const resume = useCallback(() => {
    const element = scrollerRef.current;
    if (element) element.scrollTop = element.scrollHeight;
    pinTo(true);
    seenIdRef.current = newestCardId(snapshot);
    setUnread(0);
  }, [pinTo, scrollerRef, snapshot]);

  useLayoutEffect(() => {
    const element = scrollerRef.current;
    if (!element) return;
    const newest = newestCardId(snapshot);
    if (pinnedRef.current && (!frozen || !landedRef.current)) {
      landedRef.current = true;
      element.scrollTop = element.scrollHeight;
      seenIdRef.current = newest;
      setUnread(0);
      return;
    }
    let count = 0;
    for (const card of snapshot.ring.cards) {
      if (card.kind !== "history" && card.id > seenIdRef.current) count += 1;
    }
    setUnread(count);
  }, [frozen, scrollerRef, snapshot]);

  return { pinned, frozen, unread, onScroll, resume };
}
