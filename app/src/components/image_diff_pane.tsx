// Path: app/src/components/image_diff_pane.tsx
// Description: One side of an image diff: Git-labelled header, checkerboard image slot, size footer

import type React from "react";
import { useEffect, useState } from "react";
import type { ImageDiffSide } from "../shared/protocol.js";
import { useImageBlobUrl } from "../hooks/use_image_blob_url.js";
import { formatBytes } from "../lib/format_bytes.js";

export type ImageDiffSlot = "before" | "after";

/** A one-sided change shown alone: the header names the state and the snapshot the bytes came from */
export type ImageDiffSoloState = "new" | "deleted";

interface ImageDiffPaneProps {
  path: string;
  slot: ImageDiffSlot;
  side: ImageDiffSide | null;
  solo?: ImageDiffSoloState;
}

interface Dimensions {
  width: number;
  height: number;
}

function gitTerm(source: ImageDiffSide["source"]): string {
  switch (source) {
    case "head":
      return "HEAD";
    case "index":
      return "INDEX";
    case "worktree":
      return "WORKTREE";
    case "ours":
      return "STAGE 2";
    case "theirs":
      return "STAGE 3";
  }
}

/** Plain word plus the Git term the bytes came from; a missing side names what happened instead. */
export function paneHeader(
  slot: ImageDiffSlot,
  side: ImageDiffSide | null,
  solo?: ImageDiffSoloState
): string {
  if (side === null) return slot === "before" ? "NEW" : "DELETED";
  if (solo !== undefined) return `${solo.toUpperCase()} · ${gitTerm(side.source)}`;
  switch (side.source) {
    case "head":
      return "PREVIOUS · HEAD";
    case "index":
      return slot === "before" ? "PREVIOUS · INDEX" : "CURRENT · INDEX";
    case "worktree":
      return "CURRENT · WORKTREE";
    case "ours":
      return "OURS · STAGE 2";
    case "theirs":
      return "THEIRS · STAGE 3";
  }
}

function emptyCopy(slot: ImageDiffSlot): string {
  return slot === "before" ? "NO PREVIOUS VERSION" : "DELETED";
}

export function ImageDiffPane({ path, slot, side, solo }: ImageDiffPaneProps): React.JSX.Element {
  const [dimensions, setDimensions] = useState<Dimensions | null>(null);
  const payload = side !== null && !side.truncated ? side.dataBase64 : undefined;
  const source = useImageBlobUrl(payload, side?.mimeType);

  useEffect(() => {
    setDimensions(null);
  }, [payload]);

  const body = ((): React.JSX.Element => {
    if (side === null) {
      return <p className="image-diff-pane__slot">{emptyCopy(slot)}</p>;
    }
    if (side.truncated) {
      return (
        <p className="image-diff-pane__slot">
          {`TOO LARGE TO PREVIEW · OVER ${formatBytes(side.bytes)}`}
        </p>
      );
    }
    if (source.status === "error") {
      return <p className="image-diff-pane__slot">{source.message}</p>;
    }
    if (source.status !== "ready") {
      return <p className="empty-state empty-state--waiting">Preparing image</p>;
    }
    return (
      <img
        className="image-diff-pane__image"
        src={source.url}
        alt={`${paneHeader(slot, side, solo)} ${path}`}
        draggable={false}
        onLoad={(event) => {
          setDimensions({
            width: event.currentTarget.naturalWidth,
            height: event.currentTarget.naturalHeight,
          });
        }}
      />
    );
  })();

  const footer =
    side === null
      ? null
      : dimensions === null
        ? formatBytes(side.bytes)
        : `${dimensions.width}×${dimensions.height} · ${formatBytes(side.bytes)}`;

  return (
    <section className="image-diff-pane" data-slot={slot} data-empty={side === null}>
      <header className="image-diff-pane__header">{paneHeader(slot, side, solo)}</header>
      <div className="image-diff-pane__body">{body}</div>
      {footer !== null && <footer className="image-diff-pane__footer">{footer}</footer>}
    </section>
  );
}
