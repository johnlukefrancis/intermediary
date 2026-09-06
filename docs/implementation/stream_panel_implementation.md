# Stream Panel Implementation
Updated on: 2026-09-06
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006, ADR-007, ADR-008, ADR-009, ADR-010, ADR-012
Status: **Candidate — installed-app acceptance owed.** Rungs 1–8 landed and checked (tsc, eslint, TS tests,
cargo check with zero warnings, Rust tests, Windows-side Tauri crate check), and the second adversarial
review's six P1 closures landed on top of them; the 0.1.23 installer build and the installed-app witness
(`docs/commands/verify_stream.md`) are the acceptance, and nothing here is Built until JL has seen it run.
This document is the durable execution owner for the Stream panel; the design contract is
`docs/design/stream_panel_design.md`.

## Intent and witness

Add a **Stream** mode to the left activity panel and make it the default: a terminal-like, continuously
scrolling live feed that prints the actual content of each edit as coding agents work — diff hunks for modified
files, new-file bodies, deletion cards, images dropping in — sequenced and animated, bounded to twenty cards so
it never grows or lags. Witness (`docs/commands/verify_stream.md`): JL runs a coding agent against a configured
repo in the installed Windows app and sees each save print as a content card within roughly 150–300 ms, images
drop in, deletions strike, a 500-file checkout collapses into one burst card without jank, and old cards
truncate away.

## Owner decision

Verdict: **extend** the repo watcher with one additive delta fact and the panel with one mode; **tune** the
diff viewer (extract `parsePatch` into a shared owner), the config model (extract `UiState`; persist the
left-panel mode), and the pointer-drag threshold (one shared hook).

- The agent process next to the disk owns "what changed inside this file". `crates/im_agent/src/repos/delta/`
  settles each path (120 ms trailing, 500 ms max hold), reads it off the notify path on a blocking worker
  behind a global two-permit semaphore, diffs it against the previous sighting (or the Git index the first time)
  with the `similar` crate under a deadline, and publishes one `fileDelta` event with the baseline named on the
  wire. Reads are budgeted (32 per 2 s per repo); withheld paths produce a counter, never an event.
- The UI owns view state only: a per-repo store outside React (`app/src/lib/stream/`) with a twenty-card ring,
  a cadence conductor, click-expand, follow-scroll, and bounded image tiles fetched through the existing
  `readImageFile` under a 4 MiB gate. Cards are created only from deltas that arrived; nothing can vanish or end
  as a bare filename. The same file re-saved inside 1.5 s extends the newest card (`×N`).
- `fileChanged` and `snapshot` are untouched. No new command, socket, port, CSP, or Tauri surface.
- The stream keeps animating while the window is visible but unfocused (a scoped carve-out of the motion
  governor, recorded as a design-system amendment); hidden or minimized lands cards instantly.
- Images flow left to right as strips (JL, 2026-09-06 live run); the scroller is a track, never a compressing column.

Decisions JL made on 2026-09-06: persist `uiState.filesMode` (default `stream`); extend the newest card on
re-edits; click expands / double-click opens; seed with compact history rows; keep animating while visible;
add `similar`; first card is the diff vs the index, labelled; add a minimal `node --test` runner.

## Implementation route (PR ladder)

Each rung runs `docs/commands/checks_local.md`; new files carry the two-line header, then the header and ledger
scripts in `docs/commands/workflow/closeout_checks.md`.

1. **Docs first** — this document, `docs/design/stream_panel_design.md`, `docs/guide.md` rows.
2. **Rust pure core + tests** — `similar` dependency; `repos/delta/{settle_queue, baseline_cache,
   unified_patch, delta_read}.rs` with `cfg(test)` units.
3. **Protocol** — `protocol/events_delta.rs` + `tests_delta.rs`; zod mirror `app/src/shared/protocol_file_meta.ts`,
   `protocol_events_delta.ts`; `fileDelta` joins `AgentEvent` on both sides.
4. **Watcher wiring** — `repo_watcher_startup.rs` extraction, `DeltaService` in `EventContext`, `DeltaIntent`
   on `apply_change`, `delta_resolve.rs`, `delta_worker.rs`, `source_control/index_blob.rs`, `EventBus::has_receivers`.
5. **UI foundation** — `lib/diff/diff_lines.ts` + `components/diff/diff_line_rows.tsx` extraction,
   `lib/files/files_mode.ts`, persisted `uiState.filesMode` (zod + Rust `ui_state.rs` mirror + validation),
   `useSetFilesMode`, `HelloState.agentVersion`, the STREAM rocker cell, `tabs/repo_tab_file_panel.tsx`,
   `StreamPanel` shell with history rows, `scripts/test/run_ts_tests.mjs`.
6. **Store, reducers, hooks, text cards** — `lib/stream/*` (+ tests), `use_drag_out_pointer.ts`,
   `hooks/stream/*`, `components/stream/*` text path, `stream_card.css`.
7. **Images and bursts** — `use_stream_images.ts`, `stream_image_strip.tsx` / `stream_image_tile.tsx`, `stream_burst_card.tsx`.
8. **Motion and sequencing** — `stream_motion.css`, pressure bands, governor carve-out, design-system amendment.
9. **Closeout** — `verify_stream.md`, system overview, roadmap, changelog, ledger, `STREAM_MIN_AGENT_VERSION`,
   rebuild and reinstall the local installer (`docs/commands/build_installer_from_wsl.md`), run the app.

## Verification checklist

- [x] Rung 1: design doc has Problem, Goals, Non-goals, MVP, Naming, Behavior table, Acceptance, Accepted
      boundaries, governor amendment; both docs registered in `docs/guide.md`.
- [x] Rung 2: `cargo check` (workspace) and `cargo test -p im_agent` green; every new file ≤ 250 LOC.
- [x] Rung 3: round-trip tests green; `tsc`/`eslint` green; no import cycle; a pasted `fileDelta` parses in DevTools.
- [x] Rung 4 (unit + in-process witness; host/WSL daemon witness owed): one delta mark per `RenameMode::Both` with unchanged unlink + add; watcher files ≤ 300 LOC;
      `fileDelta` envelopes observed for edit / create / delete / rename / image / checkout on a host repo and a
      WSL repo; agent stops cleanly (`docs/commands/verify_wsl_agent_tree_kill.md`).
- [x] Rung 5 (pixel identity of the workspace diff owed to the witness): workspace diffs pixel-identical after the extraction; `filesMode` round-trips through the Rust
      config; STREAM is the default; `test:ts` green; handset FILES shows the shell.
- [x] Rung 6 (reducers proven by 43 TS tests; in-app witness owed): cards print in the diff grammar; ring holds 20; re-save extends the newest card; click / double-click /
      drag / context menu work from a card; mode switch and workspace open keep the ring; `.stream-card` count ≤ 20
      after a checkout.
- [x] Rung 7 (lifecycle proven by reading; in-app witness owed): six PNGs → six tiles, the seventh degrades the oldest; overwrite → two-up; delete → ghost; heic →
      `NO PREVIEW` without a request; checkout → one burst card.
- [x] Rung 8 (static review: transform/opacity only; taste pass owed): JL taste pass; no Layout/Recalc from `.stream-*`; ≥ 55 fps through a flood; unfocused keeps
      animating; minimized lands instantly; reduced motion instant.
- [ ] Rung 9: `docs/commands/workflow/closeout_checks.md` end to end; installer rebuilt, reinstalled, app running.
- [ ] Rung 10 (review round 5): the six P1 closures re-checked in the installed app — a first-sighting card after a save-then-stage, an image overwritten mid-fetch (`IMAGE CHANGED`), a slow checkout costing one budget, a `QUEUE_CAP` overflow printing only `VS INDEX` afterwards, a `seq` gap notice under a stalled socket, and a stalled read that never frees its permit.

## Live receipt

- Target: `/home/johnf/dev/intermediary`, branch `master`, base `1f99568`.
- Approved plan: `/home/johnf/.claude/plans/claude-i-have-a-silly-backus.md` (2026-09-06).
- Frontier: rungs 1–8 landed; four adversarial reviews (0 P0, 11 P1, 16 P2, 15 P3) adjudicated — every P1 and the behaviour-visible P2/P3s fixed, the rest recorded as accepted boundaries in the design doc and `docs/known_issues.md`; version bumped to 0.1.23.
- Adversarial closure (2026-09-06): an external review found six P1 contract failures and one P2; two closure rounds landed (index-first baselines, revision-bound pixels with `maxBytes` at the byte owner and a re-stat, causal budget refill, lossless invalidation, two-lane bounded transport with `seq` on counters, owned permits with bounded waits, the zero-length first-sighting rule) — `docs/reports/stream_adversarial_review_20260906.md`. During the first round a lane restored non-owned files from the index and wiped two parallel lanes' edits; both were recovered by replaying their recorded write commands; lane briefs now forbid formatters and restores over non-owned files.
- **Review round 5 (2026-09-06, external adversarial review of the staged 0.1.23 candidate — `docs/reports/stream_adversarial_review_20260906.md`): 0 P0, 6 P1, 1 P2, all accepted and all closed in this round** against one frozen wire contract (`mtimeMs` on the image payload, `seq` on `fileDeltaCounters`, optional `maxBytes` on `readImageFile`; no new event — drops are `seq` gaps):
  - P1-1 index baseline read before the worktree read and carried across re-settles; a zero-stat first sighting prints no card; `SINCE LAST` documented as *since the previous sighting*.
  - P1-2 image pixels bound to the revision (`bytes` + `mtimeMs`, else `IMAGE CHANGED`); `maxBytes` enforced at the byte owner in both backends; `MAX_TILE_PIXELS` 24 MP and `MAX_RETAINED_PIXELS` 64 MP.
  - P1-3 causal burst ownership: the agent's budget refills only when quiet or below `BURST_REFILL_MAX_PENDING`; the UI applies deltas before closing a burst and absorbs members for `BURST_ABSORB_GRACE_MS`, capped by `BURST_MEMBER_CAP` / `BURST_TOP_DIRS_TRACKED`.
  - P1-4 dropped paths deduped to `QUEUE_CAP`, whole-cache clear on overflow, both rename endpoints evicted.
  - P1-5 `EVENT_QUEUE_CAP` 1024 bounded per-connection queues with drop counters in both agents; `INTAKE_CAP` 1024 in the UI; `GONE_BUDGET` 64; counters on the shared sequence.
  - P1-6 read permits owned by the blocking job; image metadata under `READ_DEADLINE`.
  - P2 this document's status, the 16:10 slot aspect in the architecture doc, the `GONE_BUDGET` wording in `docs/known_issues.md`, and candidate wording in the roadmap and changelog.
- Next: rebuild, install, JL acceptance per `docs/commands/verify_stream.md`, then commit. (Three live rounds on GLITCHFISH already drove the scroller-track fix, image strips, proportional tile sizing, and the cross-repo seed fix; taste numbers tune live.)
- Open: taste numbers (`LINE_CAP` 12, `CADENCE_BASE_MS` 260, `RING_SIZE` 20) tune at the witness; `STREAM_MIN_AGENT_VERSION` is finalized with the release version.
