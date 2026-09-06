# Stream Panel Architecture
Updated on: 2026-09-06
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

---

The Stream is the default mode of the left activity panel: a bounded live feed of what agents edit. The agent
owns "what changed inside this file" and publishes it as one additive event; the UI owns only view state.
The behaviour contract is `docs/design/stream_panel_design.md`; the witness route is
`docs/commands/verify_stream.md`.

## Ownership

| Concern | Owner | Notes |
| --- | --- | --- |
| Per-path settle queue | `crates/im_agent/src/repos/delta/settle_queue.rs` | Trailing coalescer: `SETTLE_WINDOW` 120 ms, `MAX_LATENCY` 500 ms, `QUEUE_CAP` 256; add→remove and remove→add collapse; re-marks fold; dropped paths are recorded (bounded) so the worker can evict their baselines. Every entry point takes `now`. |
| Read budget | `crates/im_agent/src/repos/delta/delta_budget.rs` | `BurstBucket` charges one token per pending change that will emit (`BURST_BUDGET` 32 per `BURST_WINDOW` 2 s); no token → `Withhold` (no event, baseline evicted) or, for a delete, `GoneOnly` (a bare `gone` event, no read, no spawn). Re-settles are never re-charged. One `info` per window that denied anything, one `warn` per path per window. |
| Baseline cache | `crates/im_agent/src/repos/delta/baseline_cache.rs` | Byte-bounded LRU of the last text served per path: `CACHE_BYTES_PER_REPO` 16 MiB, entries ≤ `MAX_DELTA_FILE_BYTES` 512 KiB; `get/insert/remove/rename`; never persisted. |
| Settled read | `crates/im_agent/src/repos/delta/delta_read.rs` | Blocking `read_settled`: stat → size gate → bounded read (`take(MAX + 1)`) → re-stat; `Unsettled` when the stamp moved or a `Modify` read back empty, `Opaque(tooLarge\|binary\|unreadable)`, `Missing`, `Text`; `accept_moving` on the final attempt after `MAX_RESETTLES` 3. |
| Diff | `crates/im_agent/src/repos/delta/unified_patch.rs` | `similar` under `DIFF_DEADLINE` 150 ms; hunks-only unified grammar (`@@`, ` `, `+`, `-`) the UI parser already accepts; `PATCH_MAX_BYTES` 64 KiB cut on a hunk boundary (line boundary for a single oversized hunk); `all_added_patch` / `all_removed_patch` headers count the rows actually emitted. |
| Resolver | `crates/im_agent/src/repos/delta/delta_resolve.rs` (+ `delta_stamp.rs`) | Baseline ladder cache → index blob → none (all-added); images are metadata only; `READ_DEADLINE` 2 s around both blocking joins; a rename moves the baseline exactly once (`resettles == 0`); a budget-exhausted delete never spawns. |
| Worker | `crates/im_agent/src/repos/delta/delta_worker.rs` | `select!` over stop / armed deadline / nudge; drains `DRAIN_BATCH` 16 in first-seen order and resolves sequentially; skips to cache eviction when the bus has no receivers; stamps `seq` (1 per watcher start), `folded`, `withheld`, `dropped`; publishes `fileDeltaCounters` when the queue goes quiet or a window closes with counters no card would carry. |
| Service and limits | `crates/im_agent/src/repos/delta/mod.rs` | `DeltaService` (sync, await-free `note_change` / `note_rename`; `stop` cancels the git token, waits `STOP_GRACE` 250 ms, then aborts); `DeltaLimits` holds the process-wide `Semaphore(DELTA_READ_CONCURRENCY = 2)` on `AgentRuntime`. |
| Watcher marks | `crates/im_agent/src/repos/repo_watcher_delta_marks.rs`, `repo_watcher_events.rs` | `apply_change(path, change_type, DeltaIntent)` publishes `fileChanged` exactly as before and marks the queue on `Note`; two-path rename arms mark one rename; the Windows `From`/`To` split is paired by `PendingRename` inside `RENAME_PAIR_WINDOW` 80 ms (strictly below the settle window). |
| Index baseline | `crates/im_agent/src/source_control/index_blob.rs` | `git show :0:./<rel>` through `runner::run_read` with the worker's cancel token, `INDEX_BLOB_TIMEOUT` 5 s, stdout bound `MAX_DELTA_FILE_BYTES`; exit 128 / NUL / non-UTF-8 → `None`. |
| Wire types | `crates/im_agent/src/protocol/events_delta.rs` ↔ `app/src/shared/protocol_events_delta.ts` | `fileDelta { repoId, seq, path, fromPath?, kind, op, mtime, tracked?, folded, withheld, dropped, payload: text\|image\|opaque\|gone }` and `fileDeltaCounters { repoId, withheld, dropped }`; hand-kept in sync, `kind`-tagged payload, camelCase. |
| UI store | `app/src/lib/stream/stream_store.ts` (+ `stream_store_support.ts`, `stream_store_registry.ts`) | One store per repo outside React (`STORE_MAX` 4, visible pinned): `FLUSH_MS` 48 intake buffer, pure reducers (`stream_ring_apply.ts`, `stream_ring_apply_burst.ts`, `stream_ring_apply_support.ts`), ring ops (`stream_ring.ts`), cadence (`stream_cadence.ts`), burst detection (`stream_burst_detect.ts`), card grammar (`stream_card_grammar.ts`, `stream_card_body.ts`), every bound in `stream_bounds.ts`. History rows are seeded only from the repo's own `snapshot` event through `intake` (the registry routes every event by its `repoId`; the store drops a snapshot whose `repoId` is not its own; a ring holding any card is never reseeded) — there is no other seed route. Image deltas never form file cards: `stream_image_strip.ts` folds them into the `images` strip at the feed's tail (`stream_strip_types.ts`; a repeat path replaces its tile in place in the newest strip wherever it sits; a new path opens a new strip only past `IMAGE_STRIP_MAX` or once any card printed after the strip — never on time), `stream_strip_view.ts` owns the strip head grammar, and `stream_tile_targets.ts` the tile retention arithmetic. |
| UI hooks | `app/src/hooks/stream/{use_stream_host, use_repo_stream, use_stream_follow, use_stream_images}.ts` | One agent subscription for all repos (`StreamHost`, mounted once in `app.tsx`); `useSyncExternalStore` binding for the active repo; follow-scroll with the unread pill; bounded image tiles keyed by strip and path (`IMAGE_CARD_MAX_BYTES` 4 MiB, `IMAGE_FETCH_CONCURRENCY` 2, `MAX_IMAGE_TILES` 24 under `IMAGE_TILE_BYTES_BUDGET`, Blob URLs revoked on release). |
| UI surface | `app/src/components/stream/*`, `app/src/styles/stream/{stream_panel, stream_card, stream_card_body, stream_card_image, stream_motion}.css` | Panel shell with the shared header (STREAM cell first, LIVE / HELD / OFFLINE / UPDATE slot), scroller (`role="log"`), cards (head, text/image/burst/notice/history bodies), all arrival choreography in `stream_motion.css` (transform/opacity only; governor carve-out; hidden = instant). The strip row is a CSS grid (`repeat(auto-fit, minmax(min(STRIP_TILE_PX, row/STRIP_MIN_COLUMNS), 1fr))`): one tile spans the row, tiles share it, shrink to `STRIP_TILE_PX`, then wrap at a constant column width; the slot is 4:3 of the column capped at `STRIP_SLOT_MAX_PX`, so strip height is a function of tile count and panel width only. |
| Shared diff grammar | `app/src/lib/diff/diff_lines.ts`, `app/src/components/diff/diff_line_rows.tsx` | One parser and one row renderer for the workspace diff viewer and stream cards. |
| Persisted choice | `uiState.filesMode` in `app/src/shared/config/ui_state_schema.ts` ↔ `src-tauri/src/lib/config/types/ui_state.rs` | `stream \| auto \| latest \| active`, default `stream`, `.catch("stream")`; validated in Rust; no config-version bump. |

## Lifecycle

1. `RepoWatcher::start` loads the tracked set, constructs `DeltaService` (spawning the worker), and hands it to
   `EventContext`. Every notify event still publishes `fileChanged` first; on `DeltaIntent::Note` the path is
   marked on the settle queue (lock, mutate, `notify_one` — no IO, no `await`).
2. The worker wakes on the earliest deadline or a nudge, evicts baselines for dropped paths, drains a batch,
   charges the budget, and resolves each change: settled read under the process-wide permit, baseline ladder,
   `similar` diff, cache update, `fileDelta` broadcast. Counters that no following card would carry go out as
   `fileDeltaCounters`.
3. The event bus (128 slots) → WebSocket → host-agent relay (WSL repos) → `AgentEventSchema` → the single
   `StreamHost` subscription → `routeAgentEvent` → the repo's store. A flush runs the reducers; the conductor
   admits one resolved card per cadence tick into the twenty-card ring; the panel renders through
   `useSyncExternalStore`.
4. Stop order: `stop_tx` → unwatch → `DeltaService::stop` (cancel token, grace, abort; the cache drops with the
   worker) → task abort → recent-files flush.

## Invariants

- No IO and no `await` on the notify path; every std lock recovers from poisoning.
- Every card names its baseline (`SINCE LAST` / `VS INDEX` / `NEW` / `GONE` / `MOVED`), and a card is created
  only from a delta that arrived — nothing pending can vanish.
- Every dimension is bounded: settle queue, read permits, bytes per file, cache bytes, patch bytes, budget per
  window, ring size (twenty plus one card mid-exit), lines per card, image tiles, notices, stores.
- `seq` is strictly increasing per repo per watcher start; a forward gap prints a notice, a restart does not.
- Counters are never stranded: they ride the next `fileDelta` or a `fileDeltaCounters` event.
- Re-saves of one path inside `MERGE_WINDOW_MS` extend that path's newest live card; the extension keeps the
  card static and animates only the fresh rows.
- `fileChanged` and `snapshot` are byte-for-byte unchanged; Auto / Latest / Active are untouched.
- The governor carve-out is scoped to the stream scroller; hidden or minimized renders resting states.
- The scroller is a track, never a compressing column: every child is `flex: 0 0 auto`, every entry keeps its
  natural height, and overflow goes to the scroller's own scrollbar — no card is ever reduced to its spine and
  a clipped line. Narrow-chassis relief queries the scroller's inline size (`@container`), never the viewport.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| File still moving at read time | `Unsettled` → re-arm up to `MAX_RESETTLES`, then the final attempt accepts what is on disk. |
| File vanished between stat and read | `Missing` → dropped; the unlink's own delta follows. |
| Over 512 KiB, NUL bytes, or invalid UTF-8 | `opaque` card (`TOO LARGE FOR STREAM` / `BINARY`); baseline evicted. |
| Read or diff hangs | `READ_DEADLINE` 2 s → `opaque(unreadable)`; the permit is released, the pool thread finishes on its own. |
| Budget exhausted | Add/modify/rename/image: no event, baseline evicted, counted in `withheld`; delete: `gone` without a patch. |
| Queue over `QUEUE_CAP` | Mark discarded, counted in `dropped`, baseline evicted so the next edit says `VS INDEX`. |
| Broadcast bus lag | Deltas dropped for that subscriber; the UI prints `N EDITS NOT SHOWN` from the `seq` gap. |
| Agent older than `STREAM_MIN_AGENT_VERSION` | `AGENT UPDATE REQUIRED` empty state; the table modes still work. |
| WSL backend offline | `HELD` in the header and a notice for WSL repos; host repos unaffected; resume on `online`. |
| Watcher stopped mid `git show` | The cancel token kills the child; one last delta may still publish with baseline `none`. |
| Windows split rename further apart than 80 ms | Honest delete card plus add card instead of one rename card. |
| Rename then delete of the destination in one settle window | Collapses to a remove; prints `GONE` rather than the old content. |

## Related docs

- [docs/design/stream_panel_design.md](../design/stream_panel_design.md) — problem, goals, behaviour table, acceptance, accepted boundaries
- [docs/implementation/stream_panel_implementation.md](../implementation/stream_panel_implementation.md) — the build ladder and its receipt
- [docs/commands/verify_stream.md](../commands/verify_stream.md) — the installed-app witness route
- [docs/architecture/source_control_architecture.md](source_control_architecture.md) — the Git runner and watcher signal the delta pipeline builds on
