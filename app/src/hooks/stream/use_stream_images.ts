// Path: app/src/hooks/stream/use_stream_images.ts
// Description: Per-panel image tiles keyed by strip and path: revision-bound readImageFile reads, decoded Blob tiles, bounded retention and revocation

import { useEffect, useRef, useState } from "react";
import { sendReadImageFile } from "../../lib/agent/messages.js";
import { IMAGE_CARD_MAX_BYTES, IMAGE_FETCH_CONCURRENCY } from "../../lib/stream/stream_bounds.js";
import { exceedsTilePixels, readRefusedAsChanged, sameRevision } from "../../lib/stream/stream_tile_pixels.js";
import { collectTileTargets, retainedKeys, type TileTarget } from "../../lib/stream/stream_tile_targets.js";
import type { StreamSnapshot } from "../../lib/stream/stream_types.js";
import { useAgent } from "../use_agent.js";
import { base64ToBlob } from "../use_image_blob_url.js";
import {
  decodedPixelsOf,
  emptyTiles,
  projectTiles,
  release,
  revoke,
  type StreamImageTiles,
  type StreamTileOutcome,
  type TileRecord,
} from "./stream_tile_records.js";

export type { StreamImageTile, StreamImageTiles, StreamTileStatus } from "./stream_tile_records.js";

/**
 * One tile owner per panel, keyed by strip id and path. Pixels are read only for retained,
 * previewable tiles while the stream is visible and the document showing, at most
 * IMAGE_FETCH_CONCURRENCY in flight, under IMAGE_CARD_MAX_BYTES on the agent side, and are
 * accepted only for the exact revision (bytes + mtime) the tile announced — a mismatch, and the
 * agent's own refusal of a file rewritten under its read, both leave the slot reading IMAGE CHANGED;
 * a decoded bitmap past MAX_TILE_PIXELS is released at once. Every tile outside `retainedKeys` keeps its slot and loses
 * its Blob. A tile replaced in place keeps its old Blob as BEFORE while the AFTER is refetched.
 */
export function useStreamImages(repoId: string, snapshot: StreamSnapshot): StreamImageTiles {
  const { client, helloState } = useAgent();
  const recordsRef = useRef<Map<string, TileRecord>>(new Map<string, TileRecord>());
  const tilesRef = useRef<StreamImageTiles>(emptyTiles(repoId));
  const epochRef = useRef(0);
  const pumpRef = useRef<() => void>(() => undefined);
  const [tiles, setTiles] = useState<StreamImageTiles>(() => emptyTiles(repoId));

  const cards = snapshot.ring.cards;
  // The store already carries the host's hidden flag; no second governor subscription here
  const canFetch = client !== null && helloState.status === "ok" && snapshot.visible && !snapshot.documentHidden;

  // A repo switch or an unmount invalidates every read in flight and revokes every retained tile
  useEffect(() => {
    const records = recordsRef.current;
    return () => {
      epochRef.current += 1;
      for (const record of records.values()) release(record);
      records.clear();
      tilesRef.current = emptyTiles(repoId);
    };
  }, [repoId]);

  useEffect(() => {
    /** Late results are dropped and their Blob revoked: the token is the repo epoch plus the tile's stamp */
    const settle = (target: TileTarget, epoch: number, outcome: StreamTileOutcome, url: string | null, width: number, height: number): void => {
      const record = recordsRef.current.get(target.key);
      if (epoch !== epochRef.current || record === undefined || record.status !== "loading" || record.stamp !== target.stamp) {
        revoke(url);
        return;
      }
      record.url = url;
      record.width = width;
      record.height = height;
      record.status = outcome;
      pumpRef.current();
    };

    const decode = (target: TileTarget, epoch: number, dataBase64: string, mimeType: string): void => {
      let url: string;
      try {
        url = URL.createObjectURL(base64ToBlob(dataBase64, mimeType));
      } catch {
        settle(target, epoch, "error", null, 0, 0);
        return;
      }
      const probe = new Image();
      probe.onload = () => {
        const { naturalWidth: width, naturalHeight: height } = probe;
        // The gate runs on the probe's reported size: the bitmap is let go before any slot shows it
        if (exceedsTilePixels(width, height)) {
          URL.revokeObjectURL(url);
          settle(target, epoch, "tooLarge", null, 0, 0);
          return;
        }
        settle(target, epoch, "ready", url, width, height);
      };
      probe.onerror = () => { URL.revokeObjectURL(url); settle(target, epoch, "error", null, 0, 0); };
      probe.src = url;
    };

    const start = (target: TileTarget): void => {
      if (client === null) return;
      const epoch = epochRef.current;
      void sendReadImageFile(client, repoId, target.path, IMAGE_CARD_MAX_BYTES)
        .then((result) => {
          // Never newer pixels under an older card: any other revision than the tile's is refused
          if (!sameRevision(target, result)) {
            settle(target, epoch, "superseded", null, 0, 0);
            return;
          }
          decode(target, epoch, result.dataBase64, result.mimeType);
        })
        // A read the agent refused because the file moved under it is IMAGE CHANGED, not a failure
        .catch((error: unknown) => {
          settle(target, epoch, readRefusedAsChanged(error) ? "superseded" : "error", null, 0, 0);
        });
    };

    const sync = (): void => {
      const records = recordsRef.current;
      const targets = collectTileTargets(repoId, cards);
      const live = new Set<string>(targets.map((target) => target.key));
      for (const [key, record] of records) {
        if (live.has(key)) continue;
        release(record);
        records.delete(key);
      }
      const retained = retainedKeys(targets, decodedPixelsOf(records));
      let inFlight = 0;
      for (const [key, record] of records) {
        if (retained.has(key)) {
          if (record.status === "loading") inFlight += 1;
        } else if (record.status !== "dropped") {
          release(record);
          record.status = "dropped";
        }
      }
      for (const target of targets) {
        if (!retained.has(target.key) || !canFetch || inFlight >= IMAGE_FETCH_CONCURRENCY) continue;
        const record = records.get(target.key);
        if (record !== undefined && record.stamp === target.stamp) continue;
        // Replaced in place: the ready pixels become the BEFORE while the AFTER is read again
        const previousUrl = record?.status === "ready" ? record.url : null;
        if (record !== undefined) {
          revoke(record.previousUrl);
          if (record.status !== "ready") revoke(record.url);
        }
        inFlight += 1;
        records.set(target.key, { status: "loading", url: null, previousUrl, width: 0, height: 0, stamp: target.stamp });
        start(target);
      }
      const published = projectTiles(repoId, targets, records, tilesRef.current);
      if (published === null) return;
      tilesRef.current = published;
      setTiles(published);
    };

    pumpRef.current = sync;
    sync();
  }, [canFetch, cards, client, repoId]);

  return tiles;
}
