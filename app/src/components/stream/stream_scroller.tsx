// Path: app/src/components/stream/stream_scroller.tsx
// Description: The scroll container: ring in order, roving focus, follow pill, and a throttled screen-reader digest

import type React from "react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { StreamOpenTarget } from "../../hooks/stream/use_repo_stream.js";
import { useStreamFollow } from "../../hooks/stream/use_stream_follow.js";
import type { StreamImageTiles } from "../../hooks/stream/use_stream_images.js";
import type { FileTypeFilter } from "../../lib/files/file_feed.js";
import { DIGEST_THROTTLE_MS } from "../../lib/stream/stream_bounds.js";
import { isExpandable } from "../../lib/stream/stream_ring.js";
import { stripCountLabel, stripDirLabel } from "../../lib/stream/stream_strip_view.js";
import type { StreamRingCard, StreamSnapshot, StreamStripTile } from "../../lib/stream/stream_types.js";
import { StreamBurstCard } from "./stream_burst_card.js";
import { StreamCard } from "./stream_card.js";
import { StreamFollowPill } from "./stream_follow_pill.js";
import { StreamHistoryRow } from "./stream_history_row.js";
import { StreamImageStrip } from "./stream_image_strip.js";
import { StreamNoticeRow } from "./stream_notice_row.js";

interface StreamScrollerProps {
  snapshot: StreamSnapshot;
  /** Image tiles by strip and path, owned by the panel */
  tiles: StreamImageTiles;
  filter: FileTypeFilter;
  handset: boolean;
  onExpand: (id: number) => void;
  onOpen: (target: StreamOpenTarget) => void;
  onDrag: (path: string) => void;
  onContextMenu: (event: React.MouseEvent, path: string) => void;
}

type Entry = StreamRingCard | StreamSnapshot["ring"]["notices"][number];

/** Every ring card renders; the type filter only hides (data-filtered), so the ring is never unmounted */
function entriesOf(snapshot: StreamSnapshot): Entry[] {
  const { cards, notices } = snapshot.ring;
  if (notices.length === 0) return [...cards];
  const arrivedAt = (entry: Entry): number => (entry.kind === "history" ? 0 : entry.arrivedAtMs);
  return [...cards, ...notices].sort((a, b) => arrivedAt(a) - arrivedAt(b) || a.id - b.id);
}

/** A card the reader can see and focus under the current filter; notices never take focus */
function focusable(entry: Entry, filter: FileTypeFilter): entry is StreamRingCard {
  if (entry.kind === "notice") return false;
  if (filter === "all" || entry.kind === "burst") return true;
  if (entry.kind === "images") return filter === "image";
  return entry.fileKind === filter;
}

/** What the newest live card is, for the screen-reader digest */
function describe(card: StreamRingCard): string | null {
  if (card.kind === "file") return card.path;
  if (card.kind === "images") {
    const dir = stripDirLabel(card.tiles);
    return `${stripCountLabel(card.tiles.length).toLowerCase()}${dir ? ` in ${dir}` : ""}`;
  }
  return null;
}

function useDigest(snapshot: StreamSnapshot): string {
  const [digest, setDigest] = useState("");
  const lastAtRef = useRef(0);
  const timerRef = useRef<number | null>(null);
  useEffect(() => {
    const speak = (): void => {
      lastAtRef.current = Date.now();
      const newest = [...snapshot.ring.cards].reverse().map(describe).find((text) => text !== null);
      setDigest(newest === undefined ? "" : `${String(snapshot.ring.cards.length)} entries in the stream; latest ${newest}`);
    };
    const wait = DIGEST_THROTTLE_MS - (Date.now() - lastAtRef.current);
    if (wait <= 0) {
      speak();
      return;
    }
    timerRef.current = window.setTimeout(speak, wait);
    return () => {
      if (timerRef.current !== null) window.clearTimeout(timerRef.current);
      timerRef.current = null;
    };
  }, [snapshot]);
  return digest;
}

export function StreamScroller({ snapshot, tiles, filter, handset, onExpand, onOpen, onDrag, onContextMenu }: StreamScrollerProps): React.JSX.Element {
  const scrollerRef = useRef<HTMLDivElement | null>(null);
  const [focusedId, setFocusedId] = useState<number | null>(null);
  const entries = useMemo(() => entriesOf(snapshot), [snapshot]);
  // A focused card evicted from the ring must not keep the follow frozen or hold the tab stop
  const activeId = focusedId !== null && entries.some((entry) => focusable(entry, filter) && entry.id === focusedId)
    ? focusedId
    : null;
  useEffect(() => {
    if (focusedId !== null && activeId === null) setFocusedId(null);
  }, [activeId, focusedId]);
  const anyExpanded = snapshot.ring.cards.some((card) => isExpandable(card) && card.expanded);
  const follow = useStreamFollow(scrollerRef, snapshot, activeId !== null || anyExpanded);
  const digest = useDigest(snapshot);

  const focusIndex = (index: number): void => {
    const target = entries[index];
    if (target === undefined || !focusable(target, filter)) return;
    scrollerRef.current?.querySelector<HTMLElement>(`[data-stream-id="${String(target.id)}"]`)?.focus();
  };

  const onKeyDown = (event: React.KeyboardEvent): void => {
    if (!(event.target instanceof Element)) return;
    // A card's own controls (OPEN DIFF) keep their Space and Enter
    if (event.target.closest("button") !== null) return;
    const host = event.target.closest<HTMLElement>("[data-stream-id]");
    if (host === null) return;
    const id = Number(host.dataset["streamId"]);
    const index = entries.findIndex((entry) => entry.kind !== "notice" && entry.id === id);
    const entry = entries[index];
    if (entry === undefined || entry.kind === "notice") return;
    const step = (from: number, by: number): number => {
      let next = from + by;
      while (next >= 0 && next < entries.length) {
        const candidate = entries[next];
        if (candidate !== undefined && focusable(candidate, filter)) break;
        next += by;
      }
      return next;
    };
    switch (event.key) {
      case "ArrowDown": focusIndex(step(index, 1)); break;
      case "ArrowUp": focusIndex(step(index, -1)); break;
      case "Home": focusIndex(step(-1, 1)); break;
      case "End": focusIndex(step(entries.length, -1)); break;
      // A strip owns Enter (it opens the selected tile) and never reaches here with it
      case "Enter": onOpen(entry); break;
      // The same predicate the pointer path uses: the card writes data-expandable only when it has something to unfold
      case " ": if (isExpandable(entry) && host.dataset["expandable"] !== undefined) onExpand(entry.id); break;
      case "Escape": host.blur(); setFocusedId(null); follow.resume(); break;
      default: return;
    }
    event.preventDefault();
  };

  const onFocus = useCallback((id: number) => { setFocusedId(id); }, []);
  const onOpenTile = useCallback((tile: StreamStripTile) => { onOpen({ kind: "tile", tile }); }, [onOpen]);
  const onBlur = (event: React.FocusEvent): void => {
    if (!(event.relatedTarget instanceof Node) || !event.currentTarget.contains(event.relatedTarget)) setFocusedId(null);
  };
  const lastFocusable = [...entries].reverse().find((entry) => focusable(entry, filter));
  const tabIndexFor = (id: number): number => (activeId === id || (activeId === null && lastFocusable?.id === id) ? 0 : -1);

  return (
    <div
      ref={scrollerRef}
      className="stream-scroller"
      role="log"
      aria-live="off"
      aria-label="Edit stream"
      data-pressure={snapshot.pressure}
      onScroll={follow.onScroll}
      onKeyDown={onKeyDown}
      onBlur={onBlur}
    >
      {entries.map((entry) => {
        switch (entry.kind) {
          case "file":
            return <StreamCard key={entry.id} card={entry} handset={handset} filtered={!focusable(entry, filter)} tabIndex={tabIndexFor(entry.id)} onFocus={onFocus} onExpand={onExpand} onOpen={onOpen} onDrag={onDrag} onContextMenu={onContextMenu} />;
          case "images":
            return <StreamImageStrip key={entry.id} card={entry} tiles={tiles} filtered={!focusable(entry, filter)} handset={handset} tabIndex={tabIndexFor(entry.id)} onFocus={onFocus} onExpand={onExpand} onOpenTile={onOpenTile} onDrag={onDrag} onContextMenu={onContextMenu} />;
          case "burst":
            return <StreamBurstCard key={entry.id} card={entry} tabIndex={tabIndexFor(entry.id)} onFocus={onFocus} />;
          case "history":
            return <StreamHistoryRow key={entry.id} id={entry.id} path={entry.path} fileKind={entry.fileKind} lastSeenAtIso={entry.lastSeenAtIso} filtered={!focusable(entry, filter)} tabIndex={tabIndexFor(entry.id)} onFocus={onFocus} onOpen={() => { onOpen(entry); }} onContextMenu={onContextMenu} />;
          case "notice":
            return <StreamNoticeRow key={`n${String(entry.id)}`} notice={entry} />;
        }
      })}
      {snapshot.settling.length > 0 && (
        <div className="stream-settling" aria-hidden="true">{`SETTLING ${snapshot.settling.map((path) => path.split("/").at(-1) ?? path).join(" · ")}`}</div>
      )}
      {follow.unread > 0 && (!follow.pinned || follow.frozen) && <StreamFollowPill unread={follow.unread} onResume={follow.resume} />}
      <div className="sr-only" role="status">{digest}</div>
    </div>
  );
}
