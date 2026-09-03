# Source Control Design
Updated on: 2026-09-03
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

---

## Problem

Intermediary has replaced VS Code for moving between repos and worktrees, reading files, and building
context bundles. The one remaining reason to open VS Code is its Source Control view: seeing which files
changed in the active repo/worktree and how many, what is staged versus unstaged, staging and unstaging,
writing a commit message, and committing. This design brings that capability into Intermediary so the
working tree can be managed without leaving the app.

## Goals

- Show the active repo's changed files with counts, split into STAGED CHANGES, CHANGES, and MERGE CHANGES.
- Stage and unstage per file and per section; commit with a message; discard a file's changes behind a
  confirm; open a per-file diff in the shared workspace; push and pull with ahead/behind counts.
- Work for WSL-rooted and host-rooted repos through the agent that already owns each root, with no new
  socket, port, or Tauri surface (ADR-010).
- Stay current without polling: the watcher signals Git-state and working-tree changes.
- Fit every existing mode: standard deck, workspace (file open), and handset.
- Surface the same awareness in the ZIPS file explorer: changed files and directories carry Git-status
  decorations derived from the same status, so work is visible without opening SOURCE.

## Non-goals

- Branch management, merge/rebase tooling, history browsing, blame, or hunk-level staging.
- Running Git from the Tauri process or the webview.
- Cancelling a running commit/push from the UI (see Cancellation).
- A second layout breakpoint or a permanent third column.

## MVP

Right-rail segmented icon rocker (`DeckSectionSwitcher`: archive-box ZIPS cell, git-branch SOURCE cell) on
the existing right column; the Source Control column with status line (branch, ahead/behind, sha, refresh,
pull, push), warnings, commit box, three sections, and rows; diff kind in the shared workspace; handset
rocker prepends a stacked-documents FILES cell; `uiState.activeRail` persisted globally. Protocol:
`sourceControlStatus`, `sourceControlDiff`, and one tagged `sourceControlAction` (stage, unstage, discard,
commit, push, pull); event `sourceControlChanged`.

## Naming

Intermediary already uses "staged" for drag-handoff staging (`StagedInfo`, `stagedByPath`, `.badge--staged`,
`stageFile`). Git-side identifiers therefore say `index` / `worktree` / `conflict` (`SourceControlEntry.area`)
and protocol names are `sourceControl*`; only user-facing copy says "STAGED CHANGES". `.badge--staged` is
never used for Git state.

## Behavior table

| Situation / input | Expected visible behavior |
| --- | --- |
| Repo tab opens (any mode) | Status is fetched for the active repo; the SOURCE tab shows the total change count in accent (hidden at zero). |
| SOURCE rail selected in the standard deck or workspace mode | The right column shows the Source Control column; ZIPS is one click away; the choice persists across restarts and across the 980/860 resize band. |
| Handset deck | The icon rocker shows FILES / ZIPS / SOURCE cells (stacked-documents / archive-box / git-branch glyphs); picking ZIPS/SOURCE also sets the persisted rail. With a file or diff open, handset shows the workspace only (close returns to the deck section). |
| ZIPS rail with a changed working tree | File rows whose path is in the status carry a tinted name and a `[letter]` badge (the same `CHANGE_BADGES` palette as SOURCE rows); directory rows carry a tinted name and a count of distinct changed paths beneath them, colored by the worst change beneath; deleted files count toward their directory without a row of their own; expanded directories re-list in place on `sourceControlChanged` so a newly created file appears with its badge. |
| File edited, created, deleted, or renamed in the working tree (not under node_modules/target or the repo's ignore globs) | `sourceControlChanged` arrives within ~250 ms (coalesced); the column refetches once after a 300 ms trailing debounce. |
| External `git add` / `git commit` / branch switch in a terminal, main repo or linked worktree | Same as above via `.git` metadata watches (`index`, `HEAD`, `refs/**`, `packed-refs`, `MERGE_HEAD`…); linked worktrees watch their real git dir under the main repo. |
| Click + on a CHANGES row / − on a STAGED row / the section's + or − | The action runs, all action buttons disable meanwhile, the returned fresh status replaces the lists. |
| COMMIT with a message and a non-empty STAGED section | Label reads `COMMITTING…` with `aria-busy`; on success the message clears, the new short sha flashes in the status line, STAGED empties. |
| COMMIT when nothing is committable (index equals HEAD, no merge in progress) or the message is blank | Button disabled with hint "Stage changes to commit"; the agent additionally refuses with `GIT_NOTHING_TO_COMMIT`. A merge resolved to HEAD's tree remains committable. |
| `omitted.stagedOutsideRoot > 0` (repo entry rooted below the Git top level) | Warning row "N STAGED OUTSIDE THIS FOLDER WILL ALSO BE COMMITTED"; COMMIT asks for confirmation; COMMIT stays enabled because `status.committable` is Git's answer, not the listed rows. |
| `truncated` status (Git output over 8 MiB) | Degraded banner; STAGE ALL and COMMIT disabled. |
| Double-click a row | Diff opens in the shared workspace: index diff for STAGED rows, worktree diff for CHANGES rows, whole file as added for untracked; deleted rows do not open. |
| Discard Changes on a CHANGES row | Confirm modal (destructive); tracked files restore from the index, untracked files are deleted; never directories. |
| PULL / PUSH | `git pull --ff-only`; `git push` to the upstream, or `push -u <remote> HEAD` when exactly one remote exists; failures surface Git's message. |
| Git missing / not a repository / older installed agent | `GIT NOT FOUND` / `NOT A GIT REPOSITORY` / `AGENT UPDATE REQUIRED` empty states. |
| Action rejected without a `GIT_*` code (socket closed, transport timeout) | `COMMIT RESULT UNKNOWN — REFRESHING`: never "failed"; after reconnect the refetched status reports the outcome. |

## Cancellation and timeouts

Reads (status, diff) are cancellable and killed immediately on cancel. Mutations (stage, unstage,
discard, commit, push, pull) are deliberately non-cancellable: a killed `git commit` bypasses Git's
lockfile cleanup and leaves `.git/index.lock`, wedging the repo for every tool. Mutations are serialized
per repo, use a graceful stop on timeout (SIGTERM then wait on Unix), and report `GIT_ABORTED` naming a
leftover lock. Timeout ladder (per Git command < host→WSL request < UI request, outer tiers covering the
summed worst case of one request): status/diff 20/90/120 s; stage/unstage/discard 60/120/150 s; commit
120/240/300 s; push/pull 180/300/360 s. A UI timeout cancels nothing agent-side.

## Layout notes

- The zips panel was headerless; the rail adds a slim (~36px) header so the bundle explorer keeps its rows.
- With one bundle preset the preset selector is empty; when a second preset ships it stays inside the ZIPS
  body under the rail header, never as a second segmented rocker in the header.
- Rows use the stacked name-over-directory idiom and must fit the 300px workspace-mode rail.

## Acceptance

1. Counts and sections match `git status` for a WSL repo and a host repo, including `MM`, renames,
   untracked, and conflicts.
2. Stage/unstage single and all, commit, discard, diff, push, and pull work from the installed app.
3. An external commit in a terminal (main repo and linked worktree) refreshes the view without a manual
   refresh; a `cargo build` writing `target/` does not cause a refresh storm.
4. SOURCE survives a resize across the handset/standard band and an app restart.
5. No `.git/index.lock` remains after closing the app mid-commit.
6. ZIPS tree decorations (file badges, directory counts/colors) match `git status` for a WSL repo and a
   host repo; a file created inside an already-expanded directory appears with its badge without a manual
   refresh.
