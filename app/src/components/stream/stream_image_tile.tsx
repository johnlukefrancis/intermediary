// Path: app/src/components/stream/stream_image_tile.tsx
// Description: One tile of an image strip: a 16:10 checkerboard slot sized by its column, badge, edit counter, name, the BEFORE/AFTER pair

import type React from "react";
import { useCallback } from "react";
import type { StreamImageTile as TilePixels } from "../../hooks/stream/use_stream_images.js";
import { getExtension } from "../../hooks/repo_workspace_types.js";
import { useDragOutPointer } from "../../hooks/use_drag_out_pointer.js";
import { baseName } from "../../lib/bundles/bundle_selection_visibility.js";
import { CHANGE_BADGES } from "../../lib/source_control/change_badges.js";
import { IMAGE_CARD_MAX_BYTES } from "../../lib/stream/stream_bounds.js";
import { badgeFor, formatBytes, previewableImage } from "../../lib/stream/stream_card_grammar.js";
import type { StreamStripTile } from "../../lib/stream/stream_types.js";

export interface StreamImageTileProps {
  tile: StreamStripTile;
  /** The drop stagger's --stream-chain: the reading-order index, or 0 for a tile appended to a static strip */
  chain: number;
  /** Owned by useStreamImages; undefined until the panel has seen this slot */
  pixels: TilePixels | undefined;
  /** The strip is expanded: a modified tile with a retained BEFORE becomes a pair */
  expanded: boolean;
  /** The newest tile of a strip that already went static: it drops alone */
  fresh: boolean;
  selected: boolean;
  onOpen: (tile: StreamStripTile) => void;
  onDrag: (path: string) => void;
  onContextMenu: (event: React.MouseEvent, path: string) => void;
}

/** Why no pixels were ever requested: the size that failed the gate, or the extension with no mime */
function noPreviewReason(path: string, bytes: number): string {
  if (bytes > IMAGE_CARD_MAX_BYTES) return formatBytes(bytes);
  return getExtension(path)?.toUpperCase() ?? "UNSUPPORTED";
}

/** The slot's note when it holds no pixels; the slot's size follows its column either way, never its content */
function slotNote(tile: StreamStripTile, pixels: TilePixels | undefined): string {
  if (tile.body.status === "gone" || tile.op === "remove") return "DELETED";
  if (!previewableImage(tile.path, tile.body.mimeType, tile.body.bytes)) {
    return `NO PREVIEW · ${noPreviewReason(tile.path, tile.body.bytes)}`;
  }
  switch (pixels?.status) {
    case "dropped":
      return "RELEASED";
    case "error":
      return "PREVIEW FAILED";
    // The file moved on past the revision this card announced; its next delta brings its own tile
    case "superseded":
      return "IMAGE CHANGED";
    case "tooLarge":
      return "NO PREVIEW · TOO LARGE";
    default:
      return "";
  }
}

interface SlotProps {
  url: string | null;
  alt: string;
  note: string;
  label: string | null;
  /** Decoded size of `url` when known: the CSS caps the thumb at twice it so an icon is never blown up */
  decoded: { width: number; height: number } | null;
}

function Slot({ url, alt, note, label, decoded }: SlotProps): React.JSX.Element {
  const thumbStyle = decoded === null
    ? undefined
    : ({ "--stream-thumb-w": `${String(decoded.width)}px`, "--stream-thumb-h": `${String(decoded.height)}px` } as React.CSSProperties);
  return (
    <span className="stream-strip__slot" data-slot={label?.toLowerCase() ?? "only"}>
      {url !== null ? (
        <img className="stream-card__thumb" src={url} alt={alt} draggable={false} style={thumbStyle} />
      ) : (
        note && <span className="stream-strip__note">{note}</span>
      )}
      {label !== null && <span className="stream-strip__label">{label}</span>}
    </span>
  );
}

export function StreamImageTile({ tile, chain, pixels, expanded, fresh, selected, onOpen, onDrag, onContextMenu }: StreamImageTileProps): React.JSX.Element {
  const badge = CHANGE_BADGES[badgeFor(tile.op, tile.tracked)];
  const deleted = tile.body.status === "gone" || tile.op === "remove";
  // A deleted tile shows the pixels it had (its own, else its BEFORE) greyed and struck
  const afterUrl = pixels?.url ?? (deleted ? pixels?.beforeUrl ?? null : null);
  const beforeUrl = pixels?.beforeUrl ?? null;
  const pair = expanded && tile.op === "modify" && beforeUrl !== null && !deleted;
  const note = slotNote(tile, pixels);
  // The record's size belongs to its own url; a BEFORE shown in a deleted slot has no known size
  const decoded = pixels !== undefined && pixels.width > 0 && afterUrl === pixels.url ? { width: pixels.width, height: pixels.height } : null;
  const size = pixels !== undefined && pixels.width > 0 ? `${String(pixels.width)}×${String(pixels.height)}` : "";
  const handleDrag = useCallback(() => { onDrag(tile.path); }, [onDrag, tile.path]);
  const pointer = useDragOutPointer({ onDragStart: handleDrag, enabled: !deleted });

  return (
    <figure
      className="stream-strip__tile"
      role="listitem"
      data-op={tile.op}
      data-fresh={fresh || undefined}
      data-pair={pair || undefined}
      data-outside-selection={tile.outsideSelection || undefined}
      aria-current={selected || undefined}
      style={{ "--stream-chain": String(chain) } as React.CSSProperties}
      title={`${tile.path}${size ? ` · ${size}` : ""}${tile.body.status === "image" ? ` · ${formatBytes(tile.body.bytes)}` : ""}`}
      onDoubleClick={() => { onOpen(tile); }}
      onContextMenu={(event) => {
        event.preventDefault();
        event.stopPropagation();
        onContextMenu(event, tile.path);
      }}
      {...pointer}
    >
      {pair ? (
        <span className="stream-strip__pair">
          <Slot url={beforeUrl} alt={`Previous ${tile.path}`} note="" label="BEFORE" decoded={null} />
          <Slot url={afterUrl} alt={tile.path} note={note} label="AFTER" decoded={decoded} />
        </span>
      ) : (
        <Slot url={afterUrl} alt={deleted ? `Deleted ${tile.path}` : tile.path} note={note} label={null} decoded={decoded} />
      )}
      <span className={`badge badge--${badge.variant} stream-strip__badge`} title={badge.label}>{badge.letter}</span>
      {tile.edits > 1 && <span className="stream-strip__edits" title={`${String(tile.edits)} edits merged`}>{`×${String(tile.edits)}`}</span>}
      <figcaption className="stream-strip__name">{baseName(tile.path)}</figcaption>
    </figure>
  );
}
