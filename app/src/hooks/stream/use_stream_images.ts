// Path: app/src/hooks/stream/use_stream_images.ts
// Description: Per-panel image tiles keyed by strip and path: gated readImageFile reads, decoded Blob tiles, bounded retention and revocation

import { useEffect, useRef, useState } from "react";
import { sendReadImageFile } from "../../lib/agent/messages.js";
import { IMAGE_FETCH_CONCURRENCY } from "../../lib/stream/stream_bounds.js";
import { beforeKeys, collectTileTargets, retainedKeys, type TileTarget } from "../../lib/stream/stream_tile_targets.js";
import type { StreamSnapshot } from "../../lib/stream/stream_types.js";
import { useAgent } from "../use_agent.js";
import { base64ToBlob } from "../use_image_blob_url.js";

/** "dropped" is a tile released to stay inside MAX_IMAGE_TILES; it keeps its slot, never refetches */
export type StreamTileStatus = "idle" | "loading" | "ready" | "dropped" | "error";

export interface StreamImageTile {
  status: StreamTileStatus;
  /** This slot's own Blob URL, alive exactly as long as the tile is retained */
  url: string | null;
  width: number;
  height: number;
  /** The pixels this path showed before its newest edit: the BEFORE half of an expanded pair */
  beforeUrl: string | null;
}

/** The panel's tile set: `byKey` is keyed by `tileKey(repoId, cardId, path)` (stream_tile_targets.ts) */
export interface StreamImageTiles {
  readonly repoId: string;
  readonly byKey: ReadonlyMap<string, StreamImageTile>;
}

interface TileRecord {
  status: StreamTileStatus;
  url: string | null;
  /** The Blob a replaced-in-place tile showed before its refetch; revoked with the record */
  previousUrl: string | null;
  width: number;
  height: number;
  /** The tile's updatedAtMs the pixels were read for; a newer stamp means a refetch */
  stamp: number;
}

function emptyTiles(repoId: string): StreamImageTiles {
  return { repoId, byKey: new Map<string, StreamImageTile>() };
}

function revoke(url: string | null): void {
  if (url !== null) URL.revokeObjectURL(url);
}

function release(record: TileRecord): void {
  revoke(record.url);
  revoke(record.previousUrl);
  record.url = null;
  record.previousUrl = null;
}

function sameTile(a: StreamImageTile, b: StreamImageTile): boolean {
  return (
    a.status === b.status &&
    a.url === b.url &&
    a.width === b.width &&
    a.height === b.height &&
    a.beforeUrl === b.beforeUrl
  );
}

/**
 * One tile owner per panel, keyed by strip id and path. Pixels are read only for retained,
 * previewable tiles while the stream is visible and the document showing, at most
 * IMAGE_FETCH_CONCURRENCY in flight; every tile outside `retainedKeys` keeps its slot and loses
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
    const publish = (targets: readonly TileTarget[], records: Map<string, TileRecord>): void => {
      const next = new Map<string, StreamImageTile>();
      const befores = beforeKeys(targets);
      let changed = false;
      for (const [index, target] of targets.entries()) {
        const record = records.get(target.key);
        const beforeKey = befores[index] ?? null;
        const before = beforeKey === null ? undefined : records.get(beforeKey);
        const candidate: StreamImageTile = {
          status: record?.status ?? "idle",
          url: record?.url ?? null,
          width: record?.width ?? 0,
          height: record?.height ?? 0,
          beforeUrl: record?.previousUrl ?? (before?.status === "ready" ? before.url : null),
        };
        const carried = tilesRef.current.byKey.get(target.key);
        if (carried !== undefined && sameTile(carried, candidate)) {
          next.set(target.key, carried);
        } else {
          next.set(target.key, candidate);
          changed = true;
        }
      }
      const current = tilesRef.current;
      if (!changed && current.repoId === repoId && next.size === current.byKey.size) return;
      const published: StreamImageTiles = { repoId, byKey: next };
      tilesRef.current = published;
      setTiles(published);
    };

    /** Late results are dropped and their Blob revoked: the token is the repo epoch plus the tile's stamp */
    const settle = (key: string, epoch: number, stamp: number, url: string | null, width: number, height: number): void => {
      const record = recordsRef.current.get(key);
      if (epoch !== epochRef.current || record === undefined || record.status !== "loading" || record.stamp !== stamp) {
        revoke(url);
        return;
      }
      record.url = url;
      record.width = width;
      record.height = height;
      record.status = url === null ? "error" : "ready";
      pumpRef.current();
    };

    const decode = (target: TileTarget, epoch: number, dataBase64: string, mimeType: string): void => {
      let url: string;
      try {
        url = URL.createObjectURL(base64ToBlob(dataBase64, mimeType));
      } catch {
        settle(target.key, epoch, target.stamp, null, 0, 0);
        return;
      }
      const probe = new Image();
      probe.onload = () => { settle(target.key, epoch, target.stamp, url, probe.naturalWidth, probe.naturalHeight); };
      probe.onerror = () => { URL.revokeObjectURL(url); settle(target.key, epoch, target.stamp, null, 0, 0); };
      probe.src = url;
    };

    const start = (target: TileTarget): void => {
      if (client === null) return;
      const epoch = epochRef.current;
      void sendReadImageFile(client, repoId, target.path)
        .then((result) => { decode(target, epoch, result.dataBase64, result.mimeType); })
        .catch(() => { settle(target.key, epoch, target.stamp, null, 0, 0); });
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
      const retained = retainedKeys(targets);
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
      publish(targets, records);
    };

    pumpRef.current = sync;
    sync();
  }, [canFetch, cards, client, repoId]);

  return tiles;
}
