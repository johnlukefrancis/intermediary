// Path: app/src/hooks/images/use_image_tiles.ts
// Description: Revision-bound image reads, bounded thumbnail retention and actual unfinished-request concurrency

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { sendReadImageFile } from "../../lib/agent/messages.js";
import { IMAGE_TILE_MAX_BYTES, IMAGE_FETCH_CONCURRENCY } from "../../lib/images/image_tile_bounds.js";
import { readRefusedAsChanged, sameRevision } from "../../lib/images/image_tile_pixels.js";
import { retainedKeys, type TileTarget } from "../../lib/images/image_tile_targets.js";
import { useAgent } from "../use_agent.js";
import { emptyTiles, projectTiles, release, revoke, type ImageTiles, type ImageTileOutcome, type TileRecord } from "./image_tile_records.js";
import { decodeTile } from "./image_tile_thumbnail.js";

export type { ImageTile, ImageTiles, ImageTileStatus } from "./image_tile_records.js";

export function useImageTiles(targets: readonly TileTarget[], canFetch: boolean): ImageTiles {
  const { client, helloState } = useAgent();
  const recordsRef = useRef<Map<string, TileRecord>>(new Map<string, TileRecord>());
  const tilesRef = useRef<ImageTiles>(emptyTiles());
  const epochRef = useRef(0);
  const inFlightRef = useRef(0);
  const pumpRef = useRef<() => void>(() => undefined);
  const [tiles, setTiles] = useState<ImageTiles>(emptyTiles);
  const enabled = canFetch && client !== null && helloState.status === "ok";

  useEffect(() => {
    const records = recordsRef.current;
    return () => {
      epochRef.current += 1;
      pumpRef.current = () => undefined;
      for (const record of records.values()) release(record);
      records.clear();
      tilesRef.current = emptyTiles();
    };
  }, []);

  useLayoutEffect(() => {
    const settle = (record: TileRecord, epoch: number, outcome: ImageTileOutcome, url: string | null, width: number, height: number): void => {
      if (epoch !== epochRef.current || recordsRef.current.get(record.target.key) !== record || record.status !== "loading") {
        revoke(url);
        return;
      }
      record.url = url;
      record.width = width;
      record.height = height;
      record.status = outcome;
    };

    const start = (record: TileRecord): void => {
      if (client === null) return;
      const target = record.target;
      const epoch = epochRef.current;
      inFlightRef.current += 1;
      void sendReadImageFile(client, target.repoId, target.path, IMAGE_TILE_MAX_BYTES)
        .then(async (result) => {
          if (!sameRevision(target, result)) {
            settle(record, epoch, "superseded", null, 0, 0);
            return;
          }
          if (epoch !== epochRef.current || recordsRef.current.get(target.key) !== record || record.status !== "loading") return;
          const decoded = await decodeTile(result.dataBase64, result.mimeType);
          if (decoded.outcome === "error") settle(record, epoch, "error", null, 0, 0);
          else settle(record, epoch, decoded.outcome, decoded.outcome === "ready" ? decoded.url : null, decoded.width, decoded.height);
        })
        // TODO(ts-precision): Rejected IPC/decode promises are the dynamic error seam.
        .catch((error: unknown) => {
          settle(record, epoch, readRefusedAsChanged(error) ? "superseded" : "error", null, 0, 0);
        })
        .finally(() => {
          inFlightRef.current -= 1;
          pumpRef.current();
        });
    };

    const sync = (): void => {
      const records = recordsRef.current;
      const live = new Set<string>(targets.map((target) => target.key));
      const retained = retainedKeys(targets);
      for (const [key, record] of records) {
        if (!live.has(key)) {
          release(record);
          records.delete(key);
        } else if (!retained.has(key) && record.status !== "dropped") {
          release(record);
          record.status = "dropped";
        }
      }
      for (const target of targets) {
        let record = records.get(target.key);
        const sameSource = record?.target.sourceKey === target.sourceKey;
        if (record !== undefined && !sameSource) {
          release(record);
          records.delete(target.key);
          record = undefined;
        }
        if (!retained.has(target.key)) continue;
        if (record === undefined || record.target.stamp !== target.stamp || !sameRevision(record.target, target)) {
          const previousUrl = record?.status === "ready" ? record.url : null;
          if (record !== undefined) {
            revoke(record.previousUrl);
            if (record.status !== "ready") revoke(record.url);
          }
          record = { status: "idle", url: null, previousUrl, width: 0, height: 0, target };
          records.set(target.key, record);
        }
        if (record.status !== "idle" || !enabled || inFlightRef.current >= IMAGE_FETCH_CONCURRENCY) continue;
        record.status = "loading";
        start(record);
      }
      const published = projectTiles(targets, records, tilesRef.current);
      if (published === null) return;
      tilesRef.current = published;
      setTiles(published);
    };

    pumpRef.current = sync;
    sync();
  }, [enabled, targets, client]);

  return tiles;
}
