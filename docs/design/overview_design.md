# Repository Overview Design
Updated on: 2026-10-09
Owners: JL · Codex
Depends on: ADR-000, ADR-005, ADR-007

## Product contract

Overview is one continuous feed of work across active repositories. Every change
carries its repository identity. It uses the whole small application window,
without the repository's ZIP, Git or terminal rail. Stream supplies the visual
grammar: clear filenames over subordinate directories, compact badges, restrained
surfaces and readable diffs. There is no grid of repository dashboards.

Only repositories with changes in the last fifteen minutes appear. An open tab
or a healthy watcher alone does not qualify. Roots enter automatically when work
arrives and leave after inactivity. Saved activity supplies startup orientation,
labelled as history; it never fabricates live edit content. Newly observed activity
uses observation time, independent of file modification time.

The navigation tab names the destination; there is no repeated Overview title.
One compact toolbar contains active-root filters and actual monitoring issues;
it disappears when both are absent. Updates are continuous, without a Pause
button or a routine Live indicator.
Repository accents and short labels identify work
without repeating full roots, watch status and explanatory headings on every
entry. Consecutive changes from the same repository share a repository head.
Active repository chips offer a quick focus filter. An explicit action opens a
repository's full Stream: its entire feed header is one button, with a fixed-size
arrow at the right. Full root identity remains available in a tooltip.

The feed is one glass panel in the same deck inset as the file workspace, so opening
a file never shifts the frame. Overview reuses Stream's shared pieces directly: the
`.diff-line` renderer, the follow pill, notice rows, badges and the strip checkerboard.
Its card chassis, head tracks and uppercase meta chips copy Stream's values. The one
deliberate difference is that the 3 px spine carries the repository accent, not the
operation, because the badge already names the operation. Section heads and filter
chips use the repository tab's mono uppercase lettering in that repository's accent.

## Ownership and bounds

One app-lifetime activity subscription supplies Stream and Overview. Overview
captures every configured root, including worktrees never opened this session.
Its bounded projection retains six recent paths and twelve settled edits per
root. Each edit has at most six lines of 180 characters. The visible feed uses
recent work only, ordered chronologically and capped at eighty entries.

The existing agent pipeline owns file changes and settled content. Overview
adds no filesystem watches, WebSocket or Git polling. Settled image edits carry
the exact announced revision into the shared thumbnail loader. Both surfaces use
the same byte/pixel limits, two concurrent reads per mounted view and twenty-four
retained image slots. A replaced slot can also retain its captured prior version.
Older revisions never display newer pixels. Consecutive images share compact
rows. Clicking an image or filename opens the actual current file in the app's
full-pane workspace, using the existing image/text viewers and file actions.
Closing returns to the mounted Overview, preserving filters and reading state.
The detailed Stream registry retains its four-store LRU. Activity events carry
the exact configured root path; replaced roots cannot inherit delayed events or
persisted history from another root. This contract requires agent 0.1.24.

History refresh occurs on connection/reconciliation with one request in flight,
starting with host roots. Disconnected WSL roots wait for reconnection; a connected
backend can recover from a slow first handshake through queued refreshes.
Raw observations and settled previews remain distinct evidence;
missing previews are disclosed. A delta's content and timestamp belong to that
delta, never an inferred join with a later raw notification.

## Reading and availability

- New entries follow the bottom while the reader is there. Scrolling up holds
  position and exposes a compact return-to-live control.
- Opening a file retains the feed behind its workspace. Closing returns to
  current activity with the reader's scroll/focus behavior preserved.
- Cold startup waiting is distinct from lost established coverage. Failed,
  disconnected and incompatible watches remain visible as concise status,
  including when the active feed is empty. WSL failure does not mark host roots
  unavailable.
- Connecting, quiet and unavailable states use the same centered glass empty
  panel as the no-repositories screen. They retain the app's visual language.
- Sequence gaps and withheld previews remain disclosed. The feed is bounded
  monitoring, not a durable activity journal or a claim about agent task status.

## Acceptance

Observe independent edits in multiple real repositories through the installed
Windows app. Confirm their labels, automatic entry/expiry, chronological feed,
follow behaviour, image previews and exact Stream navigation. Inspect the small
window, narrow layout, themes and keyboard focus. Check startup, historical
orientation and unavailable monitoring without an idle repository catalogue.

Corroborate root replacement, history provenance, retained bounds and watch
recovery with focused tests. Run required frontend/Rust checks and current native
packaging. Install and reopen Overview for JL; JL owns visual and UX acceptance.

## Large-file follow-up

JL's 6.6 MB Coilbound HTML exposed the agent's existing 512 KiB live-text input
limit. The UI excerpt cap is not the cause: ordinary deltas already contain
changed hunks with three context lines. Investigation found a separate tracked
Git-diff route with a 2 MiB output cap and twenty-second deadline; those budgets
are unsuitable to adopt unchanged for recurring live previews. A future bounded
large-file path must name its baseline and account for atomic replacement saves,
whose temporary source path can otherwise select an absent baseline. This was
investigated at JL's request; no new large-file runtime policy is implemented.

The configured root is `/home/johnf/dev/Coilbound`. Its `Coilbound.html` measured
6,659,613 bytes and was tracked and clean during inspection. Git hunks are
therefore possible when it differs from the index, but cannot reconstruct a
previously observed version once that difference has been committed or staged.
