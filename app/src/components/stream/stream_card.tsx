// Path: app/src/components/stream/stream_card.tsx
// Description: Memoized file card chassis: spine, head, body dispatch, click / double-click / drag / right-click / keys

import type React from "react";
import { memo, useCallback } from "react";
import { useDeferredClick } from "../../hooks/stream/use_deferred_click.js";
import { useDragOutPointer } from "../../hooks/use_drag_out_pointer.js";
import { cardKind, formatBytes, lineCap, railVariant, selectLines } from "../../lib/stream/stream_card_grammar.js";
import type { StreamCardBody, StreamFileCard } from "../../lib/stream/stream_types.js";
import { StreamCardHead } from "./stream_card_head.js";
import { StreamTextBody } from "./stream_text_body.js";

export interface StreamCardProps {
  card: StreamFileCard;
  handset: boolean;
  /** Hidden by the type filter: the card stays mounted so switching back reveals it in place */
  filtered: boolean;
  tabIndex: number;
  onFocus: (id: number) => void;
  onExpand: (id: number) => void;
  onOpen: (card: StreamFileCard) => void;
  onDrag: (path: string) => void;
  onContextMenu: (event: React.MouseEvent, path: string) => void;
}

/** Ghost bodies for what has no lines to print; image pixels live in strips, never in a file card */
function ghostText(body: Exclude<StreamCardBody, { status: "text" }>): string {
  switch (body.status) {
    case "image":
      return `IMAGE · ${formatBytes(body.bytes)}`;
    case "opaque":
      if (body.reason === "tooLarge") return "TOO LARGE FOR STREAM";
      if (body.reason === "binary") return `BINARY · ${formatBytes(body.bytes)}`;
      return "UNREADABLE";
    case "gone":
      return "REMOVED";
  }
}

/** Printed line count (the print stagger's divisor) and what the cap hid (the expand affordance) */
function selectionOf(card: StreamFileCard, handset: boolean): { printed: number; hidden: number } {
  if (card.body.status !== "text") return { printed: 0, hidden: 0 };
  const selection = selectLines(card.body.segments, card.expanded, lineCap(handset));
  return { printed: selection.lines.length, hidden: selection.hiddenLines + card.body.beyondCap };
}

function StreamCardView({ card, handset, filtered, tabIndex, onFocus, onExpand, onOpen, onDrag, onContextMenu }: StreamCardProps): React.JSX.Element {
  const kind = cardKind(card);
  const selection = selectionOf(card, handset);
  const expandable = selection.hidden > 0 || card.expanded;
  const handleDrag = useCallback(() => { onDrag(card.path); }, [card.path, onDrag]);
  const pointer = useDragOutPointer({ onDragStart: handleDrag, enabled: card.body.status !== "gone" });
  const expand = useCallback(() => { onExpand(card.id); }, [card.id, onExpand]);
  const deferred = useDeferredClick(expand);

  return (
    <article
      className="stream-card"
      data-stream-id={card.id}
      data-kind={kind}
      data-rail={railVariant(kind)}
      data-content={card.body.status}
      data-static={card.static || undefined}
      data-exiting={card.exiting || undefined}
      data-expanded={card.expanded || undefined}
      data-expandable={expandable || undefined}
      data-outside-selection={card.outsideSelection || undefined}
      data-filtered={filtered || undefined}
      style={{ "--stream-line-count": String(selection.printed) } as React.CSSProperties}
      tabIndex={tabIndex}
      onFocus={() => { onFocus(card.id); }}
      onClick={(event) => {
        if (event.detail === 1 && expandable) deferred.click();
      }}
      onDoubleClick={() => {
        deferred.cancel();
        onOpen(card);
      }}
      onContextMenu={(event) => {
        event.preventDefault();
        onContextMenu(event, card.path);
      }}
      {...pointer}
      title="Click to expand; double-click to open; drag to stage for handoff; right-click for file actions"
    >
      <StreamCardHead card={card} />
      {card.body.status === "text" ? (
        <StreamTextBody card={card} body={card.body} handset={handset} onOpen={onOpen} />
      ) : (
        <div className="stream-card__ghost" data-ghost={card.body.status}>{ghostText(card.body)}</div>
      )}
    </article>
  );
}

/** Re-renders only when the card's own facts move; handlers must be stable */
export const StreamCard = memo(StreamCardView, (previous, next) =>
  previous.card.id === next.card.id &&
  previous.card.updatedAtMs === next.card.updatedAtMs &&
  previous.card.expanded === next.card.expanded &&
  previous.card.static === next.card.static &&
  previous.card.exiting === next.card.exiting &&
  previous.card.outsideSelection === next.card.outsideSelection &&
  previous.handset === next.handset &&
  previous.filtered === next.filtered &&
  previous.tabIndex === next.tabIndex &&
  previous.onFocus === next.onFocus &&
  previous.onExpand === next.onExpand &&
  previous.onOpen === next.onOpen &&
  previous.onDrag === next.onDrag &&
  previous.onContextMenu === next.onContextMenu
);
