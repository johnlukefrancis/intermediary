// Path: app/src/components/overview/use_overview_follow.ts
// Description: Overview tail-follow with a bounded unread set and no private timer

import { useCallback, useLayoutEffect, useRef, useState, type RefObject } from "react";
import type { OverviewFeedEntry } from "./overview_feed_entries.js";

export function useOverviewFollow(scroller: RefObject<HTMLDivElement | null>, entries: readonly OverviewFeedEntry[], viewKey: string, frozen: boolean): {
  unread: number; onScroll: () => void; resume: () => void;
} {
  const pinned = useRef(true);
  const seen = useRef(new Set<string>());
  const landedView = useRef<string | null>(null);
  const [unread, setUnread] = useState(0);

  const acknowledge = useCallback(() => {
    seen.current = new Set(entries.map((entry) => entry.key));
    setUnread(0);
  }, [entries]);

  const onScroll = useCallback(() => {
    const element = scroller.current;
    if (element === null) return;
    pinned.current = element.scrollHeight - element.scrollTop - element.clientHeight <= 24;
    if (pinned.current) acknowledge();
  }, [acknowledge, scroller]);

  const resume = useCallback(() => {
    const element = scroller.current;
    if (element !== null) element.scrollTop = element.scrollHeight;
    pinned.current = true;
    acknowledge();
  }, [acknowledge, scroller]);

  useLayoutEffect(() => {
    const element = scroller.current;
    if (element === null) return;
    if (landedView.current !== viewKey || (pinned.current && !frozen)) {
      landedView.current = viewKey;
      pinned.current = true;
      element.scrollTop = element.scrollHeight;
      acknowledge();
      return;
    }
    setUnread(entries.filter((entry) => !seen.current.has(entry.key) && entry.kind !== "notice").length);
  }, [acknowledge, entries, frozen, scroller, viewKey]);

  return { unread, onScroll, resume };
}
