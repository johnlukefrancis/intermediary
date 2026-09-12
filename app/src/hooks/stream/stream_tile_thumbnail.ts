// Path: app/src/hooks/stream/stream_tile_thumbnail.ts
// Description: Decodes one tile's fetched bytes into the thumbnail Blob the strip retains: probe the source, gate it, downscale past STRIP_THUMB_MAX_PX, else keep it

import { exceedsTilePixels, thumbnailSize } from "../../lib/stream/stream_tile_pixels.js";
import { base64ToBlob } from "../use_image_blob_url.js";

/** How a decode ended: `ready` carries the Blob URL the slot shows; `tooLarge` still names the size the probe saw */
export type DecodedTile =
  | { outcome: "ready"; url: string; width: number; height: number }
  | { outcome: "tooLarge"; width: number; height: number }
  | { outcome: "error" };

/** Header-only: an Image reports its natural size on load without decoding pixels until painted */
function probe(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => { resolve(image); };
    image.onerror = () => { reject(new Error("Image did not decode")); };
    image.src = url;
  });
}

/**
 * Paints the source at thumbnail size and encodes it. The full bitmap exists only inside this
 * call and is closed before the encode. WebP keeps alpha; a WebView without a WebP encoder
 * falls back to PNG by the canvas contract.
 */
async function downscale(source: HTMLImageElement, width: number, height: number): Promise<Blob> {
  const bitmap = await createImageBitmap(source, { resizeWidth: width, resizeHeight: height, resizeQuality: "high" });
  try {
    const canvas = new OffscreenCanvas(width, height);
    const context = canvas.getContext("2d");
    if (context === null) throw new Error("No 2d context for the thumbnail");
    context.drawImage(bitmap, 0, 0);
    return await canvas.convertToBlob({ type: "image/webp", quality: 0.92 });
  } finally {
    bitmap.close();
  }
}

/**
 * The one route from fetched bytes to a retained tile. The source Blob URL lives only for the
 * probe and the downscale unless the source already fits STRIP_THUMB_MAX_PX, in which case it IS
 * the tile (pixel-exact icons, animated small GIFs). `width`/`height` are always the source's.
 */
export async function decodeTile(dataBase64: string, mimeType: string): Promise<DecodedTile> {
  let sourceUrl: string;
  try {
    sourceUrl = URL.createObjectURL(base64ToBlob(dataBase64, mimeType));
  } catch {
    return { outcome: "error" };
  }
  let sourceIsTile = false;
  try {
    const source = await probe(sourceUrl);
    const { naturalWidth: width, naturalHeight: height } = source;
    if (exceedsTilePixels(width, height)) return { outcome: "tooLarge", width, height };
    const fit = thumbnailSize(width, height);
    if (fit === null) {
      sourceIsTile = true;
      return { outcome: "ready", url: sourceUrl, width, height };
    }
    const thumbnail = await downscale(source, fit.width, fit.height);
    return { outcome: "ready", url: URL.createObjectURL(thumbnail), width, height };
  } catch {
    return { outcome: "error" };
  } finally {
    if (!sourceIsTile) URL.revokeObjectURL(sourceUrl);
  }
}
