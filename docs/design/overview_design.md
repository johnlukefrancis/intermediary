# Repository Overview Design
Updated on: 2026-10-12
Owners: JL · Codex
Depends on: ADR-000, ADR-005, ADR-007

## Product contract

Overview is one continuous feed of work across active repositories. Every change
carries its repository identity. It uses the whole small application window,
with one read-only Git column beside the feed instead of the repository's ZIP,
Git or terminal rail. Stream supplies the visual
grammar: clear filenames over subordinate directories, compact badges, restrained
surfaces and readable diffs. There is no grid of repository dashboards.

Only repositories with changes in the last fifteen minutes appear. Git-ignored,
untracked paths (engine `Saved/`, build output) are not work: the agent never reports
them, so they enter neither the feed, Stream nor recent files. A modify that settles
with no textual change is not news either: like Stream, Overview drops the edit and
retracts its raw change signal. An open tab
or a healthy watcher alone does not qualify. Roots enter automatically when work
arrives and leave after inactivity. Saved activity supplies startup orientation,
labelled as history; it never fabricates live edit content. Newly observed activity
uses observation time, independent of file modification time.

The navigation tab names the destination; there is no repeated Overview title.
One compact glass toolbar strip spans both columns. It contains active-root
filters and actual monitoring issues, and disappears when both are absent. A chip
filters the feed and the Git column together. Updates are continuous, without a Pause
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

## Git column

The right column is one glass panel headed `[ WORKING TREE ]`, sharing the
repository tabs' splitter and persisted rail width. It lists only working trees with
something to show: clean roots normally hide, with a short exception for a lane
whose last agent turn just ended. With no changed or finishing root the column
reads `ALL CLEAN`. Reading, held and failed roots stay visible. The header counts
changed roots as `N CHANGED`; the temporary clean row does not increase that count.

Crosstalk turn events add a repository-coloured glow. Once a lane is observed,
its dirty tree has a soft, slow breathing glow on the title and visible items
while it remains in Overview. When the last working session in a lane becomes
idle and Git is known clean, a stronger pulse fades over 24 seconds. A recent
turn-end temporarily admits that root to the shared Overview filter, then normal
filesystem inactivity rules resume. The clean row says `CLEAN`; this means the
turn ended with no pending changes, not that the agent's entire task is complete.
Reduced motion holds a steady dirty highlight and gently fades the clean one.

A config group is one project whose members are worktrees. A grouped project gets
one sticky accent rule with its group label, and each changed worktree hangs beneath
it on an accent guide with its own label, branch line and `SOURCE ›`. An ungrouped
repository keeps a single block whose sticky head copies the feed's repository
rule. Labels and `SOURCE ›` open that root's SOURCE rail. The mono branch line shows
ahead/behind and the `CONFLICTS`, `TRUNCATED` and `MID-OPERATION` badges.

Sections match the SOURCE rail exactly: MERGE CONFLICTS (alert tone), STAGED
CHANGES and CHANGES, where CHANGES includes untracked paths; empty sections are
omitted. Sections start collapsed, so each worktree reads as its branch line and
section counts; a section head expands it until the block remounts. Rows use the
SOURCE face on one 24 px line: icon, name,
head-truncated directory, then the change badge. Each section shows eight rows, then
`+N MORE IN SOURCE`. A click, Enter or Space opens the row's diff full-pane; the
opened entry is frozen until reopened. A deleted text row is inert. Staging,
discard, commit, push and context menus stay in SOURCE.

State lines replace the groups: `READING WORKING TREE` before a first status, the
SOURCE error heading on failure, and `WSL HELD`, `UPDATE REQUIRED` or `OFFLINE` from
the watch availability. A reread keeps the last status with a 4 px pulse in the
head, never a spinner. With no working repository there is no column, only the
centered empty panel. Narrow windows stack the column under the feed, capped at
45vh. The handset deck switches between FEED and GIT; GIT carries the shown
repositories' change count and a conflict alert.

## Ownership and bounds

One app-lifetime activity subscription supplies Stream and Overview. Overview
captures every configured root, including worktrees never opened this session.
Its bounded projection retains six recent paths and twelve settled edits per
root. Each edit has at most six lines of 180 characters. The visible feed uses
recent work only, ordered chronologically and capped at eighty entries.

The existing agent pipeline owns file changes and settled content. Overview
adds no filesystem watches, WebSocket or Git polling. The Git column reads the
shared source-control status registry, so it always agrees with SOURCE. Settled image edits carry
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
- Cold startup waiting is distinct from failed coverage. Failed, disconnected and
  incompatible watches remain visible as concise status, including when the
  active feed is empty. WSL failure does not mark host roots unavailable.
- Connecting, quiet and unavailable states use the same centered glass empty
  panel as the no-repositories screen. They retain the app's visual language.
- Lost edits use Stream's rule: only a forward sequence gap or the agent's
  withheld and dropped counters count, and a watcher restart is a new stream. A
  transport flap alone is never a loss. `N EDITS NOT SHOWN` sits where the loss
  happened, merges until the next edit and ages out like an edit. The feed is
  bounded monitoring, not a durable activity journal or a claim about agent task
  status.

## Crosstalk speech bubbles

While Overview is visible, newly delivered Crosstalk messages appear as small
speech bubbles beside the native window. The side with enough free monitor work
area wins; when neither side fits, the same surface sits inside the right edge.
Moving, resizing or changing monitor repositions it. Leaving Overview, opening
global notes or minimizing hides and clears it. New bubbles appear above neighbouring
windows even when another app has focus or partly covers Intermediary. Clicking a
covering window brings it above the bubble; clicking elsewhere leaves the bubble
visible. Only a new message raises the surface again. Bubbles never activate or
raise the main window and do not take keyboard focus.

Each card shows the original purple Codex icon or orange Claude Code mascot,
sender repository and faint session tag, recipient repository, bottom-right time
and three lines of actual text. Client names and click instructions are absent.
Hover holds its remaining fourteen-second lifetime; click expands the body and
keeps it open until folded or dismissed.
At most three cards remain, preserving a held card during incoming bursts.
Spring-in motion comes from the adjacent window edge; departure shrinks and
fades. Reduced-motion preferences use opacity only. Dark, warm and light colours
come from the app's existing theme tokens. Configured repository/group accents
colour the sender label, icon surround, border and glow; unmatched agents use the app accent.

When the other participant answers after the previous card disappears, that
message returns as a muted two-line echo above the reply. Its original icon,
repository accent and session tag return first; the reply settles beneath it.
Clicking either text expands the exchange and holds its lifetime. Only the latest message between
the exact two agents, seen within five minutes, qualifies. A still-visible original
is never duplicated. Context clears with the surface and is never a saved transcript.

The observer reads only new complete message records from Crosstalk's canonical
`events.jsonl`. Status events never become speech. Startup does not replay history,
and Intermediary neither claims inboxes nor changes agent delivery.

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

JL's 6.6 MB Coilbound HTML exposed the agent's 512 KiB live-text limit. The
shipped policy keeps full text baselines up to 512 KiB and, for text files up to
16 MiB (and at most 2^20 lines), retains one 64-bit fingerprint per line instead,
charged against the same 16 MiB per-repo baseline budget and LRU. A large file's
first sighting diffs against its index blob (read up to 16 MiB, fingerprinted)
and prints `VS INDEX`; an untracked one prints `NEW` with its first rows. Later
saves print `SINCE LAST` against the fingerprints: added and context rows carry
real text, `+N −M` is exact, removed rows are omitted and each hunk header says
how many removed lines were not retained, with `TRUNC` raised on both surfaces.
Above 16 MiB the card stays `Opaque(tooLarge)`. An atomic save that renames an
unindexed temp over the file keeps the destination's own baseline, and when the
destination is uncached its own index blob is the fallback. Every published row
is cut at 1 KiB, so one enormous minified line still shows its start instead of
an empty `TRUNC` card.

The configured root is `/home/johnf/dev/Coilbound`. Its `Coilbound.html` measured
6,659,613 bytes and was tracked and clean during inspection. Git hunks are
therefore possible when it differs from the index, but cannot reconstruct a
previously observed version once that difference has been committed or staged.
