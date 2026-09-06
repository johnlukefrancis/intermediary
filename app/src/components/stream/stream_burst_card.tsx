// Path: app/src/components/stream/stream_burst_card.tsx
// Description: Fixed-height card standing in for a flood of changes: count, span, top dirs, per-op and per-kind strips, resolved

import type React from "react";
import type { StreamBurstCard as BurstCard } from "../../lib/stream/stream_types.js";

interface StreamBurstCardProps {
  card: BurstCard;
  tabIndex: number;
  onFocus: (id: number) => void;
}

function seconds(ms: number): string {
  return `${(ms / 1000).toFixed(1)} s`;
}

export function StreamBurstCard({ card, tabIndex, onFocus }: StreamBurstCardProps): React.JSX.Element {
  const changes = card.byOp.add + card.byOp.modify + card.byOp.remove + card.byOp.rename;
  const dirs = card.topDirs.map((entry) => `${entry.dir} ${String(entry.count)}`).join(" · ");
  return (
    <article
      className="stream-card stream-card--burst"
      data-stream-id={card.id}
      data-kind="burst"
      data-rail="accent"
      data-static={card.static || undefined}
      data-exiting={card.exiting || undefined}
      tabIndex={tabIndex}
      onFocus={() => { onFocus(card.id); }}
      aria-label={`${String(changes)} changes in ${seconds(card.elapsedMs)}`}
    >
      <div className="stream-burst__count">{`×${String(card.files)}`}</div>
      <div className="stream-burst__copy">
        <div className="stream-burst__line">{`${String(changes)} CHANGES IN ${seconds(card.elapsedMs)}`}</div>
        {dirs && <div className="stream-burst__line stream-burst__dirs">{dirs}</div>}
        <div className="stream-burst__line stream-burst__ops">
          <span data-op="add">{`A ${String(card.byOp.add)}`}</span>
          <span data-op="modify">{`M ${String(card.byOp.modify)}`}</span>
          <span data-op="remove">{`D ${String(card.byOp.remove)}`}</span>
          {card.byOp.rename > 0 && <span data-op="rename">{`R ${String(card.byOp.rename)}`}</span>}
        </div>
        <div className="stream-burst__line stream-burst__kinds">
          <span data-file-kind="code">{`CODE ${String(card.byKind.code)}`}</span>
          <span data-file-kind="docs">{`DOCS ${String(card.byKind.docs)}`}</span>
          <span data-file-kind="image">{`IMG ${String(card.byKind.image)}`}</span>
        </div>
      </div>
      <div className="stream-burst__resolved">{`${String(card.resolved)} RESOLVED`}</div>
    </article>
  );
}
