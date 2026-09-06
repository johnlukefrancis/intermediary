// Path: app/src/components/stream/stream_image_strip.tsx
// Description: The image strip card: head with count and op tally, the growing-then-wrapping tile grid, clock-span footer, tile selection keys

import type React from "react";
import { memo, useCallback, useState } from "react";
import { useDeferredClick } from "../../hooks/stream/use_deferred_click.js";
import type { StreamImageTiles } from "../../hooks/stream/use_stream_images.js";
import { getFileFamily, FileIcon } from "../../lib/icons/index.js";
import { baseName } from "../../lib/bundles/bundle_selection_visibility.js";
import { STRIP_MIN_COLUMNS, STRIP_SLOT_MAX_PX, STRIP_TILE_HANDSET_PX, STRIP_TILE_PX } from "../../lib/stream/stream_bounds.js";
import { formatBytes } from "../../lib/stream/stream_card_grammar.js";
import {
  stripClockSpan,
  stripCountLabel,
  stripDirLabel,
  stripNewestClock,
  stripOpTally,
  stripRail,
  stripTotalBytes,
} from "../../lib/stream/stream_strip_view.js";
import { tileKey } from "../../lib/stream/stream_tile_targets.js";
import type { StreamImageStripCard, StreamStripTile } from "../../lib/stream/stream_types.js";
import { StreamImageTile } from "./stream_image_tile.js";

export interface StreamImageStripProps {
  card: StreamImageStripCard;
  /** Every tile's pixels, keyed by `tileKey(repoId, card.id, path)`; owned by the panel */
  tiles: StreamImageTiles;
  /** Hidden by the type filter: the strip stays mounted so switching back reveals it in place */
  filtered: boolean;
  /** Handset chassis: tiles take the narrower column minimum so three or four still seat per row */
  handset: boolean;
  tabIndex: number;
  onFocus: (id: number) => void;
  onExpand: (id: number) => void;
  onOpenTile: (tile: StreamStripTile) => void;
  onDrag: (path: string) => void;
  onContextMenu: (event: React.MouseEvent, path: string) => void;
}

const OP_LETTERS = { add: "A", modify: "M", remove: "D", rename: "R" } as const;

/** Expansion pairs BEFORE/AFTER, so it is offered only when some modified tile still has a BEFORE */
function anyPairable(card: StreamImageStripCard, tiles: StreamImageTiles): boolean {
  return card.tiles.some(
    (tile) => tile.op === "modify" && tile.body.status === "image" && (tiles.byKey.get(tileKey(tiles.repoId, card.id, tile.path))?.beforeUrl ?? null) !== null
  );
}

/** The strip's own layout numbers, published inline from stream_bounds.ts for stream_card_image.css */
function stripStyle(handset: boolean): React.CSSProperties {
  return {
    "--stream-strip-tile": `${String(handset ? STRIP_TILE_HANDSET_PX : STRIP_TILE_PX)}px`,
    "--stream-strip-slot-max": `${String(STRIP_SLOT_MAX_PX)}px`,
    "--stream-strip-min-columns": String(STRIP_MIN_COLUMNS),
  } as React.CSSProperties;
}

function StreamImageStripView({ card, tiles, filtered, handset, tabIndex, onFocus, onExpand, onOpenTile, onDrag, onContextMenu }: StreamImageStripProps): React.JSX.Element {
  const [selected, setSelected] = useState(0);
  const count = card.tiles.length;
  const tally = stripOpTally(card.tiles);
  const dir = stripDirLabel(card.tiles);
  const expandable = anyPairable(card, tiles) || card.expanded;
  const expand = useCallback(() => { onExpand(card.id); }, [card.id, onExpand]);
  const deferred = useDeferredClick(expand);
  const current = Math.min(selected, Math.max(0, count - 1));
  const currentPath = card.tiles[current]?.path;
  // The selected tile is mirrored here so a reader hears which tile Left/Right landed on
  const label = `${stripCountLabel(count).toLowerCase()}${dir ? ` in ${dir}` : ""}${currentPath === undefined ? "" : `, selected ${baseName(currentPath)}`}`;

  /** Left/Right/Enter belong to the strip; Up/Down/Home/End/Escape/Space bubble to the scroller */
  const onKeyDown = (event: React.KeyboardEvent): void => {
    switch (event.key) {
      case "ArrowLeft": setSelected(Math.max(0, current - 1)); break;
      case "ArrowRight": setSelected(Math.min(count - 1, current + 1)); break;
      case "Enter": {
        const tile = card.tiles[current];
        if (tile !== undefined) onOpenTile(tile);
        break;
      }
      default: return;
    }
    event.preventDefault();
    event.stopPropagation();
  };

  // A tile's double-click bubbles to the article, so the single click it armed is dropped there
  return (
    <article
      className="stream-card stream-card--strip"
      data-stream-id={card.id}
      data-kind="images"
      data-rail={stripRail(card.tiles)}
      data-static={card.static || undefined}
      data-exiting={card.exiting || undefined}
      data-expanded={card.expanded || undefined}
      data-expandable={expandable || undefined}
      data-filtered={filtered || undefined}
      style={stripStyle(handset)}
      tabIndex={tabIndex}
      aria-label={label}
      onFocus={() => { onFocus(card.id); }}
      onKeyDown={onKeyDown}
      onClick={(event) => {
        if (event.detail === 1 && expandable) deferred.click();
      }}
      onDoubleClick={deferred.cancel}
      title={expandable ? "Click to pair BEFORE/AFTER; double-click a tile to open it" : "Double-click a tile to open it; drag or right-click a tile for file actions"}
    >
      <header className="stream-card__head">
        <span className="stream-card__icon"><FileIcon family={getFileFamily(card.tiles[0]?.path ?? "image.png")} /></span>
        <span className="stream-card__path">
          <span className="stream-card__name">{stripCountLabel(count)}</span>
          {dir && <span className="stream-card__dir" title={dir}>{dir}</span>}
        </span>
        <span className="stream-card__meta stream-burst__ops">
          {(["add", "modify", "remove", "rename"] as const).map((op) =>
            tally[op] > 0 ? <span key={op} data-op={op}>{`${OP_LETTERS[op]} ${String(tally[op])}`}</span> : null
          )}
        </span>
        <span className="stream-card__clock">{stripNewestClock(card.tiles)}</span>
      </header>
      <div className="stream-strip__row" role="list">
        {card.tiles.map((tile, index) => (
          <StreamImageTile
            key={tile.path}
            tile={tile}
            chain={card.static ? 0 : index}
            pixels={tiles.byKey.get(tileKey(tiles.repoId, card.id, tile.path))}
            expanded={card.expanded}
            fresh={card.static && tile.updatedAtMs === card.updatedAtMs}
            selected={index === current}
            onOpen={onOpenTile}
            onDrag={onDrag}
            onContextMenu={onContextMenu}
          />
        ))}
      </div>
      <footer className="stream-card__foot">
        <span className="stream-card__foot-note">{`${stripClockSpan(card.tiles)} · ${formatBytes(stripTotalBytes(card.tiles))}`}</span>
      </footer>
    </article>
  );
}

/** Re-renders when the strip's own facts, its pixels, or the handlers move */
export const StreamImageStrip = memo(StreamImageStripView, (previous, next) =>
  previous.card.id === next.card.id &&
  previous.card.updatedAtMs === next.card.updatedAtMs &&
  previous.card.expanded === next.card.expanded &&
  previous.card.static === next.card.static &&
  previous.card.exiting === next.card.exiting &&
  previous.card.tiles === next.card.tiles &&
  previous.tiles === next.tiles &&
  previous.filtered === next.filtered &&
  previous.handset === next.handset &&
  previous.tabIndex === next.tabIndex &&
  previous.onFocus === next.onFocus &&
  previous.onExpand === next.onExpand &&
  previous.onOpenTile === next.onOpenTile &&
  previous.onDrag === next.onDrag &&
  previous.onContextMenu === next.onContextMenu
);
