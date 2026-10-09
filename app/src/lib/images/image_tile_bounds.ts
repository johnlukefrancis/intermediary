// Path: app/src/lib/images/image_tile_bounds.ts
// Description: Shared image tile byte, decode, concurrency and thumbnail retention bounds

/** Agent read ceiling; source bytes are transient and retained tiles are thumbnails. */
export const IMAGE_TILE_MAX_BYTES = 25 * 1024 * 1024;

/** Unfinished image reads and decodes per tile owner, including removed targets. */
export const IMAGE_FETCH_CONCURRENCY = 2;

/** Newest fetchable thumbnails retained across all sources in one tile owner. */
export const MAX_IMAGE_TILES = 24;

/** Long edge of a retained thumbnail; smaller sources remain pixel-exact. */
export const IMAGE_THUMB_MAX_PX = 1024;

/** Source pixel ceiling checked before creating the decoded bitmap. */
export const MAX_TILE_PIXELS = 64_000_000;
