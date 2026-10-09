# Repository Overview Implementation
Updated on: 2026-10-09
Owners: JL · Codex
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

## Live receipt

- Target: `/home/johnf/dev/intermediary`, `master`, base `2d79133`.
  Remote fetched; local and origin/master match and include Stream/macOS work.
- Requested result: one tailored, repository-labelled stream of actual recent
  work, locally installed and reopened. JL owns visual and UX acceptance.
- Implemented: active membership with fifteen-minute expiry; one chronological
  feed; compact repository filters; reading follow; shared revision-
  checked image thumbnails; full-pane current-file viewing; shared glass empty
  states; aligned navigation and persisted Overview destination.
- Ownership: one app-lifetime event route; bounded per-root Overview projection;
  shared image and workspace owners with Stream. Root identity gates events,
  history, async refresh errors and file requests. See the current architecture.
- Reviews: Sol High design/candidate/image challenges and Luna evidence passes
  completed. Root replacement, history provenance, watch recovery, backend version,
  image read accounting and stale image revision findings repaired.
- Startup witness: local image opened in 3.8 seconds during WSL initialization;
  WSL activity recovered without a repository-tab visit. Background history now
  starts with host roots and admits one request across effect generations. Socket
  connectivity is separate from sticky transport health; connected timeouts keep
  their finite recovery pass, and explicit recovery retries earlier history errors.
- Coilbound investigation: existing 512 KiB input/baseline limits reject its
  large HTML before computing a diff. Normal deltas already carry changed hunks
  with three context lines. Atomic replacement may also lose the destination
  baseline. Findings are in the design follow-up; no large-file policy change.
- Temporary image/text witness repository removed through the normal UI. All
  fourteen original configured roots preserved; prior selected Blender repository
  restored behind Overview. Original dirty Linux bundle retained as
  `target/overview_witness/im_agent_before`.
- Delivered: current Windows 0.1.24 installed and reopened normally on Overview.
  Installed executable hash matches the built executable; debugging port is closed.
  Final native capture confirms recovered WSL, no monitoring warning, and five
  actually active roots in the unified feed. No commit or push.
- Header follow-up: JL saw absent arrows on some headers; they returned before
  reproduction. Native inspection confirmed every group had an unconditional,
  visible, hit-testable action, including both Blender roots. The intermittent
  disappearance remains unconfirmed. Each entire header is now its one navigation
  button with a fixed-size SVG arrow. Native name, arrow and Enter activation open
  the correct repository Stream. Every rendered header retained its arrow through
  scrolling at 1078px and 360px, with no horizontal overflow. Typecheck, ESLint,
  Windows cargo check and packaging pass. No new permanent test for this small
  presentation change; source/native checks cover it. Installed and returned to
  Overview with the previously selected Blorb repository preserved behind it.
- Worktree visibility investigation: Blender's main and islands roots remain
  independent throughout configuration, watcher ownership, saved history and feed
  grouping. At 01:58 UTC, islands' latest source modification and saved observation
  both identified `projects/blorb/quests/island_waterways.md` at 01:31:19 UTC;
  main's latest observation was at 01:52:33 UTC. Islands had expired from the
  fifteen-minute view, matching the native UI. No source-edit misattribution found.
  The activity window measures observed edits, not whether an agent is still
  thinking or rendering. Both `work/` symlinks also reach the same external output
  directory; earlier shared job results appear in both histories, independently
  labelled. No activity-window or watcher change made for this investigation.
- Delivered refinement: removed the Overview Pause control and its retained
  state, plus routine Live indicators in Overview and Stream. Scroll-away reading,
  file-workspace retention and actual monitoring failures remain. The Overview
  toolbar contains filters/issues only and vanishes when both are absent.
  Typecheck, ESLint, Windows cargo check and NSIS/MSI packaging passed. Native
  inspection confirmed the clean connecting state, both healthy headers, new
  activity arriving, and Stream's real OFFLINE/HELD status clearing on recovery.
  Installed bytes match the built executable. Left Overview open with Workbench
  wb-code preserved as the selected repository; no debugging port was enabled.
  No new permanent tests or broader Rust/TS suite rerun for this presentation cut.

## Changed owners

- `components/overview`, `hooks/overview`, `lib/overview`, `styles/overview`:
  consolidated feed, active membership, history, reading and file interactions.
- `overview_view.tsx`, app/navigation/config/tab-bar owners: global destination,
  persistence, compact navigation and full-pane workspace routing.
- Shared image, repository-workspace and empty-state owners: reuse Stream and
  existing app behavior; obsolete Stream-specific thumbnail owners retired.
- Agent protocol/watch/history owners and Tauri UI-state schema: exact root
  attribution, stopped-watch detection, observation timestamps and persistence.
- Design/architecture/guide/roadmap/changelog, verification commands and inventory:
  current contracts, file ownership and repeatable verification. Version 0.1.24.

## Verification

- Frontend: typecheck and ESLint pass; TypeScript tests 113/113.
- Rust: im_agent 367/367, im_host_agent 39/39; Windows cargo check and the
  Overview UI-state persistence test pass. Version contract passes at 0.1.24.
- Windows: current release build, NSIS/MSI packaging and normal installation pass.
- Native product: interleaved real repository edits, exclusion of idle roots,
  scroll retention through new edits, return to live, exact Stream
  navigation, restart persistence, shared image loading, full-pane image/text
  opening and return, context menus, dark/light/warm themes and narrow layouts.
  Image workspace inspected at 360px with no horizontal overflow.
- File discipline: changed handwritten owners stay within 300 lines and two-line
  comments. Headers and full ledger regenerated; whitespace check passes.
- Evidence: ignored `target/overview_witness`; commands in
  [verify_overview](../commands/verify_overview.md).
- Limits: macOS native witness not run here. Linux app cargo check cannot run
  without Pango/Cairo development dependencies; Windows check is the native gate.
  Existing Windows unused-mut, Node deprecation and frontend chunk-size warnings
  remain. WSL cold initialization can still be slow; already-sent history requests
  are not cancelled by view/config changes. Native drag-out was not exercised;
  Overview routes it through the existing shared drag owner.

## Completion

- [x] Verify remote, authority, architecture and implementation.
- [x] Challenge and repair the candidate; run focused and required checks.
- [x] Inspect ordinary product interactions and update docs/inventory.
- [x] Install current build normally and leave Overview open for JL.
