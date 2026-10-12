# Crosstalk Overview Bubbles
Updated on: 2026-10-12
Owners: JL · Codex
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

## Live receipt

- Target: `/home/johnf/dev/intermediary`, `master`, base `200b9ad`; clean at entry.
- Commission: real agent messages as small speech bubbles beside Overview,
  adaptive left/right placement, inside placement when maximized, hover hold,
  click expansion, gentle expiry, no stolen focus. Install and relaunch for JL.
- Verdict: extend Overview. Crosstalk owns delivered messages; the Tauri shell
  owns one bounded read-only observer and one transparent owned window. Its
  dedicated frontend owns at most three cards and expiry. The main frontend
  supplies Overview visibility and theme only. No repository polling or mailbox
  consumption. No message content executed as HTML.
- Source coordination: Crosstalk author installed canonical JSONL feed `events.jsonl`
  in Windows and WSL (producer commit `28eee8d`). Delivered-message and status kinds,
  unique IDs, participants, actual text and sent time; readable CLI log is a projection.
- Steering: JL requested playful arrival/departure motion, existing app themes and
  sender repository colours. Native renderer uses shared theme tokens and configured
  repo/group accents, reduced-motion opacity, spring entrance and shrink/fade exit.
- Native first witness: actual delivered messages appeared in the installed app;
  JL confirmed "definitely works" with a screenshot. Follow-up asks for original
  purple Codex and orange Claude Code mascot icons, no client-name or click-hint
  text, and the time at bottom-right. Those refinements are installed and witnessed.
- Extension: delayed replies recall the preceding message as a compact animated
  echo above the reply. The renderer owns recent pair context: exact reversed
  participants, latest message only, five-minute eligibility and sixteen pairs.
  Only already-disappeared messages return; visible originals are not duplicated.
  Leaving Overview or minimizing clears both cards and context. No disk history.
- Verification: TypeScript and ESLint pass; 141 TS tests and two focused Windows
  Rust tests pass; Windows cargo check passes. Existing terminal unused-mut and
  test-only dead-code warnings remain. Header coverage and full file ledger updated.
- Native witness: delayed reply recalls the expired original with distinct icons
  and accents in one 206px card; click expands both; reduced motion uses opacity.
  Left/right/inside placement, hover/expiry, burst limits, no focus stealing and
  three themes inspected. Minimize/restore and repo navigation clear cards and
  context. Restoring now keeps the empty native window hidden: Windows visibility
  is applied on the UI thread, independently of Tao's cached flag.
- Installed build: current NSIS/MSI packaging succeeded; silently installed
  executable SHA256 `331EA7392EF41C1665592EB4AA9BD6697521C572F8BC77C27B3DA0BF101C8825`
  matches the build. Reopened normally; temporary DevTools port 9225 is closed.
  The final live delayed exchange was delivered through Crosstalk. JL's current
  repository tab kept bubbles hidden, as intended; subsequent user tab/window
  changes are preserved. Implementation and verification are complete.
- Evidence roots: `src-tauri/src/lib/crosstalk`, `app/src/crosstalk`,
  `target/crosstalk_witness` (ignored); current Overview design/architecture.
- Tag refinement: existing sender tags appear faintly after the repo name in
  both headers. Installed native screenshot `agent_tags.png` distinguishes
  `INTERMEDIARY #439a` from recalled `INTERMEDIARY #7903`; no runtime/CSP errors.
  TypeScript, ESLint, Windows cargo check and installer packaging pass again.
  Silently installed, reopened normally, original repository tab and window bounds
  restored; temporary DevTools port 9225 closed.
  Logic tests were not rerun for this presentation-only refinement. The old
  `live_bubbles.png` is entirely transparent, an empty capture rather than a white
  application pane; it is not product evidence.

## Acceptance

- [x] Delivered messages show actual participants and text; status stays silent.
- [x] Left/right/inside placement follows the native window and monitor bounds.
- [x] Hover holds, click expands, bursts stay bounded and focus is preserved.
- [x] Leaving Overview or minimizing hides the surface without stale replay.
- [x] Required frontend/Rust checks, focused invariants and native inspection pass.
- [x] Current installer built, silently installed and reopened normally.

## Changed owners

- `app/src/crosstalk/{main.tsx,bubble.tsx,speech.css,conversation.ts,identity.ts}`:
  bounded cards, recent pair context, product icons, expansion and themed motion.
  `conversation_test.ts` and `identity_test.ts` protect pair and colour attribution;
  `assets/{codex.svg,claude_code.svg}` bundle the original-colour product mascots.
- `app/crosstalk.html`, `vite.config.ts`: dedicated companion entrypoint.
  `app/src/hooks/overview/use_crosstalk.ts` and `app/src/app.tsx`: visibility,
  theme and configured repo/group styles from the main app.
- `src-tauri/src/lib/crosstalk/{mod.rs,observer.rs,feed.rs,placement.rs}`:
  native lifecycle, bounded event reader and monitor-aware window placement.
  `src-tauri/src/lib/mod.rs` integrates setup, commands, events and shutdown.
- `src-tauri/Cargo.toml`, `Cargo.lock`: filesystem notifications and macOS
  transparency support. `src-tauri/capabilities/crosstalk.json` and
  `src-tauri/tauri.conf.json`: event-only companion capability.
- Overview design, architecture, verification commands, changelog and guide
  describe the behavior; `docs/inventory/file_ledger.{json,md}` is regenerated.

## Limits

The producer's JSONL delivery log is best-effort across concurrent Windows/WSL
writers; malformed records are skipped and incomplete lines are buffered. Pair
context uses exact participants and recency because the feed has no reply ID.
Ignored native screenshots and witness results live in `target/crosstalk_witness`.

JL retains visual and interaction acceptance. macOS native witness is outside
this Windows session.
