# Verify the Stream Panel End-to-End
Updated on: 2026-09-06
Owners: JL · Agents
Depends on: ADR-010, ADR-012

Manual witness of the STREAM mode of the left activity panel in the **installed** Windows app. Image cards
render from Blob URLs under the release CSP (the dev CSP is null), and the burst, follow-scroll, and motion
behaviour only mean anything against a real agent editing a real repo. Run this after touching
`crates/im_agent/src/repos/delta/`, `crates/im_agent/src/protocol/events_delta.rs`, `app/src/lib/stream/`,
`app/src/hooks/stream/`, `app/src/components/stream/`, or `app/src/styles/stream/`. Build and install first with
`docs/commands/build_installer_from_wsl.md`.

Each step names what to do and what proves it. The acceptance list and behavior table in
`docs/design/stream_panel_design.md` are the oracle; this file is the route.

## 0. Baseline

Open one **host-rooted** and one **WSL-rooted** repo tab. Expect the left panel to open in STREAM (the first
rocker cell lit), a `LIVE` dot beside the rocker, and either compact history rows from the recent list or
`> WATCHING <REPO> — WAITING FOR EDITS`. Open DevTools (`Ctrl+Shift+I`) and keep the Console visible.

## A. Text cards and the store

1. From a terminal, edit a tracked code file in the host repo and save. Expect a `[M] path` card to enter within
   about a third of a second, its hunk lines printing one by one, the chip `VS INDEX`, `+N −M`, and the spine
   sweep. The Console shows the `fileDelta` envelope with `baseline: "index"`, `folded`, `withheld: 0`.
2. Save the same file again after a few seconds. Expect a new card with `SINCE LAST` showing only the new lines.
3. Save it twice within a second. Expect the newest card to extend (new lines beneath) and show `×2`.
4. Create a new file. Expect a `text-added` card that unfolds, every line `add`, chip `NEW`.
5. Delete a tracked file. Expect a `[D]` card printing the last content as removed lines that strike through;
   the card rests dimmed. Delete an untracked, never-seen file. Expect a 48 px `REMOVED` ghost (`GONE`).
6. Rename a file. Expect one `[R] old → new` card (`MOVED` when unchanged). Switch to Auto: the table still
   shows the rename as a new row (Auto Files is untouched).
7. Click a card. Expect it to expand in place (up to the expand cap) and the feed to stop following. Double-click
   a modified-file card. Expect the worktree diff to open in the shared workspace; close it. Expect the ring back,
   older cards static, and `> N CHANGES WHILE AWAY` if edits landed meanwhile. Drag a card past 6 px: the OS drag
   starts with the staged copy. Right-click: the file actions menu.
8. Repeat step 1 in the WSL repo. Same result; the `LIVE` dot stays lit.
9. Switch to Latest and back to STREAM, then switch repo tabs and back. Expect the ring intact each time.
10. Fill the ring: let an agent (or a loop of saves) land 30 or more edits so the ring is full and the cards are
    taller than the panel. Expect **every** card to keep its natural height — full head, full hunk body, footer —
    with the newest card no taller than its neighbours; a scrollbar on the scroller itself (not on the panel
    around it); the wheel over the cards moving them; and no card squeezed to its spine and a clipped half-line.
    Scroll up, let more cards land, and expect the view to hold with the `▼ N NEW` pill; click it to re-pin.
11. Run in the Console after any burst:

```js
document.querySelectorAll(".stream-card").length
```

Expect `21` or fewer: the ring holds 20 cards, and one more may be mid-exit — the evicted card plays its exit
keyframe and is spliced on the next admit by design.

## B. Image strips and bursts

1. Drop ONE screenshot-sized PNG into a folder of the repo. Expect a strip card headed `1 IMAGE · <dir>` whose
   single tile spans the panel's width and is large, its checkerboard slot 16:10 of that width (capped at 480 px
   tall), the image letterboxed inside it (never cropped). Drop a second PNG about 15 s later. Expect it to join
   the SAME strip (no new card), the row splitting into two equal halves. A third: thirds. A fourth: about four per
   row at the standard panel width. Keep dropping PNGs at any pace: expect the columns to shrink evenly until a
   column would drop under 200 px, then the row to wrap, every row — the last one included — keeping the same
   column width (12 tiles = three rows of four). After six the head reads `6 IMAGES · <dir>` (no numeric badge;
   the head has the file icon, the name cell, the `A 6` tally, and the clock), the tiles dropped in left to right
   one chain step apart in arrival order, each with its `A` badge and file name beneath, and one footer reading
   `<clock span> · <total bytes>`. Hover a tile: the title shows `path · W×H · bytes`. Drop a tiny icon (say
   16×16). Expect it at most doubled (32×32) and centred on the checkerboard, never stretched to fill the slot.
2. Drag the divider to narrow the panel, then widen it. Expect the grid to re-flow as the width changes — tiles
   growing and shrinking with their columns, rows re-wrapping — and the strip's height to change only with the
   width and the row count; never fewer than three tiles per row however narrow the panel goes. Drop a few PNGs
   with the panel narrow: same rule.
3. Overwrite one of the PNGs. Expect its tile replaced in place (same slot, same order) with the new pixels and an
   `×2` counter; the strip keeps its position and its exact height. Click the strip: that tile becomes a
   two-column BEFORE / AFTER pair, each half exactly one column wide (the other tiles keep their width). Click
   again: it collapses back.
4. Delete a PNG. Expect the tile greyed with a rule struck through its slot, or a `DELETED` ghost in the same-sized
   slot when no pixels were retained; the strip's height does not change.
5. Drop a `.heic` and a PNG over 4 MiB. Expect `NO PREVIEW · HEIC` and `NO PREVIEW · <size>` tiles at the same slot
   size as every other and **no** `readImageFile` request (DevTools → Network → WS → frames).
6. Drop more than 24 images across several strips (each strip holds at most 12; only a text card printed after a
   strip opens a new one — save a text file between batches; a quiet gap of any length never does). Expect the
   oldest fetchable tiles to read `RELEASED` at exactly their previous size — no strip changes height and the
   reading position does not move.
7. Check out a branch that differs by hundreds of files. Expect one burst card (`×N · <seconds>`, top directories,
   `A · M · D` strip) slamming in, its `RESOLVED` counter ticking to at most 32, a `> N EDITS WITHHELD · BURST`
   notice, the other cards still in the ring, and SOURCE refreshing as usual. The agent log shows at most two
   concurrent delta reads.

## C. Motion, governor, DevTools

1. With an agent editing, click into another window while the app stays visible. Expect cards to keep arriving
   **and animating** (the Stream is exempt from the pause); the substrate and the LIVE pulse stop as before.
2. Minimize the app, let a few edits land, restore. Expect the cards already in place (no animation) followed by a
   cascade, or one burst card if many landed.
3. Enable OS reduced motion. Expect cards to appear instantly with no stagger.
4. DevTools → Performance, record 30 s of real edits. Expect no `Layout` or `Recalculate Style` entries attributable
   to `.stream-*`, every `stream-*` animation composited, and 55 fps or better through a flood. Memory → take a heap
   snapshot after a long session; expect a flat heap and at most `MAX_IMAGE_TILES` (24) Blob URLs plus the retained
   BEFORE tiles of modified paths and any in-flight fetches.
5. Scroll up while cards arrive. Expect the view to hold and a `▼ N NEW` pill; click it to re-pin.

## D. Persistence, offline, version

1. Close the app, delete `uiState.filesMode` from `%LOCALAPPDATA%\com.johnf.intermediary\config.json`, relaunch.
   Expect STREAM. Pick Auto, restart. Expect Auto (the Rust mirror kept it).
2. Stop the WSL backend (Options → Restart Agent, or stop it from a terminal). Expect `HELD` and
   `> WSL BACKEND OFFLINE — STREAM HELD` on the WSL repo only; on reconnect `> RECONNECTED — RESUMING`.
3. Point the app at an older external agent. Expect `AGENT UPDATE REQUIRED · <version>+` in STREAM while Auto,
   Latest, and Active still work.
