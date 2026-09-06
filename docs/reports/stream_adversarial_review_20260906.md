# Stream Panel — External Adversarial Review (2026-09-06)
Updated on: 2026-09-06
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006, ADR-007, ADR-008, ADR-009

## Context

JL commissioned an external adversarial review of the staged 0.1.23 candidate — the whole Stream
feature (agent delta pipeline, `fileDelta` wire, UI store, cards, motion) sitting in the index on top
of `1f99568`. It is the sixth external review indexed under Reports and the fifth review round on
the Stream candidate itself — the first four were adjudicated inline into the design doc's accepted
boundaries.

The review returned **no P0** and **six P1 contract failures plus one P2**. Every P1 named the same
class of defect: a fact the design contract states as true (a card names its baseline; a burst is
caused by one flood; an overload invalidates what it dropped; loss is visible; a bound is owned) that
the implementation could not actually guarantee. All seven findings were verified against the staged
source, adjudicated by the coordinator, and **accepted**. All seven are closed in this round; three
carry a stated boundary the closure deliberately does not cross.

Two lanes executed the closures concurrently against one frozen wire contract (three additive changes:
`mtimeMs` on the image payload, `seq` on `fileDeltaCounters`, optional `maxBytes` on `readImageFile`).
No new event was added: a proposed `streamDropped` event was rejected because delta and counters share
one per-repo sequence, so every drop is already a visible `seq` gap.

This document is the durable record of the round. The behaviour contract is
`docs/design/stream_panel_design.md`; the shipped mechanisms are
`docs/architecture/stream_architecture.md`; the execution owner is
`docs/implementation/stream_panel_implementation.md`.

## Findings

### P1-1 — Save-then-stage race on a first sighting

- **Claim.** A card labelled `VS INDEX` did not prove it compared against the index the save was made
  against. The worktree was read first and the index blob fetched afterwards, so an agent that saved
  and staged inside the settle window produced a card whose `VS INDEX` chip described a baseline that
  had already moved. `SINCE LAST` was documented as "since the previous card", which the merge and
  withhold paths make untrue.
- **Verdict.** Genuine. Baseline identity is the one thing the card grammar promises.
- **Closure.** The index baseline is read **before** the worktree read on a first sighting and is
  carried across every re-settle of that path, so the compared pair is captured in one causal order
  rather than two independent ones. The zero-length rule is exact: on a first sighting (no cached
  baseline) whose settled text is empty while the captured index baseline is non-empty, the resolver
  returns `Drop` — **no card, and the cache is left untouched (no baseline stored)** — instead of a false
  all-removed; the next sighting is still a first sighting against the index. An empty read against an
  empty or absent index, or against a cached baseline, still emits
  (`delta_resolve_text::settled_empty_against_index`, `delta_resolve_text_tests.rs`). The design wording is corrected: `SINCE LAST` means *since
  the agent's previous sighting of that path*, which is the previous card only when no sighting was
  merged, withheld, or dropped.
- **Owner files.** `crates/im_agent/src/repos/delta/delta_resolve.rs`,
  `crates/im_agent/src/repos/delta/delta_read.rs`,
  `crates/im_agent/src/repos/delta/settle_queue.rs`,
  `crates/im_agent/src/source_control/index_blob.rs`, `docs/design/stream_panel_design.md`.

### P1-2 — Image pixels were not bound to the revision that produced the card

- **Claim.** The image payload carried a byte count and a mime type; the panel then fetched pixels
  through `readImageFile` in a separate round trip with no identity check. A file rewritten between
  the delta and the fetch produced a tile showing the *new* pixels under the *old* card, and no bound
  existed on decoded pixels — only on source bytes, which a 4 MiB PNG can expand past by two orders
  of magnitude.
- **Verdict.** Genuine, and the only finding that could put wrong content in front of JL.
- **Closure.** `DeltaPayload::Image` gains `mtime_ms` (`mtimeMs` on the wire), stamped at the metadata
  read that produced the event. The tile fetch accepts pixels only when both the byte count and
  `mtimeMs` match the payload; a mismatch renders `IMAGE CHANGED` and no bytes are decoded.
  `readImageFile` gains an optional `maxBytes`, enforced at the byte owner in both backends **before
  the read** (`UNSUPPORTED_IMAGE_FILE`, "Image exceeds the requested size bound"), so the gate no
  longer depends on the caller. Decoded pixels are bounded per tile (`MAX_TILE_PIXELS` 24 MP) and
  across the ring (`MAX_RETAINED_PIXELS` 64 MP).
- **Owner files.** `crates/im_agent/src/protocol/events_delta.rs`,
  `crates/im_agent/src/protocol/commands.rs`, `crates/im_agent/src/repos/image_file_reader.rs`,
  `crates/im_agent/src/repos/delta/delta_resolve.rs`,
  `crates/im_host_agent/src/runtime/local_host_repo_backend.rs`,
  `app/src/shared/protocol_events_delta.ts`, `app/src/shared/protocol_repo_commands.ts`,
  `app/src/lib/agent/messages.ts`, `app/src/hooks/stream/use_stream_images.ts`,
  `app/src/lib/stream/stream_tile_targets.ts`, `app/src/lib/stream/stream_bounds.ts`.

### P1-3 — No causal owner for a burst

- **Claim.** The agent's read budget refilled on a wall-clock window, so a checkout that took longer
  than `BURST_WINDOW` was charged `BURST_BUDGET` reads *per window* rather than once — a slow flood
  cost unbounded reads. On the UI side a burst could close while its own deltas were still queued, so
  the late members printed as ordinary cards behind the burst that was supposed to absorb them, and
  the membership set itself was unbounded.
- **Verdict.** Genuine on both sides. A burst is a causal object, not a time slice.
- **Closure.** The budget refills only when the queue is quiet or holds fewer than `DRAIN_BATCH`
  pending marks (`BURST_REFILL_MAX_PENDING`), never mid-run — one checkout costs `BURST_BUDGET` reads
  however long it takes. The UI applies pending deltas *before* it closes a burst, and a closed burst
  keeps absorbing its member paths for `BURST_ABSORB_GRACE_MS` (6 s) so a late delta bumps `RESOLVED`
  instead of opening a card. Membership is capped at `BURST_MEMBER_CAP` (256) with
  `BURST_TOP_DIRS_TRACKED` (32) directories tracked for the top-three strip.
- **Owner files.** `crates/im_agent/src/repos/delta/delta_budget.rs`,
  `crates/im_agent/src/repos/delta/delta_worker.rs`,
  `app/src/lib/stream/stream_burst_detect.ts`, `app/src/lib/stream/stream_ring_apply_burst.ts`,
  `app/src/lib/stream/stream_bounds.ts`.

### P1-4 — Lossy overload invalidation

- **Claim.** `QUEUE_CAP` overflow recorded dropped paths so the worker could evict their baselines,
  but the dropped-path record was itself bounded and lossy: past its own capacity a path could be
  dropped without its baseline being evicted, so the next edit of that path would diff against text
  the agent had already stopped tracking and print a *wrong* patch under a truthful chip. A withheld
  rename evicted one side of the pair only.
- **Verdict.** Genuine. A bound that silently degrades correctness is worse than one that degrades
  completeness.
- **Closure.** Dropped paths dedup up to `QUEUE_CAP`; past that the whole baseline cache for that repo
  is cleared, so every subsequent card is honestly `VS INDEX` or `NEW` rather than possibly wrong. A
  withheld rename evicts **both** source and destination.
- **Owner files.** `crates/im_agent/src/repos/delta/settle_queue.rs`,
  `crates/im_agent/src/repos/delta/baseline_cache.rs`,
  `crates/im_agent/src/repos/delta/delta_worker.rs`.

### P1-5 — Unbounded transport and invisible loss

- **Claim.** The per-connection event queue behind the 128-slot broadcast bus was an unbounded channel
  in both agents (a pre-existing condition the design recorded as a follow-up, but `fileDelta` events
  of up to 64 KiB made it a live memory risk). On the UI side the intake buffer was unbounded, so a
  stalled flush could hold an arbitrary backlog. And `fileDeltaCounters` carried no sequence number,
  so a dropped counters event was invisible — the one event class whose loss silently changed the
  numbers JL reads.
- **Verdict.** Genuine. Bounded-and-visible beats unbounded-and-assumed.
- **Closure.** Both agents relay through two bounded lanes per connection (`event_bus_relay.rs`, the
  lane decided from the event variant at broadcast time and carried on the bus message, never re-parsed).
  Stream events (`fileChanged`, `fileDelta`, `fileDeltaCounters`) ride a `try_send` lane under two real
  ceilings — `EVENT_QUEUE_CAP` (1024 slots) and `EVENT_QUEUE_BYTES` (8 MiB queued across both lanes) —
  with a drop counter and one rate-limited warn per drop burst, so a stalled client holds at most 8 MiB.
  Every other event kind rides a `CONTROL_QUEUE_CAP` (64) lane the relay **awaits**: a full control lane
  lets the broadcast receiver lag (the pre-existing behaviour) rather than silently dropping a snapshot,
  topology, source-control, bundle, error, or backend-status event, and the socket writer drains the
  control lane first. `fileDeltaCounters` gains
  `seq`, consumed from the same per-repo sequence as `fileDelta` (a counters event increments the
  sequence exactly like a delta), so a dropped counters event *is* a `seq` gap and prints the existing
  `> N EDITS NOT SHOWN` notice. UI intake is capped at `INTAKE_CAP` (1024), dropping the oldest
  `fileChanged` first and counting what it dropped. `gone` events are budgeted separately at
  `GONE_BUDGET` (64 per window) rather than being unbudgeted. No `streamDropped` event was added:
  one shared sequence already makes every drop visible, and a second loss channel could itself be lost.
- **Owner files.** `crates/im_agent/src/server/connection.rs`,
  `crates/im_host_agent/src/server/connection.rs`,
  `crates/im_agent/src/protocol/events_delta.rs`,
  `crates/im_agent/src/repos/delta/delta_worker.rs`,
  `crates/im_agent/src/repos/delta/delta_budget.rs`,
  `app/src/shared/protocol_events_delta.ts`, `app/src/lib/stream/stream_store.ts`,
  `app/src/hooks/stream/use_stream_host.ts`, `app/src/lib/stream/stream_bounds.ts`.

### P1-6 — Detached blocking work

- **Claim.** `READ_DEADLINE` was applied by racing the blocking join against a timer while the read
  permit was held by the *awaiting* task. On a timeout the permit was released while the blocking read
  was still running, so the two-permit semaphore stopped bounding concurrency exactly when a stalled
  filesystem made that bound matter. Image metadata reads carried no deadline at all.
- **Verdict.** Genuine (ADR-009: a bound that a timeout can defeat is not a bound).
- **Closure.** The permit is **owned by the blocking job** and moves into it, so it is released when the
  read actually finishes — a timeout abandons the *result*, never the permit. Image metadata reads run
  under `READ_DEADLINE` like every other blocking step.
- **Owner files.** `crates/im_agent/src/repos/delta/delta_resolve.rs`,
  `crates/im_agent/src/repos/delta/delta_read.rs`, `crates/im_agent/src/repos/delta/mod.rs`.

### P2 — Documentation described a system the tree did not implement

- **Claim.** The implementation doc claimed status **Built**; the architecture doc described the image
  tile slot as 4:3 while the design contract and the CSS say 16:10; `docs/known_issues.md` described
  deletes as exempt from the read budget; and the roadmap and changelog described 0.1.23 as shipped
  when no installed-app acceptance had run.
- **Verdict.** Genuine. Documentation that overstates state is a second, wrong authority.
- **Closure.** This report; implementation status is now **Candidate — installed-app acceptance owed**
  with the review round in its live receipt; the architecture ownership table says 16:10; the known
  issues entry says deletes are budgeted separately (`GONE_BUDGET`), not exempt, and records the
  bounded queue as closed; the roadmap and changelog entries keep describing 0.1.23 and now say the
  build is a candidate pending JL acceptance.
- **Owner files.** `docs/reports/stream_adversarial_review_20260906.md`,
  `docs/implementation/stream_panel_implementation.md`, `docs/architecture/stream_architecture.md`,
  `docs/design/stream_panel_design.md`, `docs/known_issues.md`, `docs/roadmap.md`,
  `docs/changelog.md`, `docs/guide.md`.

## Actions

| # | Action | Lane | State |
|---|---|---|---|
| 1 | Index baseline read first and carried across re-settles; zero-stat first sighting prints nothing | Rust | Closed |
| 2 | `mtimeMs` on the image payload; matched-only pixel fetch; `maxBytes` enforced at the byte owner in both backends; `MAX_TILE_PIXELS` / `MAX_RETAINED_PIXELS` | Rust + UI | Closed |
| 3 | Budget refills only when quiet or below `BURST_REFILL_MAX_PENDING`; deltas applied before a burst closes; `BURST_ABSORB_GRACE_MS`, `BURST_MEMBER_CAP`, `BURST_TOP_DIRS_TRACKED` | Rust + UI | Closed |
| 4 | Dropped-path dedup to `QUEUE_CAP`, whole-cache clear on overflow, both rename endpoints evicted | Rust | Closed |
| 5 | `EVENT_QUEUE_CAP` bounded queues with drop counters in both agents; `seq` on `fileDeltaCounters`; `INTAKE_CAP`; `GONE_BUDGET` | Rust + UI | Closed |
| 6 | Read permits owned by the blocking job; image metadata under `READ_DEADLINE` | Rust | Closed |
| 7 | This report, the guide row, and the five contract-doc corrections | Docs | Closed |
| 8 | Rebuild, install, and run the installed-app witness (`docs/commands/verify_stream.md`); JL acceptance; then commit | JL | Owed |

## Accepted boundaries

These are the edges the closures deliberately do not cross. They are end states, not open defects.

- **The index cannot be snapshotted at notify time.** ADR-009 forbids IO on the notify path, so the
  index baseline is captured at the settle deadline, not at the instant of the save. A stage that
  lands inside the settle window is therefore reflected in the baseline; the chip stays truthful about
  *what was compared*, which is the guarantee the card grammar makes.
- **The final event of a stream, if dropped, is invisible until the next event.** Seq gaps are
  detected by the *arrival* of a later event; there is no heartbeat, and one was rejected as a
  recurring cost paid to make a rare last-event loss visible slightly sooner.
- **An in-flight blocking read cannot be cancelled at watcher stop.** Owning the permit inside the
  blocking job makes the bound real, but a `spawn_blocking` read on a stalled filesystem still runs to
  completion on its pool thread; it is simply no longer observed, and the watcher does not wait on it.
- **`streamDropped` was rejected.** Drops are made visible through `seq` gaps on the shared
  delta/counters sequence plus one log line per drop burst. A dedicated loss event travels the same
  lossy channel it reports on.

## References

- `docs/design/stream_panel_design.md` — behaviour contract, card grammar, bounds, accepted boundaries.
- `docs/architecture/stream_architecture.md` — shipped ownership, lifecycle, invariants, failure modes.
- `docs/implementation/stream_panel_implementation.md` — execution owner, PR ladder, live receipt.
- `docs/commands/verify_stream.md` — the installed-app witness route that closes acceptance.
- `docs/known_issues.md` — the remaining owed witness and the accepted boundaries above.
- `docs/reports/zips_tree_write_surface_review_20260904.md` — the previous external review round.
