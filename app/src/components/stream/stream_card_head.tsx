// Path: app/src/components/stream/stream_card_head.tsx
// Description: The one-line card head: badge, file icon, name over dir, +N −M, baseline chip, edit count, clock

import type React from "react";
import { baseName, parentPath } from "../../lib/bundles/bundle_selection_visibility.js";
import { getFileFamily, FileIcon } from "../../lib/icons/index.js";
import { CHANGE_BADGES } from "../../lib/source_control/change_badges.js";
import { badgeFor, baselineLabel } from "../../lib/stream/stream_card_grammar.js";
import type { StreamFileCard } from "../../lib/stream/stream_types.js";

export function StreamCardHead({ card }: { card: StreamFileCard }): React.JSX.Element {
  const badge = CHANGE_BADGES[badgeFor(card.op, card.tracked, card.body.status === "opaque")];
  const chip = baselineLabel(card);
  const stats = card.body.status === "text" ? card.body.stats : null;
  const truncated = card.body.status === "text" && card.body.truncated;
  const dir = parentPath(card.path);

  return (
    <header className="stream-card__head">
      <span className={`badge badge--${badge.variant}`} title={badge.label}>{badge.letter}</span>
      <span className="stream-card__icon"><FileIcon family={getFileFamily(card.path)} /></span>
      <span className="stream-card__path">
        <span className="stream-card__name">
          {card.fromPath !== null && (
            <span className="stream-card__from">{`${baseName(card.fromPath)} → `}</span>
          )}
          {baseName(card.path)}
        </span>
        {dir && <span className="stream-card__dir" title={card.path}>{dir}</span>}
      </span>
      <span className="stream-card__meta">
        {stats !== null && stats.added + stats.removed > 0 && (
          <span className="stream-card__stats">
            <span className="stream-card__stat" data-stat="added">{`+${String(stats.added)}`}</span>
            <span className="stream-card__stat" data-stat="removed">{`−${String(stats.removed)}`}</span>
          </span>
        )}
        {truncated && <span className="badge badge--warning" title="The agent cut this patch at its byte bound">TRUNC</span>}
        {chip !== null && <span className="stream-card__chip">{chip}</span>}
        {card.outsideSelection && (
          <span className="stream-card__chip" data-chip="outside" title="Outside the active ZIP selection">OUTSIDE SELECTION</span>
        )}
        {card.edits > 1 && <span className="stream-card__edits" title={`${String(card.edits)} edits merged`}>{`×${String(card.edits)}`}</span>}
      </span>
      <span className="stream-card__clock">{card.clock}</span>
    </header>
  );
}
