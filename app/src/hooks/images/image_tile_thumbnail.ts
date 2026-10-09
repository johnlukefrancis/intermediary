// Path: app/src/hooks/images/image_tile_thumbnail.ts
// Description: Probes and gates fetched image bytes, retaining an original small image or a bounded thumbnail Blob

import { exceedsTilePixels, thumbnailSize } from "../../lib/images/image_tile_pixels.js";
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

/** The resized bitmap is closed after encoding; the canvas can fall back from WebP to PNG. */
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

/** Source URLs survive only for small images; returned dimensions always describe the source. */
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
