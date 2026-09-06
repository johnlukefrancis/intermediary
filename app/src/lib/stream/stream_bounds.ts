// Path: app/src/lib/stream/stream_bounds.ts
// Description: Every numeric bound the Stream panel obeys; the only stream module with literals

/** Cards the ring holds (file + burst + history); notices are bounded separately */
export const RING_SIZE = 20;

/** Console-prompt notice rows kept above the ring */
export const NOTICE_MAX = 3;

/** Compact seed rows taken from the existing recent list when the ring has no live cards */
export const HISTORY_ROWS = 12;

/** Diff lines a collapsed card prints before the "+N MORE" footer */
export const LINE_CAP = 12;

/** Diff lines a collapsed card prints on the handset chassis */
export const LINE_CAP_HANDSET = 6;

/** Diff lines an expanded card prints, and the most a card ever retains in memory */
export const EXPAND_CAP = 80;

/** Cards that may be expanded at once; the oldest collapses when a third opens */
export const MAX_EXPANDED = 2;

/** Intake buffer flush period: one reducer pass per this many ms, never per event */
export const FLUSH_MS = 48;

/** Calm cadence between admissions, the slowest the conductor ever prints */
export const CADENCE_BASE_MS = 260;

/** Fastest cadence between admissions under backlog pressure */
export const CADENCE_MIN_MS = 70;

/** Target time to drain the pending backlog; divided by the backlog to pick the cadence */
export const LAG_BUDGET_MS = 1500;

/** Quiet period after which the next arrival is admitted immediately instead of waiting */
export const IDLE_WAKE_MS = 1000;

/** A re-edit of the same path extends the newest card of that path while it is younger than this */
export const MERGE_WINDOW_MS = 1500;

/** Distinct paths inside BURST_WINDOW_MS that open a burst card instead of per-file cards */
export const BURST_THRESHOLD = 22;

/** Window the burst detector measures the distinct-path rate over */
export const BURST_WINDOW_MS = 1000;

/** Quiet period that closes an open burst card */
export const BURST_CLOSE_MS = 750;

/** Per-repo stores retained (LRU, the visible repo pinned) */
export const STORE_MAX = 4;

/** Largest image a card fetches pixels for; above this the card shows a NO PREVIEW ghost (4 MiB) */
export const IMAGE_CARD_MAX_BYTES = 4 * 1024 * 1024;

/** Image reads in flight at once from the panel */
export const IMAGE_FETCH_CONCURRENCY = 2;

/** Tiles one strip accepts: three rows of four at the standard panel width; no time window, only a card printed after the strip closes it */
export const IMAGE_STRIP_MAX = 12;

/** Decoded thumbnails retained across the whole ring; older tiles keep their slot and lose their Blob */
export const MAX_IMAGE_TILES = 24;

/** Summed source bytes of retained tiles; bounds decoded bitmap memory when tiles sit near the 4 MiB gate (24 MiB) */
export const IMAGE_TILE_BYTES_BUDGET = 24 * 1024 * 1024;

/** Narrowest tile column on the standard deck: one tile spans the row, two halve it, three third it, about four seat per ~850 px row, then wrap */
export const STRIP_TILE_PX = 200;

/** Narrowest tile column on the handset chassis (three or four per row at its width) */
export const STRIP_TILE_HANDSET_PX = 96;

/** Fewest tiles a row ever seats: the column minimum is capped at a third of the row so a narrow panel never drops below three */
export const STRIP_MIN_COLUMNS = 3;

/** Tallest a tile's 16:10 checkerboard slot grows; a lone tile spanning a wide panel stops here instead of filling the ring */
export const STRIP_SLOT_MAX_PX = 480;

/** Scroll distance from the bottom still counted as pinned to the tail */
export const FOLLOW_EPSILON_PX = 24;

/** Age after which a card is marked static so scroll-back never replays its arrival */
export const STATIC_AFTER_MS = 1000;

/** Throttle for the screen-reader digest of newly admitted cards */
export const DIGEST_THROTTLE_MS = 5000;

/** A notice with the same key inside this window accumulates its count in place instead of stacking */
export const NOTICE_MERGE_MS = 2000;

/** Age after which a notice row is dropped; long enough to be read, short enough to stop being stale news */
export const NOTICE_TTL_MS = 45_000;

/** Text paths wait this long for their delta before the settling line forgets them (agent MAX_LATENCY + 1 s) */
export const SETTLING_TTL_MS = 1500;

/** Settling paths retained; the settling line never shows more */
export const SETTLING_MAX = 8;

/** Top-level directories the burst card's strip names */
export const BURST_TOP_DIRS = 3;

/** Pending backlog at which the pressure band leaves calm for busy */
export const PRESSURE_BUSY_AT = 4;

/** Pending backlog at which the pressure band becomes flood */
export const PRESSURE_FLOOD_AT = 12;

/** Grace a single click waits before expanding so a double-click can open the workspace instead */
export const DBLCLICK_GRACE_MS = 220;
