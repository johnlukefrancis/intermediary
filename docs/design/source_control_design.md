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

- Show the active repo's changed files with counts, split into MERGE CONFLICTS, STAGED CHANGES, and CHANGES.
- Make a conflicted worktree impossible to miss: conflicts outrank ordinary changes in the rail cell, the
  column, and the diff view.
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
- A section-wide stage for MERGE CONFLICTS: conflicts are resolved per row by design, never in bulk, so
  stage-all/unstage-all never touch the conflict section.

## MVP

Right-rail segmented icon rocker (`DeckSectionSwitcher`: archive-box ZIPS cell, git-branch SOURCE cell) on
the existing right column; the Source Control column with status line (branch, ahead/behind, sha, refresh,
pull, push), warnings, commit box, three sections, and rows; diff kind in the shared workspace, with a
changed image opening a side-by-side image diff instead of `BINARY FILE`; handset rocker prepends a
stacked-documents FILES cell; `uiState.activeRail` persisted globally. Protocol: `sourceControlStatus`,
`sourceControlDiff`, `sourceControlImageDiff`, and one tagged `sourceControlAction` (stage, unstage,
discard, commit, push, pull); event `sourceControlChanged`.

## Naming

Intermediary already uses "staged" for drag-handoff staging (`StagedInfo`, `stagedByPath`, `.badge--staged`,
`stageFile`). Git-side identifiers therefore say `index` / `worktree` / `conflict` (`SourceControlEntry.area`)
and protocol names are `sourceControl*`; only user-facing copy says "STAGED CHANGES". `.badge--staged` is
never used for Git state.

## Behavior table

| Situation / input | Expected visible behavior |
| --- | --- |
| Repo tab opens (any mode) | Status is fetched for the active repo; the SOURCE tab shows the total change count in accent (hidden at zero). |
| SOURCE count | Distinct changed paths across the three lists plus `omitted.stagedOutsideRoot` — an `MM` file counts once, not per section. When the visible lists are empty but `stagedOutsideRoot > 0`, the body reads "NO CHANGES IN THIS FOLDER" and the warning row stays, rather than showing a hidden zero badge. |
| Worktree has unmerged paths (`status.conflicts` non-empty) | The SOURCE cell becomes an alert: error tone, a pulsing error halo, `!` plus the conflict count instead of the ordinary count, tooltip and accessible name "SOURCE · N merge conflicts". The column shows a first-row conflict banner ("N MERGE CONFLICTS — RESOLVE AND STAGE BEFORE COMMITTING"), the MERGE CONFLICTS section renders first in the error tone, and COMMIT is disabled with the hint "Resolve N merge conflicts to commit" even though `committable` is true (Git refuses commits with unmerged paths; the agent answers `GIT_UNMERGED_PATHS`). N counts listed conflicts plus `omitted.unmergedOutsideRoot` (conflicts above a subdirectory root, named in the banner as "N ABOVE THIS FOLDER" and not listed). Accessible name of the cell is "SOURCE N merge conflicts"; the tooltip adds the separator. Staging a conflict row marks it resolved, as before. |
| Double-click a MERGE CONFLICTS row | The diff opens with subtitle `MERGE CONFLICT` in the error tone, a pinned notice counting unresolved `<<<<<<<` blocks ("markers resolved · stage the file" when none remain, "diff truncated · at least N" on a cut patch, "binary file · keep one version" for binaries), and every conflict marker line (`<<<<<<<`, `|||||||`, `=======`, `>>>>>>>`) highlighted in the warning tone inside Git's combined diff. |
| SOURCE rail selected in the standard deck or workspace mode | The right column shows the Source Control column; ZIPS is one click away; the choice persists across restarts and across the 980/860 resize band. |
| Handset deck | The icon rocker shows FILES / ZIPS / SOURCE cells (stacked-documents / archive-box / git-branch glyphs); picking ZIPS/SOURCE also sets the persisted rail. With a file or diff open, handset shows the workspace only (close returns to the deck section). |
| ZIPS rail with a changed working tree | File rows whose path is in the status carry a tinted name and a `[letter]` badge (the same `CHANGE_BADGES` palette as SOURCE rows); directory rows carry a tinted name and a count of distinct changed paths beneath them, colored by the worst change beneath; deleted files count toward their directory without a row of their own; expanded directories re-list in place on `sourceControlChanged` so a newly created file appears with its badge. |
| File edited, created, deleted, or renamed in the working tree (not under node_modules/target or the repo's ignore globs) | `sourceControlChanged` arrives within ~250 ms (coalesced); the column refetches once after a 300 ms trailing debounce. |
| External `git add` / `git commit` / branch switch in a terminal, main repo or linked worktree | Same as above via `.git` metadata watches (`index`, `HEAD`, `refs/**`, `packed-refs`, `MERGE_HEAD`…); linked worktrees watch their real git dir under the main repo. |
| Click + on a CHANGES row / − on a STAGED row / the section's + or − | The action runs, all action buttons disable meanwhile, the returned fresh status replaces the lists. |
| Section's + (stage-all) or − (unstage-all) | The agent enumerates the section from a status it reads fresh at action time (stage-all = worktree entries incl. untracked, never conflicts; unstage-all = index entries) and passes those paths explicitly — never a `.` pathspec — so conflict rows in MERGE CONFLICTS are never touched by a section action. |
| COMMIT with a message and a non-empty STAGED section | Label reads `COMMITTING…` with `aria-busy`; on success the message clears, the new short sha flashes in the status line, STAGED empties. |
| COMMIT while MERGE CONFLICTS has any unresolved row | Button disabled; if sent anyway the agent refuses with `GIT_UNMERGED_PATHS` — unresolved conflicts always block commit even when the index already differs from HEAD. |
| COMMIT when nothing is committable (index equals HEAD, no merge in progress) or the message is blank | Button disabled with hint "Stage changes to commit"; the agent additionally refuses with `GIT_NOTHING_TO_COMMIT`. A merge resolved to HEAD's tree remains committable. |
| COMMIT sent with the `expectedSnapshotId` from the last status the user reviewed, but the repository moved since — another tool staged or committed, a branch switch, or a merge/cherry-pick/revert started or ended | Agent refuses with `SOURCE_CONTROL_STATE_CHANGED` "the repository changed since it was reviewed: branch, HEAD, index, or merge state"; the commit box shows "STATE CHANGED — REVIEW AGAIN", the message is kept, and status auto-refreshes so the user reviews the new state before retrying. One identity (`snapshotId`) covers the branch the commit would move, where it points, the tree it would record, and the sequencer state it would conclude — a same-HEAD, same-tree commit on a different branch, or the same index with a swapped `MERGE_HEAD`, is refused like any other move. |
| COMMIT while the rendered status carries `snapshotId: ""` (a torn index read, or state the agent could not read) | Button disabled with the hint "Review did not capture a stable snapshot; refresh before committing"; if sent anyway the agent refuses with `SOURCE_CONTROL_STATE_CHANGED` "the review did not capture a stable snapshot" rather than comparing two empties as equal. |
| COMMIT modal is open (or COMMIT was just clicked) while a background status refresh arrives | The request froze `{ message, expectedSnapshotId }` from the status rendered the moment COMMIT was clicked (plus the outside-root count the modal shows); Confirm sends exactly that frozen object — a later refresh never rebinds it, and the agent still refuses if its own re-read under the lock shows a different snapshot. |
| A pre-commit hook re-stages only paths already in the reviewed tree (e.g. lint-staged formatting the files being committed) | The commit stands; the result carries `hookChangedPaths` naming those paths, shown as an informational notice with no extra confirmation. |
| A pre-commit hook (or anything else) stages a path the reviewed tree did not touch | The commit stands — Git published it, hooks ran, and a ref rewind would be a second unreviewed mutation, not a cancellation. The result carries `hookAddedPaths` naming those paths, and the column shows a warning-tone notice "COMMIT HOOK ADDED UNREVIEWED FILES" that names them and says the last commit can be undone with a soft reset (plain words, no command). |
| The index changes between the two reads `capture_status` takes to build one status object | The read retries (up to 3 times); still torn after that → `indexTreeSha` and `snapshotId` both come back as empty strings (no stable identity), COMMIT is disabled with the no-snapshot hint, and a commit sent against it anyway is refused until the next status read. |
| Commit times out but the agent's HEAD re-check shows it landed | Reported to the UI as applied, with the new short sha (a slow post-commit hook, not a failed commit); never retried automatically. |
| `omitted.stagedOutsideRoot > 0` (repo entry rooted below the Git top level) | Warning row "N STAGED OUTSIDE THIS FOLDER WILL ALSO BE COMMITTED"; COMMIT asks for confirmation; COMMIT stays enabled because `status.committable` is Git's answer, not the listed rows. |
| `truncated` status (Git output over 8 MiB) | Degraded banner; STAGE ALL and COMMIT disabled. |
| Double-click a row | Diff opens in the shared workspace: a text file opens the text diff (index diff for STAGED rows, worktree diff for CHANGES rows, whole file as added for untracked; deleted text rows do not open); an image file (`png`/`jpg`/`jpeg`/`webp`/`gif`/`bmp`/`avif`) opens the side-by-side image diff instead. |
| Double-click a staged image row | Image diff opens with panes `PREVIOUS · HEAD` / `CURRENT · INDEX`; a rename shows the HEAD side read from `originalPath`. |
| Double-click an unstaged (CHANGES) image row | Image diff opens with panes `PREVIOUS · INDEX` / `CURRENT · WORKTREE`. |
| Double-click a new/untracked image row | Nothing to compare, so a single full-width pane shows the image headed `NEW · WORKTREE` (`NEW · INDEX` when staged). |
| Double-click a deleted image row | Unlike deleted text rows, deleted image rows open: a single full-width pane shows the last-known image headed `DELETED · INDEX` (`DELETED · HEAD` for a staged deletion). |
| Double-click a conflicted image row | Image diff opens with panes `OURS` / `THEIRS` and the `MERGE CONFLICT` subtitle in the error tone; a stage missing from the conflict (delete/modify) shows its labelled empty slot. |
| An image diff side exceeds the 12 MiB per-side bound | That pane shows `TOO LARGE TO PREVIEW` with the reported size instead of the image; the other side still renders normally if it is within bound. |
| A changed file is a non-image binary | Diff stays on the existing `BINARY FILE` state; no image route. |
| Image diff on handset | The two panes stack vertically instead of sitting side by side. |
| Discard Changes on a copied row | Confirm modal names the destination path only (`[path]`) and says it will be deleted; the copy's source file is never touched, so an unrelated edit already sitting in that source file survives. |
| Discard Changes on a renamed row | Confirm modal names both endpoints (`[originalPath, path]`) and says what happens to each — the current path is discarded, the original path is restored. |
| Discard Changes on a CHANGES row where the on-disk file changed since the last status read | The stale target's stamp mismatches; the agent refuses the whole action with `SOURCE_CONTROL_STATE_CHANGED` naming that path, and the row's confirm re-reads status rather than assuming the earlier list is still accurate. |
| Discard Changes on an existing tracked or untracked file | The target is atomically renamed into its own quarantine directory (`<git_dir>/intermediary-discard/<opId>-<targetIndex>/`, one per target of the action) as `claimed` first, and bytes + `mtimeMs` + `mtimeNanos` are verified on the quarantined file; a mismatch renames it back and refuses with `SOURCE_CONTROL_STATE_CHANGED` (`notApplied` when no earlier target in the batch already succeeded). On a match a `verified` marker (`<path>\n<plan>\n`) is written before anything is destroyed, then `git restore --worktree` (tracked) or the delete (untracked) runs, and the claim is renamed `retained` — the bytes stay on disk until the next agent start, so a discard the user regrets can still be recovered by hand. Anything failing after that marker is written (the Git command, or the retention itself) renames the claim to `unrestored` instead, reports `effect: unknown`, and names that path: the worktree path is already empty, so those bytes are the only copy the user has. |
| Discard Changes on a row that was already missing at the last status read (target carries `expectedMissing: true`) | If the path now exists, refused with `SOURCE_CONTROL_STATE_CHANGED` "<path> was created after it was reviewed" — a newer file appeared and is preserved rather than restored over; otherwise `git restore --worktree` runs to bring the tracked file back. |
| Discard Changes on a target the review asserted nothing about (a rename origin the UI showed as already gone) | Absent is what that target should be, so it is restored (no claim to make). If something is there now — a directory, a symlink, a file behind an unreadable parent — nothing can prove a discard would destroy what the user looked at, so it is refused with `SOURCE_CONTROL_STATE_CHANGED` "cannot identify <path> before discarding it (not a regular file the review could stamp)". |
| Discard Changes where the worktree and its repository live on different volumes (a linked worktree on another drive) | The claim rename can never move the file, so the discard is refused with `SOURCE_CONTROL_UNSUPPORTED_LAYOUT` ("the worktree and its repository live on different volumes"), `effect: notApplied`; the column heads it "UNSUPPORTED REPOSITORY LAYOUT". It is a layout to change, not a state that settles. |
| A claimed file cannot be put back (verification mismatch rolls back, or a later step fails) because something now occupies the original path, or the filesystem has no rename that refuses to replace | The put-back never overwrites what is there: the claim is renamed to `unrestored` inside its quarantine directory and held, the failure names that path, and `effect: unknown`. On WSL's 9p mount of a Windows drive (`/mnt/c`) no no-replace rename exists at all, so a rollback there always holds the bytes rather than returning them — serve Windows drives through the host agent instead. |
| Discard Changes on multiple targets, one restores successfully and a later one fails | `effect: unknown`, with the message listing what was already restored — never `notApplied` once any target's effect boundary was crossed. |
| Agent starts and finds quarantine directories under `<git_dir>/intermediary-discard/` (a previous session's retained bytes, or a crash) | One bounded sweep per git dir on the first status read. A directory whose `<opId>` names a discard running right now — a sibling configured root over the same git dir can start one at any moment — is left alone entirely. Otherwise: a directory with a `verified` marker and no `unrestored` file is removed and its marker's path and plan logged — that finishes exactly the destruction the discard was authorized to do, and releases the previous session's retained bytes. A directory holding `unrestored` bytes, or one with no `verified` marker at all (a process that died between claiming and verifying), is kept and logged; nothing unproven is ever deleted. One directory that cannot be read or removed is logged with its path and does not stop the rest, and the sweep logs how many it removed, held, and failed on. |
| Discard Changes generally | Confirm modal (destructive) lists every target path and what happens to it (restored from the index, or deleted); tracked files restore, untracked files are deleted; never directories. |
| Copy row action (stage/unstage/discard) | Acts on the destination path only; the copy's source is never staged, unstaged, or discarded by that action. |
| Cross-root rename (one endpoint inside the configured root, one outside) | The row shows a warning; the count of such rows adds to `omitted.stagedOutsideRoot` so COMMIT's confirmation names how many outside-root changes ride along. |
| Discard Changes on a CHANGES row | Confirm modal (destructive); tracked files restore from the index, untracked files are deleted; never directories. |
| PULL / PUSH | `git pull --ff-only`; `git push` to the upstream, or `push -u <remote> HEAD` when exactly one remote exists; failures surface Git's message. |
| Git missing / not a repository / older installed agent | `GIT NOT FOUND` / `NOT A GIT REPOSITORY` / `AGENT UPDATE REQUIRED` empty states. |
| Action rejected without a `GIT_*` code (socket closed, transport timeout, `effect: unknown`) | `COMMIT RESULT UNKNOWN — REFRESHING`: never "failed"; the UI reconciles by refetching status with backoff until `mutationInProgress` is false, then reports the outcome from that status. |
| App close/restart while a commit (or any mutation) is still running | The agent drains: it stops admitting new mutations and waits for the in-flight one to finish normally. `drained: false` never triggers an exit — the agent keeps waiting up to a 450 s emergency bound, then terminates its own owned process tree, logs `unknown` with the residue, and exits; nothing is killed mid-command inside that bound. |
| Host's WSL backend goes unavailable mid-shutdown while a mutation was forwarded to it | Counted as drained only when the host has no outstanding forwarded mutation request id to that backend; otherwise the host keeps waiting, up to the same emergency bound. |
| Supervisor stops/restarts the agent during a long mutation | Labels the stop drained only on an explicit `drained: true` ack; on `drained: false` or no ack it waits for the process up to 480 s before its kill path runs; the process disappearing without an ack is logged `unknown`, never `drained`, and WSL distro termination is skipped while finality is unknown. |
| Forced stop of the Windows Git process tree | Git's children run inside a Job Object with no kill-on-close limit: the tree is terminated on forced stop, drain expiry, and shutdown finalization, taking hooks, credential helpers, and `git-remote-*` descendants with it instead of detaching their pipes. Helpers that close their pipes outlive Git as on Unix. |
| Forced stop of the host agent itself (emergency kill after a drain never completed) | On Windows the supervisor spawns the host agent into a supervisor-owned Job Object with no kill-on-close limit and terminates that job on the emergency path, so Git, hooks, and credential helpers under a hung agent go with it instead of being orphaned. An agent the app adopted rather than spawned has no job: it is stopped by binary identity and its descendants are not owned, which the log says outright ("no tree owner (adopted agent)"). |
| A Git mutation (stage, unstage, discard, commit, push, pull) cannot be given a process-tree owner on Windows | Refused before the process spawns, `effect: notApplied`; if the job could only be attached after the spawn and that attach failed, the child is killed and the outcome is reported `unknown`. Reads still run without a job (a detached reader is honest about what it owns) and log that once. |
| A tracked file changes under a folder the watcher's structural matcher would otherwise ignore (e.g. a tracked file living under `target/`) | The event still emits — the watcher's tracked-path set (from `git ls-files`) overrides the structural exclude for tracked paths, so SOURCE stays current; only untracked noise under those folders stays suppressed. |

## Cancellation and timeouts

WSL-routed reads (status, diff) carry a cancel token and are killed immediately on cancel. Host-rooted
reads run in-process and are bounded by their Git timeout only — there is no UI cancel path to serve one,
so cancelling a host read is not offered. Mutations (stage, unstage, discard, commit, push, pull) are
deliberately non-cancellable from the UI: a killed `git commit` bypasses Git's lockfile cleanup and can
leave `.git/index.lock`, wedging the repo for every tool. Mutations are serialized per repo (keyed by the
physical git dir), use a graceful stop on timeout (SIGTERM then wait on Unix; `TerminateProcess` on
Windows), and report `GIT_ABORTED` naming any leftover lock the agent can see.

Timeout ladder (agent per Git command < host→WSL request < UI request; a request may run several Git
commands, so the outer tiers cover the summed worst case, not one command's raw budget):

- Agent per Git command, unchanged: read 20 s, index (stage/unstage/discard) 60 s, commit 120 s, remote
  (push/pull) 180 s.
- Host→WSL: status/diff 120 s, stage/unstage 280 s, discard 340 s, commit 380 s, push/pull 420 s.
- UI: status/diff 150 s, stage/unstage 310 s, discard 370 s, commit 410 s, push/pull 450 s.

Discard is its own timeout class at every tier (it used to be folded into stage/unstage). A UI timeout
cancels nothing agent-side; the hook enters `reconciling` instead (see below).

On app close/restart or SIGTERM, the agent stops admitting new mutations and waits for every in-flight
mutation to finish normally. `drained: false` never exits it: the agent keeps waiting up to a 450 s
emergency bound (longer than any bounded mutation: status 100 s + remote 180 s + status 100 s + margin),
then terminates its owned process tree, logs `unknown` with the residue, and exits. The shutdown response
is sent when the drain completes (`drained: true`) or at the emergency bound (`drained: false`,
`activeMutations`). The host agent forwards `shutdown` to the WSL backend first; `WSL_BACKEND_UNAVAILABLE`
counts as drained only when the host has no outstanding forwarded mutation request id to that backend,
otherwise the host waits the same 450 s bound. The Tauri supervisor sends `shutdown` and labels the stop
drained only on an explicit `drained: true` ack; on `drained: false` or no ack it waits for the process up
to 480 s before its existing kill path runs as the emergency bound, logging `unknown` (never `drained`) if
the process disappears without an ack; WSL distro termination is skipped while finality is unknown. On
Windows, children of the Git runner are placed in a Job Object with no kill-on-close limit, so a forced
stop, an expired post-exit drain, and shutdown finalization each take the whole descendant tree — hooks,
credential helpers, `git-remote-*` — with it instead of leaving post-exit pipe holders detached, while a
helper that closed its pipes outlives Git as on Unix.

After an `unknown` outcome (timeout, forced stop, transport failure) the hook enters `reconciling` and
refetches status with backoff (1 s, 2 s, 4 s, capped 8 s, up to the action's UI budget) until a status with
`mutationInProgress === false` arrives; only then does the view return to `ready`, with the outcome
reported from before/after HEAD (commit) or from the refreshed lists.

## Layout notes

- The zips panel was headerless; the rail adds a slim (~36px) header so the bundle explorer keeps its rows.
- With one bundle preset the preset selector is empty; when a second preset ships it stays inside the ZIPS
  body under the rail header, never as a second segmented rocker in the header.
- Rows use the stacked name-over-directory idiom and must fit the 300px workspace-mode rail.

## Accepted boundaries

These were raised by the third adversarial review (`docs/reports/source_control_hardening_review_20260903.md`)
and are recorded as decisions, not as open defects. Each names what the product deliberately does not do.

- **No private commit transaction (hooks inside it, CAS publish).** It would reimplement `git commit` —
  hook environment, `commit.cleanup`, gpgsign, sequencer cleanup, reflog, rerere — and isolate nothing
  real, because hooks mutate the worktree, not only the index. The snapshot binds the commit's input and
  Git owns publication.
- **A hook may rewrite reviewed paths or edit the message (`prepare-commit-msg`).** Hooks are
  repository-trusted code that every Git client runs. What a hook did is reported (`hookChangedPaths`,
  `hookAddedPaths`), never pre-empted, and a commit Git published is never rewound.
- **No content digest on status.** A changed file's identity is its size plus its nanosecond mtime.
  Two different contents sharing both are possible; hashing every changed file on every status read is
  not a cost this product pays, and the retained quarantine is the recovery path when it happens.
- **Restore stays Git's, so a TOCTOU window exists around it.** `git restore --worktree` keeps eol,
  filter, and LFS fidelity that a hand-rolled write would lose. The window is one process spawn per
  target, and the claim already took the bytes the user reviewed out of the worktree first.
- **The re-verify → ref-transaction window stays open.** Between the snapshot re-read under the mutation
  lock and Git's own ref transaction, an external `git checkout` can still move HEAD. Closing it needs the
  private-transaction route above.
- **A worktree on a different volume from its repository is refused, not supported.** No rename can claim
  a target across volumes; the layout is the thing to change (`SOURCE_CONTROL_UNSUPPORTED_LAYOUT`).
- **`rename_no_replace` is not available on every filesystem.** Probed 2026-09-03 on this machine: ext4
  returns `EEXIST` when the destination exists and succeeds otherwise; WSL2's 9p mount of a Windows drive
  (`/mnt/c`) returns `EINVAL` unconditionally, so on 9p a rollback can never put bytes back and holds them
  as `unrestored` instead. Windows drives are served through the host agent, where `MoveFileExW` works.
- **Adopted agents have no process-tree owner.** An `im_agent`/`im_host_agent` the app reclaimed rather
  than spawned is stopped by binary identity; its Git descendants are not owned by any job.
- **Ownership stops at the Tauri process.** It is the outermost owner this product has; beyond it,
  finality belongs to Git's own crash safety. On Linux/WSL the agent has three shutdown owners that take
  the same drain: SIGTERM, the authenticated `shutdown` command, and EOF on the stdin pipe the supervisor
  holds for exactly as long as it intends the agent to run (`crates/im_agent/src/server/stdin_eof.rs`).
  EOF is the one that still arrives when the Tauri process dies without a chance to send anything; a
  WSL agent this supervisor *adopted* rather than spawned has no pipe and therefore only the first two.
  Behind them, the supervisor's own WSL emergency route waits that drain out (480 s, the same envelope
  the host stop uses) and only then terminates the agent's descendant process groups before the agent
  itself — so a hook holding `.git/index.lock` is never orphaned by the stop. Distro termination stays
  conditional (skipped while host finality is unknown or an interactive WSL session is open) and is
  therefore never the thing relied on to sweep up.
- **A descendant that starts its own session escapes the sweep.** The emergency route walks the agent's
  descendants from one `ps` snapshot and signals their process groups; a hook that called `setsid` has
  by definition left the agent's tree and is not reached. Accepted: `setsid` in a hook is a deliberate
  detachment, and reaching it would mean killing by heuristic rather than by ownership.

## Acceptance

1. Counts and sections match `git status` for a WSL repo and a host repo, including `MM`, renames,
   untracked, and conflicts.
2. Stage/unstage single and all, commit, discard, diff, push, and pull work from the installed app.
3. An external commit in a terminal (main repo and linked worktree) refreshes the view without a manual
   refresh; a `cargo build` writing `target/` does not cause a refresh storm.
4. SOURCE survives a resize across the handset/standard band and an app restart.
5. Discarding a copied row deletes only the destination and leaves an unrelated edit already sitting in
   the copy's source file intact.
6. A commit sent against a stale `expectedSnapshotId` (branch, HEAD, index tree, or merge state moved
   since the last review) is refused with `SOURCE_CONTROL_STATE_CHANGED`, never silently absorbs the
   newer state; a status with no stable snapshot (`snapshotId: ""`) disables COMMIT rather than
   committing unchecked.
7. A discard sent against a stale on-disk stamp is refused with `SOURCE_CONTROL_STATE_CHANGED`, naming the
   path, rather than destroying newer content than the user confirmed.
8. Stage-all/unstage-all never touch MERGE CONFLICTS rows; conflicts stay unmerged until resolved per row.
9. Closing the app mid-commit drains the mutation: the agent keeps waiting on `drained: false` rather than
   exiting, up to the 450 s emergency bound, and never kills Git mid-command inside that bound.
10. A tracked file that lives under `target/` or another structurally-ignored folder still refreshes
    SOURCE when it changes, while untracked noise under those folders keeps producing no refresh.
11. A supplied pre-commit hook scenario lands and is reported for what it is: a hook that rewrote
    reviewed paths reports them in `hookChangedPaths` (informational), and a hook that added paths the
    reviewed tree did not touch reports them in `hookAddedPaths` with the warning-tone notice — never a
    silently-widened commit, and never a ref rewind after publication.
12. Missing-then-recreated, same-length/same-mtime, and multi-path partial-failure discard scenarios all
    preserve the newer bytes on disk and never report `notApplied` once an effect has landed.
13. A host or WSL close/restart during an operation longer than 60 s leaves that operation owned to a
    terminal state, with no surviving `.git/index.lock` and no surviving Windows hook/helper process after
    the emergency stop.
14. ZIPS tree decorations (file badges, directory counts/colors) match `git status` for a WSL repo and a
    host repo; a file created inside an already-expanded directory appears with its badge without a manual
    refresh.
15. With unmerged paths, the SOURCE cell reads as an alert without opening the column, MERGE CONFLICTS is
    the first section, COMMIT is disabled with the resolve hint, and a conflict diff shows the MERGE
    CONFLICT subtitle, notice, and highlighted markers; all of it clears once every conflict is staged.
16. On a WSL repo and a host repo: a staged, unstaged, new/untracked, deleted, and conflicted image each open
    the correct side-by-side image diff with correct pane labels, dimensions and bytes in each footer, and
    transparency reading on the checkerboard; an oversized side shows `TOO LARGE TO PREVIEW`; a non-image
    binary still shows `BINARY FILE`; handset stacks the panes.
