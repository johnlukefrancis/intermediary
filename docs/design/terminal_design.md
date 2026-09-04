# Integrated Terminal Design
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010, ADR-013

---

## Problem

Intermediary has replaced VS Code for moving between repos and worktrees, reading files, building bundles,
and managing the working tree. The one capability left in VS Code is a real interactive terminal. JL lives
in PowerShell 7 with a profile that defines the aliases the workflow runs on (`wb-code`, `codex`, `claude`,
`wsl`), so the terminal has to be JL's own pwsh — profile, environment, and aliases loaded — hosting genuine
interactive TUIs, not a command box. This design brings that terminal into the deck as the third rail
section, TERMINAL, beside ZIPS and SOURCE.

## Goals

- A ConPTY-backed PowerShell 7 session per tab, started in the active repo/worktree, with JL's profile
  loaded (`-NoLogo`, never `-NoProfile`), so `claude`, `codex`, `wsl`, and `wb-code` resolve exactly as they
  do in Windows Terminal.
- A WSL-rooted repo lands in bash inside that repo through `wsl.exe`, the way the `wb-code` alias does;
  `exit` returns to pwsh.
- Sessions survive rail, repo-tab, handset, and mode switches: switching parks a session, never disposes it.
- Interactive TUIs render, reflow on resize, and accept arrows, Ctrl+C, paste, and Shift+Enter.
- Closing a tab or the app ends the console-attached process tree the way Windows Terminal does, and an
  in-app `wsl` session never keeps the distro alive at exit (ADR-013 rule 4).
- The terminal looks like part of the deck: colours come from the theme tokens and the active tab accent,
  the font from `--font-mono`.
- No new socket, port, plugin, CSP, or capability (ADR-010).

## Non-goals

- Persisting sessions or scrollback across app restarts (the persisted choice is only which rail is
  active).
- A shell other than pwsh 7, a `-NoProfile` shell, or an app-injected prompt, alias, or environment beyond
  the WSL-root entry and the `TERM_PROGRAM` / `COLORTERM` identity.
- Per-repo or simultaneous multi-distro routing. `RepoRoot::Wsl` owns a native Linux path; the app's one
  configured WSL distro (or the actual default when unset) is the distro authority for both agents and
  terminals. A UNC distro segment is input-path transport, not a second durable repo authority.
- Split panes, a splitter between the deck and the rail, search, or a settings surface for the terminal.
- Routing bytes through the host or WSL agent, or through a Tauri shell or clipboard plugin.
- A `\\wsl.localhost` UNC start directory: WSL-rooted repos enter through `wsl.exe --cd`.

## MVP

Third rocker cell TERMINAL on the right rail (`DeckSectionSwitcher`), a fourth section on the handset
rocker. The terminal column: a tab strip (`PWSH 1`, `PWSH 2` …, `×`, `+`), the xterm host, and the
notices (`> STARTING PWSH` (shown until the shell paints its first visible byte, so the seconds a profile takes are covered; typed keys already reach the shell), `> PROCESS EXITED · CODE n` with RESTART / CLOSE, `> PWSH FAILED TO START`
with the message and RETRY / CLOSE, `> NO TERMINAL` with `+ NEW`). Sessions grouped per repo (`repoId`),
capped at 12 across the app. Six Tauri commands (`terminal_open`, `terminal_write`, `terminal_resize`,
`terminal_ack`, `terminal_close`, `terminal_clipboard_text`) and one raw-byte output channel per session.
`uiState.activeRail` gains `terminal` and is now mirrored in the Rust config struct, so the choice survives
a restart. The rocker cell's glyph is a bare prompt chevron and cursor bar (`>_`), chosen at the rendered 15 px against
the archive-box and branch glyphs: framed variants blurred into a block at rail size.

## Behaviour table

| Situation / input | Expected visible behaviour |
| --- | --- |
| TERMINAL selected on the rail (standard deck or workspace mode) | The right column shows the terminal column at the rail's persisted width; the drag divider between the columns resizes it (shared with ZIPS and SOURCE, 20-70 %, default 35 %). The choice persists across restarts. |
| First TERMINAL visit for a repo | One tab opens automatically (`PWSH 1`), once per repo per app run. After the user closes every tab the group shows `> NO TERMINAL` with `+ NEW` and never re-auto-opens. |
| Host-rooted repo (`root.kind === "host"`) | pwsh starts in that directory with JL's prompt and PSReadLine. |
| WSL-rooted repo with a native Linux path | Before ConPTY opens, a bounded non-login WSL control probe resolves one explicit distro and proves the exact path is a directory there. pwsh then starts at the user profile directory and, after the profile, runs `wsl.exe -d <distro> --cd '<path>'` (single quotes in the path doubled), so the tab lands in bash inside the repo. A normal `exit` returns to pwsh; a failed initial entry exits pwsh instead of leaving a misleading prompt in the user profile. |
| WSL-rooted repo whose path is `/mnt/<drive>/…` | The path is converted to its Windows form and pwsh starts there like a host root; no `wsl.exe` entry. |
| Start directory does not exist | The open fails before an interactive process is registered, with a message naming the directory (and the resolved distro for a native WSL root); `> PWSH FAILED TO START` never masks a fallback to the user profile. |
| `+` clicked | A new tab opens in the same repo root and becomes active; `+` is disabled when twelve terminal tabs are retained, including exited and failed tabs. |
| Tab strip | One button per tab labelled `PWSH n` (n stable for the tab's life); the shell's OSC title is the tooltip; `×` closes. |
| Typing, arrows, Ctrl+C without a selection, function keys | Sent to the pty as xterm encodes them; a TUI (`claude`, `codex`, `vim`) receives them unchanged. |
| Shift+Enter | Sends `ESC CR` (Claude Code's newline). |
| Column resized, fonts finish loading, window resized, rail switched back | The terminal refits to the host; the pty is resized only when cols or rows change (trailing-debounced ~75 ms); a running TUI reflows. |
| Rail switched ZIPS → TERMINAL → SOURCE → TERMINAL; repo tab A → B → A; standard ↔ handset; file opened or closed | Every tab keeps its scrollback, cursor, and running process. The inactive session is parked off-screen (not hidden in place), so xterm pauses rendering while the pty keeps running. |
| Handset deck | TERMINAL is a fourth rocker section; the terminal fills the chassis. |
| Shell exits (`exit`, or the process ends) | The tab keeps its scrollback and shows `> PROCESS EXITED · CODE n`; RESTART opens a fresh pty into the same xterm (scrollback kept); CLOSE removes the tab. |
| Open fails (pwsh missing, bad directory, cap reached in the backend) | `> PWSH FAILED TO START` with the message; RETRY re-opens, CLOSE removes the tab. |
| `×` on a running tab | The console closes first (attached clients get `CTRL_CLOSE`); if the tree is still alive after a bounded wait the Job Object is armed for kill-on-close, terminated, and observed within a second bound. pwsh, `wsl.exe`, and conhost end; a GUI app launched from the shell (a VS Code window from `wbide`) survives unless forced escalation was required. A failed Job receipt stays backend-owned for app-exit retry. The neighbour tab becomes active. |
| Long flood (`yes`, a large `type`) | Output is credit-gated (512 KiB high / 128 KiB low unacked) against cumulative sent/consumed watermarks, so a lost or duplicate acknowledgement is recoverable, the UI stays responsive, and Ctrl+C stops it. |
| Scrollback is longer than the viewport | xterm's native 14 px scrollbar becomes clearly visible against the deck. Its thumb has distinct normal, hover, and accent-toned active states and can be grabbed and dragged through the retained scrollback; the mouse wheel continues to work. |
| Window minimized or hidden during a long listing | Bytes are acked on receipt while the document is hidden, so the child never stalls on a window nobody is looking at; the listing finishes. |
| Window loses focus | Cursor blink stops (motion governor); it resumes on refocus. |
| Repo or group removed from the tab bar, or the same repo id is rebound to another root | Its old terminal group is closed; a retained tab can never restart against stale root authority. |
| App exits | Admission freezes and every opening, running, closing, or reaping transaction reaches its joined terminal receipt before the agent supervisor runs, so an in-app `wsl` session never holds the distro open at the idle probe. |
| Page reload (dev) | The old page's sessions are closed by the backend on navigation (`webviewNavigation`); the new page starts empty. |
| Modal open (`[data-intermediary-modal-root]`) when a session is adopted | Focus stays with the modal; the terminal takes focus on adopt otherwise. Escape inside the terminal never closes the workspace. |

## Keys and clipboard

Windows Terminal defaults, resolved before xterm sees the key:

| Key / gesture | Behaviour |
| --- | --- |
| Ctrl+Shift+C | Copy the selection. |
| Ctrl+Shift+V | Paste. |
| Ctrl+C | Copy when a selection exists, else `^C` to the pty. |
| Ctrl+V | Paste. |
| Right-click | Copy the selection if any, else paste; the native context menu is suppressed. |
| Shift+Enter | `ESC CR`. |
| Everything else | xterm's own encoding. |

Copy uses `navigator.clipboard.writeText` (already used by the app). Paste reads the clipboard through
the `terminal_clipboard_text` command, because WebView2 blocks `navigator.clipboard.readText` without a
permission prompt the app cannot grant from its window config; the text is fed to `terminal.paste`, which
brackets it when the running program asked for bracketed paste.

## Look

Deck tokens, not the Windows Terminal scheme. The xterm theme is read from the `.app` element at adopt
time and again when the accent or theme mode changes: `--terminal-bg`, `--terminal-fg`,
`--terminal-cursor`, `--terminal-cursor-accent`, `--terminal-selection`, and the sixteen
`--terminal-ansi-*` slots (`black` … `white`, `bright-black` … `bright-white`), each defined per theme
file from the existing success/error/info/warning, text, and accent tokens. The background carries
`--window-opacity-alpha` and the terminal allows transparency, so the deck substrate shows through like
every other panel. Font `var(--font-mono)` at 14 px; cursor bar with blink; scrollback 10,000 lines.
The native xterm scrollbar retains its full 14 px interaction target and uses opaque deck text colours for
its thumb, an elevated track, and the active accent while it is being dragged.

## Accepted boundaries

- **Console-first close, job as escalation.** Closing ends what is attached to the console, like Windows
  Terminal; the Job Object exists so the bounded wait is real, not as the primary close. A process that
  detaches from the console (a GUI app, a `nohup`/`tmux` survivor inside WSL) deliberately survives an
  ordinary close. Forced escalation owns and ends the whole Job tree; a failed Job receipt stays
  retained and blocks WSL idle teardown instead of being reported as final.
- **No persistence across restarts.** Sessions are processes; the app does not pretend to restore them.
  Only `uiState.activeRail` persists, and TERMINAL re-auto-opens one tab on the next visit.
- **Twelve retained terminals.** Every retained xterm/WebGL tab counts, including exited and failed tabs.
  Independently, every admitted backend transaction occupies one of the same twelve process/thread slots
  through opening, running, closing, and reaping; its two long-lived workers are joined before release.
- **Token theme, not a terminal scheme.** The palette is derived from the deck tokens and follows the tab
  accent; there is no per-terminal colour scheme or opacity control.
- **Hidden window acks on receipt.** While the document is hidden the backend's credit window is not the
  bound; xterm's own buffer is. A build that prints while the window is minimized completes.
- **Font is the deck's `--font-mono`.** No terminal-specific font or size setting.
- **pwsh 7 only.** The shell is resolved from Program Files, then PATH; there is no shell picker.
- **Windows ConPTY is an owned spawn seam.** One narrow Windows adapter creates the pseudoconsole and
  applies both the ConPTY and Job Object attributes in the same `CreateProcessW` call. The installer build
  proves that branch compiles; the installed-app route remains the product witness.

## Acceptance

1. The TERMINAL cell is present beside ZIPS and SOURCE; the divider between the columns resizes the rail and the width survives a restart.
2. A host repo tab opens in its folder with JL's prompt and PSReadLine; a WSL repo tab lands in bash
   inside the repo and `exit` returns to pwsh.
3. `wb-code`, `claude`, and `codex` resolve; their TUIs render, take arrows, Ctrl+C, paste, and
   Shift+Enter, and reflow on resize.
4. ZIPS → TERMINAL → SOURCE → TERMINAL and repo A → B → A keep every tab's scrollback and process;
   each open tab keeps one WebGL renderer, paused while parked.
5. `exit` shows the notice; RESTART and CLOSE work; closing the last tab shows `+ NEW`.
6. Closing a tab ends pwsh, `wsl.exe`, and conhost (Task Manager) while a VS Code window launched from
   the shell survives.
7. A flood stays responsive and Ctrl+C stops it; its scrollbar thumb is plainly visible and draggable
   through the resulting scrollback; a long listing finishes while the window is minimized.
8. Twelve retained exited/failed tabs prevent a thirteenth tab; closing one frees exactly one frontend
   resource. A repo id rebound to another root closes its old group before another terminal opens.
9. Closing the app with an in-app `wsl` session open logs the terminal shutdown before
   `wsl_exit_teardown outcome=terminated … reason=idle`.
10. Removing a repo ends its terminals; TERMINAL persists across an app restart and auto-opens one tab.
