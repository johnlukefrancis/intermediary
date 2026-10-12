# Crosstalk Overview Bubbles
Updated on: 2026-10-12
Owners: JL · Codex
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

## Live receipt

- Options toggle follow-up, base `bfa1b58`, clean at entry: add a saved
  **Crosstalk bubbles** switch to Options → General, enabled by default.
  Persisted config owns the preference; the existing presentation `active` gate
  hides/clears bubbles immediately and drops messages while disabled. Turn
  observation and repo pulses continue. Re-enabling waits for new messages.
  Complete: installed Options control visually inspected; default-on, immediate
  native hiding, dropping messages while off, off across app restart, no replay
  and new messages after re-enable pass (`target/crosstalk_witness/toggle_results.json`,
  `toggle_options.png`). Typecheck, ESLint, 144 TS tests, Windows cargo check and
  27 Rust config tests pass. Existing terminal warnings are unrelated.
  Rebuilt MSI/NSIS and silently installed matching executable SHA256
  `58690534F1C2428FBA714E837DA3F583C3912956FC1030C9C66280320A35A9F0`.
  Testing leaves bubbles enabled; app relaunched normally with the inspection
  port closed. Uncommitted.
- Changed owners: `app/src/shared/config/persisted_config.ts` and
  `src-tauri/src/lib/config/types.rs` persist the default-on preference;
  `app/src/hooks/use_config_actions.ts` and `use_config.tsx` expose its setter;
  `app/src/components/status_bar.tsx`, `options_overlay.tsx` and
  `options/general_section.tsx` wire the Options switch; `app/src/app.tsx`
  projects it through existing visibility. Architecture and changelog document
  the behavior; this receipt records installation and verification.

## Previous delivery

- Active follow-up (base `77f5d1e`, lane-glow work remains uncommitted): JL wants
  bubbles visible over a neighbouring focused window, even when Intermediary is
  partly obscured. JL clarified that re-engaging a covering window must put it
  above the bubble, whereas clicking elsewhere must leave the bubble visible.
  Raise only on a displayed message's arrival, at the ordinary window level with
  no activation or owner movement. Resizing/expiry/theme changes must not raise.
  The companion reports each accepted arrival through its existing size command;
  native placement uses a synchronous promote/demote with
  `SWP_NOACTIVATE | SWP_NOOWNERZORDER`. Direct `HWND_TOP` stayed behind focused
  Chrome in the native witness due to Windows foreground permission rules.
  Clicking already-focused Chrome also left the normal-tier bubble in front.
  A visible-only UI-thread mouse hook now yields only on an overlapping window's
  button-down, passes the input through and detaches on hide/shutdown.
  Always-on-top was inspected successfully, then rejected by this clarified UX.
  Native witness now targets idle focused Chrome under a new bubble, click-to-cover,
  unrelated clicks, later arrivals, resize without resurfacing and normal hiding.
  JL also uses full-width portrait stacks: preserve inside-right placement, a
  contained scrollable stack and no outward pointer when neither side fits.
  Completed: installed native checks pass for arrival above focused Chrome,
  click-to-cover, non-overlapping clicks leaving cards visible, resize staying
  covered, same-height full-stack replacements resurfacing, and expiry/minimize.
  On the real portrait monitor, the full-width app contains the stack with no
  pointer; clicking the gallery on the other display leaves it visible and gives
  Chrome focus. Native fixtures stay out of the canonical Crosstalk log.
- Current verification: TypeScript, ESLint, 144 TS tests, Windows cargo check,
  two native Crosstalk tests (including portrait containment), headers, full ledger
  and installer packaging pass. Existing terminal warnings remain unrelated.
  Native evidence: `target/crosstalk_witness/arrival_results.json`,
  `portrait_results.json`, `portrait_inside.png`. Earlier witness setup needed
  hidden helper processes and hit-tested clicks while JL used other windows;
  foreground moves to other agent terminals were not app focus theft.
- Installed executable SHA256:
  `7BDF962A2EF2534075E20E69155C1E579BDCD4BDE7995E5BC27E38CB1CC3CEDB`.
  Build and installed bytes match. Reopened normally, saved native arrangement
  restored and inspection port 9225 confirmed closed. Work is uncommitted;
  macOS native behavior was not inspected in this Windows session.

## Earlier delivery

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
  Windows-only `pointer.rs` owns click-to-cover without focus-change hiding.
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
