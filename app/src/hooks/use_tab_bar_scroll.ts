// Path: app/src/hooks/use_tab_bar_scroll.ts
// Description: Scroll overflow detection, snap-to-next-tab, and active-tab reveal for the tab bar track

import { useRef, useState, useEffect, useCallback } from "react";

interface ScrollState {
  isOverflowing: boolean;
  canScrollLeft: boolean;
  canScrollRight: boolean;
}

interface UseTabBarScrollResult {
  trackRef: React.RefObject<HTMLDivElement>;
  scrollState: ScrollState;
  scrollLeft: () => void;
  scrollRight: () => void;
}

/** Selector matching the flex-shrink:0 tab containers inside the track */
const TAB_SELECTOR = ".single-tab-container, .group-tab-container";

function readScroll(el: HTMLElement): ScrollState {
  const { scrollLeft, scrollWidth, clientWidth } = el;
  const isOverflowing = scrollWidth > clientWidth + 1;
  return {
    isOverflowing,
    canScrollLeft: scrollLeft > 1,
    canScrollRight: scrollLeft + clientWidth < scrollWidth - 1,
  };
}

export function useTabBarScroll(activeRepoId: string | null): UseTabBarScrollResult {
  const trackRef = useRef<HTMLDivElement>(null);
  const [scrollState, setScrollState] = useState<ScrollState>({
    isOverflowing: false,
    canScrollLeft: false,
    canScrollRight: false,
  });

  const sync = useCallback(() => {
    const el = trackRef.current;
    if (!el) return;
    setScrollState(readScroll(el));
  }, []);

  useEffect(() => {
    const el = trackRef.current;
    if (!el) return;

    const ro = new ResizeObserver(sync);
    ro.observe(el);
    el.addEventListener("scroll", sync, { passive: true });
    sync();

    return () => {
      ro.disconnect();
      el.removeEventListener("scroll", sync);
    };
  }, [sync]);

  /* Auto-scroll the active tab into view */
  useEffect(() => {
    const track = trackRef.current;
    if (!activeRepoId || !track) return;
    const activeEl = track.querySelector<HTMLElement>("[aria-current='page']");
    if (!activeEl) return;
    const trackRect = track.getBoundingClientRect();
    const elRect = activeEl.getBoundingClientRect();
    if (elRect.left < trackRect.left) {
      track.scrollBy({ left: elRect.left - trackRect.left - 8, behavior: "smooth" });
    } else if (elRect.right > trackRect.right) {
      track.scrollBy({ left: elRect.right - trackRect.right + 8, behavior: "smooth" });
    }
  }, [activeRepoId]);

  const scrollLeft = useCallback(() => {
    const track = trackRef.current;
    if (!track) return;
    const trackLeft = track.getBoundingClientRect().left;
    const tabs = track.querySelectorAll<HTMLElement>(TAB_SELECTOR);

    // Find the rightmost tab whose left edge is off-screen to the left
    for (let i = tabs.length - 1; i >= 0; i--) {
      const tab = tabs[i];
      if (!tab) continue;
      const rect = tab.getBoundingClientRect();
      if (rect.left < trackLeft - 1) {
        const target = tab.offsetLeft + tab.offsetWidth - track.clientWidth;
        track.scrollTo({ left: Math.max(0, target), behavior: "smooth" });
        return;
      }
    }
  }, []);

  const scrollRight = useCallback(() => {
    const track = trackRef.current;
    if (!track) return;
    const trackRight = track.getBoundingClientRect().right;
    const tabs = track.querySelectorAll<HTMLElement>(TAB_SELECTOR);

    // Find the first tab whose right edge is off-screen to the right
    for (const tab of tabs) {
      const rect = tab.getBoundingClientRect();
      if (rect.right > trackRight + 1) {
        const maxScroll = track.scrollWidth - track.clientWidth;
        track.scrollTo({ left: Math.min(tab.offsetLeft, maxScroll), behavior: "smooth" });
        return;
      }
    }
  }, []);

  return { trackRef, scrollState, scrollLeft, scrollRight };
}
