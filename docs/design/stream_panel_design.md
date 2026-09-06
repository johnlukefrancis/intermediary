# Stream Panel Design
Updated on: 2026-09-06
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006, ADR-007, ADR-008, ADR-009, ADR-010

---

The durable execution owner for this work is `docs/implementation/stream_panel_implementation.md`
(PR ladder, verification checklist, live receipt). This document is the behavior contract.

## Problem

Intermediary's left activity panel is the Auto Files table: a ranked list of recently changed file
*names*. When a coding agent works in a repo, the panel says that something happened and never says
what. The content is already in the product — the diff viewer, the image viewer, and the shared text
workspace shipped with Source Control — but reaching it costs a click per file, and by then the next
edit has landed. Watching an agent work should read like watching a build log: the edits print
themselves, in order, as they happen, and the panel stays bounded while they do.

## Goals

- A new left-panel mode, **Stream**, default for every profile: a continuously scrolling live feed
  that prints the actual content of each edit as it lands — diff hunks for modified files, the body of
  a new file, the removed lines of a deletion, image tiles for pictures.
- Each card names its own baseline, so what a card shows is never ambiguous: the diff versus the Git
  index the first time a path is seen in this agent run, versus the previous card after that.
- Latency of roughly 150–300 ms from save to card body on screen, for host-rooted and WSL-rooted repos
  alike, through the agent that already owns each root.
- Bounded in every dimension — settle queue, read concurrency, bytes per file, diff bytes, burst
  budget, ring size, lines per card, image tiles, DOM nodes — so a 500-file checkout collapses into
  one card instead of a stall.
- Satisfying to watch: cards enter, lines print, images drop, deletions strike, and the cadence slows
  or tightens with pressure, all composited and all governed.
- Reuses the shipped diff-line grammar, image viewer, drag-out handoff, and file context menu; the
  chosen mode persists across restarts.
- No new command, socket, port, CSP, or Tauri surface (ADR-010).

## Non-goals

- An agent-side ring or replay buffer. The agent publishes each delta once; scrollback is the UI's ring.
- Persisting the stream. The ring is view state and dies with the app; only `uiState.filesMode` persists.
- Virtualization. The ring is bounded at twenty cards precisely so a windowing layer is never needed.
- A new command, socket, port, CSP entry, or Tauri capability; the UI never runs Git.
- A fourth `FileSortMode`. Stream is a *mode* of the panel, not a fourth way to sort a table.
- Skeleton or placeholder cards. A card is created only from a delta that arrived, so nothing that
  appears can later vanish or end as a bare filename.

## MVP

The agent gains one additive event, `fileDelta`, published by a bounded delta pipeline beside the repo
watcher: each notify event settles per path, is read off the notify path on a blocking worker, and is
diffed against the previous sighting (or the Git index the first time) with the baseline named on the
wire. The UI gains a per-repo store outside React that owns a twenty-card ring, a cadence conductor, a
pending queue, expansion, follow-scroll, and bounded image tiles — the panel renders the store and
computes no diffs. The left panel's rocker gains a STREAM cell ahead of Auto / Latest / Active, over a
new `FilesMode` vocabulary; `uiState.filesMode` persists globally with the Rust mirror and validation
the `activeRail` precedent established, defaulting to `stream`.

## Naming

- **Stream** is the panel mode (`FilesMode = "stream" | FileSortMode`). `FileSortMode` keeps its three
  table values and its meaning: how the Auto Files table is ordered. Stream is never a sort value.
- **Card** is one admitted entry: a file card, a burst card, or a history row. **Ring** is the bounded
  ordered set of cards. **Burst** is the single card that absorbs a flood. **Notice** is a console-prompt
  row outside the ring (reconnects, withheld counts, backend state). **History row** is a compact seed
  row derived from the existing recent list — path and time, never content.
- **Baseline** vocabulary is user-visible and fixed: `SINCE LAST` (diff versus this run's previous card
  for that path), `VS INDEX` (first sighting; diff versus the Git index), `NEW` (no baseline; every line
  added), `GONE` (content unavailable, the path is simply reported removed).
- The wire says `previousSighting` / `index` / `none`; the chip is the human form of the same fact.

## Card grammar

**A card shows what changed since the agent's previous sighting of that path, or since the index the
first time (or nothing at all for a new file), and the card says which.**

One chassis, `.stream-card[data-kind][data-content][data-static]`, with a 3 px accent spine whose colour
carries the change class: added → success, modified → info, deleted → error, binary → warning, burst →
accent. Colour is never the only channel — the badge letter, the `+`/`−` glyphs, and the strike rule
carry the same fact.

- **Head** (24 px grid): `[A]`/`[M]`/`[D]`/`[R]`/`[T]` badge, file icon, filename with a head-truncated
  directory line, tabular `+N −M` (plus `[TRUNC]` when the patch was cut), an absolute clock stamped
  once at admit, the baseline chip, and `×N` when edits merged into this card.
- **Text bodies** render the shared `.diff-line[data-kind]` grammar at stream density: `meta` lines
  dropped, at most one hunk header, the first `LINE_CAP` lines, footer `+N MORE · OPEN DIFF`. A deletion
  prints up to five removed lines with the strike rule and rests at 0.72 opacity; a `gone` payload is a
  48 px `REMOVED` ghost.
- **Image strips** are one card per run of consecutive image edits: a grid of checkerboard tiles in
  arrival order, each with its change badge, name, and `×N` counter, headed `N IMAGES · <dir>` and
  footed with the clock span and total bytes. **The grid law (JL, 2026-09-06):** tiles take space in
  proportion to their count — a lone tile spans the row and is large; two halve it; three third it;
  about four seat per row at JL's panel width (`STRIP_TILE_PX` is the column minimum), then the row
  wraps and every row, the last included, keeps the same column width. A tile's slot is 16:10 of its
  column (screenshots and game frames; capped at `STRIP_SLOT_MAX_PX`) whether its pixels are loaded,
  released, deleted, or never fetched, so a strip's height is a function of its tile count and the
  panel width only. The thumb letterboxes inside its slot (`object-fit: contain`, never a crop) and
  never grows past twice its decoded size, so a screenshot fills the slot while a small icon is at
  most doubled and centred on the checkerboard. Expanding a strip pairs each modified tile with its
  retained BEFORE across two columns. Pixels arrive only through `readImageFile` as bytes into a Blob
  URL, under the size and mime gate; an image path whose metadata the agent could not read still
  lands as a tile (zero bytes, `PREVIEW FAILED` or `NO PREVIEW`), never as a file card.
- **Opaque bodies** are one 48 px line: `BINARY · 4.2 MB` or `TOO LARGE FOR STREAM`.
- **Burst cards** are fixed 96 px: `×N`, `N CHANGES IN 1.4 s`, the top three directories, an `A · M · D`
  strip, and a `RESOLVED` count. **History rows** are 24 px; **notices** are 32 px console prompts.

## Bounds

Every number is a named constant with a one-line reason at its owner; no stream module carries a bare
numeric literal outside the two bounds files.

**Agent** (`crates/im_agent/src/repos/delta/*`): `SETTLE_WINDOW` 120 ms · `MAX_LATENCY` 500 ms ·
`QUEUE_CAP` 256 · `DRAIN_BATCH` 16 · `MAX_RESETTLES` 3 · `MAX_DELTA_FILE_BYTES` 512 KiB ·
`CACHE_BYTES_PER_REPO` 16 MiB · `DIFF_DEADLINE` 150 ms · `CONTEXT_RADIUS` 3 · `PATCH_MAX_BYTES` 64 KiB ·
`DELTA_READ_CONCURRENCY` 2 (global) · `BURST_BUDGET` 32 per `BURST_WINDOW` 2 s · `INDEX_BLOB_TIMEOUT` 5 s ·
`INDEX_BLOB_LIMIT` = `MAX_DELTA_FILE_BYTES`. Ceilings: 16 MiB of baseline cache per repo, at most two
reads in flight per agent process, bus worst case ≈ 9 MiB.

**UI** (`app/src/lib/stream/stream_bounds.ts`): `RING_SIZE` 20 · `NOTICE_MAX` 3 · `HISTORY_ROWS` 12 ·
`LINE_CAP` 12 · `LINE_CAP_HANDSET` 6 · `EXPAND_CAP` 80 · `MAX_EXPANDED` 2 · `FLUSH_MS` 48 ·
`CADENCE_BASE_MS` 260 · `CADENCE_MIN_MS` 70 · `LAG_BUDGET_MS` 1500 · `IDLE_WAKE_MS` 1000 ·
`MERGE_WINDOW_MS` 1500 · `BURST_THRESHOLD` 22 · `BURST_WINDOW_MS` 1000 · `BURST_CLOSE_MS` 750 ·
`STORE_MAX` 4 · `IMAGE_CARD_MAX_BYTES` 4 MiB · `IMAGE_FETCH_CONCURRENCY` 2 · `IMAGE_STRIP_MAX` 12 ·
`MAX_IMAGE_TILES` 24 · `IMAGE_TILE_BYTES_BUDGET` 24 MiB · `STRIP_TILE_PX` 200 ·
`STRIP_TILE_HANDSET_PX` 96 · `STRIP_MIN_COLUMNS` 3 · `STRIP_SLOT_MAX_PX` 480 · `FOLLOW_EPSILON_PX` 24 · `STATIC_AFTER_MS` 1000 ·
`DIGEST_THROTTLE_MS` 5000 · `STREAM_MIN_AGENT_VERSION` = the version the release flow assigns.
Worst-case DOM: twenty cards ≈ 880 nodes, ≈ 1,520 with two expanded. `LINE_CAP` and the cadence numbers
are taste defaults JL tunes at the first witness session.

- Added at the review closure: `READ_DEADLINE` 2 s (bounds a settled read or diff join), `RENAME_PAIR_WINDOW` 80 ms (strictly inside the settle window), `NOTICE_TTL_MS` 45 s and `NOTICE_MERGE_MS` 2 s (notice rows age out and same-key notices merge), `SETTLING_TTL_MS` 1.5 s / `SETTLING_MAX` 8 (the settling line), `BURST_TOP_DIRS` 3, `PRESSURE_BUSY_AT` 4 / `PRESSURE_FLOOD_AT` 12 (cadence bands), `DBLCLICK_GRACE_MS` 220 (a double-click never leaves a card expanded).

## Motion governor amendment

The motion governor documented in `docs/design/intermediary_ui_overhaul_design.md` pauses every
animation while the window is hidden **or unfocused**, universally, via
`.app[data-motion="paused"] * { animation-play-state: paused }`. JL's decision for the Stream is that a
second monitor showing an agent at work must keep printing. This design amends the universal rule with
one scoped carve-out and one scoped tightening, both confined to the stream scroller and both loaded
after `motion.css`:

```css
/* Visible but unfocused: the stream keeps running; everything outside the scroller still pauses. */
.app[data-motion="paused"] .stream-scroller *,
.app[data-motion="paused"] .stream-scroller *::before,
.app[data-motion="paused"] .stream-scroller *::after { animation-play-state: running !important; }
/* Hidden or minimized: nothing animates and cards render in their resting state. */
.app[data-visibility="hidden"] .stream-scroller *,
.app[data-visibility="hidden"] .stream-scroller *::before,
.app[data-visibility="hidden"] .stream-scroller *::after { animation: none !important; }
```

`data-visibility="hidden"` is written on `.app` from `document.hidden` (the motion governor also reports
it), so hidden and unfocused stop being the same state for this surface only. Every keyframe's base state
is its resting state, so "no animation" is always the final look; cards older than `STATIC_AFTER_MS` carry
`data-static` and never replay on scroll-back or remount; `prefers-reduced-motion` zeroes the stagger
steps on top of the global collapse. The LIVE dot sits outside the scroller and stays governed.

## Behavior table

| Situation / input | Expected visible behavior |
| --- | --- |
| Fresh install or a config without `uiState.filesMode` | Panel opens in STREAM; up to `HISTORY_ROWS` compact rows from the recent list; the mode survives a save round-trip (Rust `files_mode`). |
| Agent saves one tracked code file (first sighting this agent run) | ~150–300 ms after the save a `[M] path` card enters; the hunks print line by line with the `VS INDEX` chip and `+N −M`; the spine sweeps. |
| Second save of the same file 5 s later | A new card prints `SINCE LAST` with only the lines that changed since the previous card. |
| Re-save inside `MERGE_WINDOW_MS` of the newest card of that file | The newest card extends: new lines print beneath, `×2` (×3…), older lines fall out of the cap first. |
| Rapid re-saves (every ~200 ms for 2 s) | Agent folds per `SETTLE_WINDOW`, emits at most one delta per `MAX_LATENCY`, each vs the previous emission; the UI extends one card with `×N`. |
| Editor truncate-then-write; ReadDirectoryChangesW early report | The settled read re-stats and re-arms (≤ 3 × 120 ms) on size/mtime movement or empty-on-modify vs a non-empty baseline; one honest card, never `−all` then `+all`. |
| New untracked text file | `text-added` card unfolds (`--ease-spring`), every line `add`, chip `NEW`, footer `NEW FILE · 214 LINES`. |
| Delete of a tracked file | `[D]` card prints the last content as all-removed (`SINCE LAST`, or `VS INDEX` from the index blob when never sighted); lines strike after printing; card rests at 0.72 opacity. |
| Delete of an untracked, never-sighted file | 48 px `[D] path · REMOVED` ghost (`GONE`). |
| Rename (notify pairs from → to) | One `[R] old → new` card; unchanged content shows `MOVED` (zero stats), a rename-with-edit shows the hunks; Auto Files still receives unlink + add. |
| Image added / several added together, however far apart in time | Whenever the next thing is an image and the tail card is a strip with room, the image joins it: one IMAGES strip card grows left to right. A lone tile spans the panel's width and is large; the second halves the row; the third thirds it; about four seat per row at the standard width, each further tile shrinking the columns evenly until one would drop under `STRIP_TILE_PX` (`STRIP_TILE_HANDSET_PX` on the handset), then the row wraps, every row keeping the same column width (never fewer than `STRIP_MIN_COLUMNS` per row; `IMAGE_STRIP_MAX` is three rows of four). Each tile is a 16:10 checkerboard slot (at most `STRIP_SLOT_MAX_PX` tall) with the file name under it; the thumb letterboxes inside it and never exceeds twice its decoded size. Tiles drop in one `--stream-chain-step` apart in reading order; the head counts them (`4 IMAGES · assets/icons`). |
| Image edited again while its strip is the newest | The tile for that path is replaced in place with the AFTER pixels and an `×N` counter. The strip keeps its position, its tile order, and its exact height; no new card is created. |
| More than `IMAGE_STRIP_MAX` images, or a text card printed after the strip | The open strip stops accepting new paths and the next image opens a new strip at the tail (a path already in the newest strip still replaces its tile in place wherever the strip sits). Time alone never closes a strip. A strip counts as one card against `RING_SIZE` however many tiles it holds. |
| Image modified with its previous tile still retained | The tile shows the AFTER pixels with an `M` badge; clicking the strip expands it and every modified tile whose BEFORE tile is still retained becomes a two-column BEFORE/AFTER pair. Clicking again collapses it. |
| Image deleted | The tile greys, a rule strikes it, and it keeps its slot; with no pixels retained the slot reads `DELETED`. The strip's height does not change. |
| Image tile released by `MAX_IMAGE_TILES` retention | The Blob URL is revoked and the slot reads `RELEASED` at exactly the same size. A released tile never changes a strip's height and never moves the reading position. |
| Image > 4 MiB, or heic/heif/tiff/tif | The slot reads `NO PREVIEW` over the size or the extension and no bytes cross the wire (gated by `fileDelta.image.bytes`/`mimeType`); the slot is the same size as every other. |
| Tile double-clicked / right-clicked / dragged | The image viewer, or the image diff for a modified tracked image / the shared file actions for that tile's path / drag-out of that tile's path. The strip itself opens nothing. |
| Strip focused with the keyboard | Left and Right move the selected tile, Enter opens it, Space toggles the BEFORE/AFTER expansion; Up, Down, Home and End still move between cards. |
| A checkout floods the repo with images | Unchanged: the burst detector opens first, the burst card absorbs the image paths, and no strip is created. A strip still waiting in the pending FIFO is folded into the burst, its tiles counted as resolved paths. |
| Binary or > 512 KiB text-classified file | `BINARY · 4.2 MB` / `TOO LARGE FOR STREAM` 48 px card; double-click still opens the workspace (which applies its own limits). |
| A branch checkout touching 500 files | Within a second the distinct-path rate crosses `BURST_THRESHOLD`: one burst card slams in (`×500 · 1.4 s`, top-3 dirs, `A 12 · M 480 · D 8`); the agent reads ≤ `BURST_BUDGET` paths whose deltas bump `RESOLVED`; withheld paths produce no events; `> 468 EDITS WITHHELD · BURST` prints from the `withheld` counter; the ring keeps its other 19 cards; ≤ 2 reads in flight in the agent; SOURCE still refreshes via its own coalescer. |
| Steady 15 files/s for 3 s (no distinct-path burst) | Cards print at the cadence floor; the pending backlog reaching `BURST_THRESHOLD` collapses the rest into a burst card; the withheld notice reports the agent's budget. |
| Delta lost on bus lag (`seq` gap) | Notice `> N EDITS NOT SHOWN`; nothing else waits on it (cards are created only from deltas that arrived). |
| Filter changed | Non-matching cards hide (`data-filtered`); ring untouched; switching back reveals them in place. |
| Path outside the active ZIP selection | Card renders muted with `OUTSIDE SELECTION` (never hidden). |
| Mode switched away and back | The table shows; the store keeps admitting; STREAM returns with the ring current, older cards static, `> N CHANGES WHILE AWAY`. |
| Card clicked / double-clicked / dragged / right-clicked | Expands in place (≤ `EXPAND_CAP` lines, follow frozen) / opens the diff or file in the shared workspace (panel slot replaced as today) / staged drag-out / the shared file actions menu. |
| Repo tab switch | The departed repo's store is retained (`STORE_MAX` 4, LRU); the arriving repo shows its ring, its history rows, or the waiting prompt. History rows come only from that repo's own `snapshot` event, routed by its `repoId` through the store's intake — never from the previous tab's recent list. |
| The ring is taller than the panel | Every entry keeps its natural height and the scroller overflows: the scroller is a track (`flex: 0 0 auto` on every child), never a column that compresses to fit; the scrollbar is the scroller's own; no card is ever reduced to its spine and a clipped line. |
| Handset | FILES section shows the stream at chassis width with `LINE_CAP_HANDSET`; other sections hide it (observe only). |
| Window visible but unfocused (second monitor) | Cards keep arriving and animating (governor carve-out); everything outside the scroller pauses as today. |
| Window hidden / minimized | Admission pauses and nothing animates; on show, backlog ≥ `BURST_THRESHOLD` collapses into one burst card, else cards cascade at cadence. |
| `prefers-reduced-motion` | Keyframes collapse (motion.css) and stagger steps are zeroed; the conductor admits the backlog immediately. |
| WSL backend offline (WSL repo) | LIVE reads HELD; `> WSL BACKEND OFFLINE — STREAM HELD`; host repos unaffected; on `online` the ring resumes. |
| Reconnect / rehydrate (`rehydrateEpoch`) | Ring kept; `> RECONNECTED — RESUMING`; `lastSeq` reset (a seq restart is a new stream, not a gap); snapshot events only seed history rows when the ring is empty. |
| Older agent (below `STREAM_MIN_AGENT_VERSION`) | `AGENT UPDATE REQUIRED · <version>+` empty state; Auto/Latest/Active still work. |
| User scrolls up | Follow unpins; new cards land below without moving the view; `▼ N NEW` pill; click or scroll to bottom re-pins. |
| Card focused with the keyboard | Follow freezes; Up/Down/Home/End move; Enter opens; Space expands; Escape releases and resumes follow. |

## Acceptance

Witnessed in the installed Windows app (the dev CSP is null, so Blob-URL images need the release build).
The runbook is `docs/commands/verify_stream.md` (finalized at closeout); the installer build steps are
`docs/commands/build_installer_from_wsl.md` and the closeout gates are
`docs/commands/workflow/closeout_checks.md`.

1. With one host-rooted and one WSL-rooted repo tab open, the rocker shows STREAM active, with history
   rows or the waiting prompt.
2. Editing a tracked file prints a card within a blink with `VS INDEX`; the next save prints `SINCE LAST`
   with only the new lines; a quick re-save extends the newest card with `×2`. The DevTools console shows
   the `fileDelta` envelope with the matching `baseline`, `folded`, and `withheld: 0`.
3. Create, delete a tracked file, delete an untracked file, and rename: unfold, all-removed strike,
   `REMOVED` ghost, and a single `[R]` card.
4. Six PNGs dropped in, one overwritten, one deleted, plus a `.heic` and a 6 MB PNG: cascade, two-up,
   ghost, and `NO PREVIEW` with no image read on the socket.
5. A branch checkout touching ~500 files: one burst card, `RESOLVED` ≤ 32, the withheld notice, at most
   twenty `.stream-card` nodes in the DOM, at most two concurrent delta reads in the agent log, and SOURCE
   still refreshing.
6. Switching to Latest and back, opening and closing a card's workspace, and switching repo tabs and back
   all leave the ring intact and static, with the while-away notice.
7. Unfocusing the window keeps cards arriving and animating; minimize and restore lands them instantly,
   then bursts or cascades; OS reduced motion lands them instantly with zero stagger.
8. Thirty seconds of real agent edits in the Performance panel: no Layout/Recalc attributable to
   `.stream-*`, every `stream-*` animation composited, ≥ 55 fps through a flood, heap flat after 500
   admits, and Blob count ≤ `MAX_IMAGE_TILES` plus in-flight.
9. Stopping the WSL backend shows HELD and the notice; restarting resumes. An agent below
   `STREAM_MIN_AGENT_VERSION` shows `AGENT UPDATE REQUIRED`.
10. Removing `uiState.filesMode` from the config and restarting lands on STREAM; choosing Auto and
    restarting lands on Auto.

## Accepted boundaries

- **The delta pipeline is always on.** Every text change in every watched repo costs one bounded read and
  diff (two concurrent, 512 KiB, 32 per 2 s, 16 MiB cache) even when no Stream is on screen; only a fully
  idle daemon with no event-bus receivers is skipped. Stream is the default mode and the store never
  discards the result. A per-repo subscription lease is the documented escalation if the cost is ever
  observed.
- **Baseline drift is named, not hidden.** After a budget-withheld or oversized sighting the path is
  evicted from the cache, and its next edit prints `VS INDEX` (tracked) or `NEW` (untracked). The chip
  always tells the truth about what the card compared.
- **`tracked` on the wire is best-effort.** The tracked-path set reloads up to about a second behind the
  index, so the flag decorates; the delete path never depends on it.
- **Bus pressure is bounded and drops are visible.** `fileDelta` adds at most 32 events per 2 s per repo
  (≤ 64 KiB each) on top of today's `fileChanged`; withheld paths add nothing, and anything the broadcast
  bus drops shows up as a `seq` gap notice rather than a silent hole.
- **A slow writer may print twice.** A process that pauses longer than about 360 ms mid-file defeats the
  settle window and prints the partial, then the rest. Honest and bounded; the alternative is holding
  edits behind an unbounded wait.
- **The stream is not visible while a file is open.** Opening a card replaces the panel slot, as every
  workspace open does today. The store keeps admitting and the ring is current on close, with the
  while-away notice. Accepted for v1.
- **The governor carve-out amends a rule documented as universal.** It is scoped to the stream scroller,
  written into this design, and mirrored into the design-system document when the motion rung lands.
- **Two CSS properties are unverified on WebView2** — `content-visibility: auto` on non-tail cards and a
  sticky follow pill inside the panel's clipped box. Both have stated fallbacks: drop the property,
  absolute-position the pill.
- **`similar` is the first new agent crate.** Pure Rust, so the statically linked Windows CRT profile is
  unaffected; it needs one network fetch on a fresh machine.
- **Adjudicated at the 2026-09-06 adversarial review** (four lenses, no P0; every P1 fixed):
  - A repo's drained batch resolves sequentially; the process-wide two-permit read semaphore bounds
    concurrency across repos, not within one. The UI admits one card per cadence tick anyway, so a
    ten-file save trickles at the cadence, not at the read rate.
  - A rename followed by a delete of the destination inside one settle window collapses to a plain remove
    and prints a `GONE` ghost rather than the last content the agent served under the old name.
  - The Windows two-event rename is paired by a slot with an 80 ms window (strictly inside the settle
    window), not by a notify tracker cookie or a same-parent check; an unpaired `From` stays the honest
    delete it always was, and the pairing is proven by unit test until the host-repo witness runs.
  - A delete whose baseline is cached but whose read budget is spent still emits a `gone` event with no
    patch: tiny, count-bounded by the queue cap per drain, and the one deliberately unbudgeted outcome.
  - The per-connection event queue behind the broadcast bus is unbounded (pre-existing); the producer-side
    budget is the mitigation, and a bounded queue with a drop counter is the recorded follow-up.
  - The DOM holds at most twenty-one cards: the ring plus one card mid-exit, spliced on the next admit.
  - An expanded card keeps its clamped line stagger during a flood (the card's declaration wins over the
    scroller's band); expansion is user-driven and rare.
  - Counters that would strand at the end of a burst travel on one additive `fileDeltaCounters` event;
    the same counters piggyback on the next `fileDelta` when one follows, and the UI merges both into
    one notice.
