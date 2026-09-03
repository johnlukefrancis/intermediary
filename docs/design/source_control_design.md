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

## Non-goals

- Branch management, merge/rebase tooling, history browsing, blame, or hunk-level staging.
- Running Git from the Tauri process or the webview.
- Cancelling a running commit/push from the UI (see Cancellation).
- A second layout breakpoint or a permanent third column.
- A section-wide stage for MERGE CHANGES: conflicts are resolved per row by design, never in bulk, so
  stage-all/unstage-all never touch the conflict section.

## MVP

Right-rail instrument switch `[ ZIPS ] [ SOURCE n ]` on the existing right column; the Source Control
column with status line (branch, ahead/behind, sha, refresh, pull, push), warnings, commit box, three
sections, and rows; diff kind in the shared workspace; handset switcher `[ FILES ] [ ZIPS ] [ SOURCE n ]`;
`uiState.activeRail` persisted globally. Protocol: `sourceControlStatus`, `sourceControlDiff`, and one
tagged `sourceControlAction` (stage, unstage, discard, commit, push, pull); event `sourceControlChanged`.

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
| SOURCE rail selected in the standard deck or workspace mode | The right column shows the Source Control column; ZIPS is one click away; the choice persists across restarts and across the 980/860 resize band. |
| Handset deck | Switcher is `[ FILES ] [ ZIPS ] [ SOURCE n ]`; picking ZIPS/SOURCE also sets the persisted rail. With a file or diff open, handset shows the workspace only (close returns to the deck section). |
| File edited, created, deleted, or renamed in the working tree (not under node_modules/target or the repo's ignore globs) | `sourceControlChanged` arrives within ~250 ms (coalesced); the column refetches once after a 300 ms trailing debounce. |
| External `git add` / `git commit` / branch switch in a terminal, main repo or linked worktree | Same as above via `.git` metadata watches (`index`, `HEAD`, `refs/**`, `packed-refs`, `MERGE_HEAD`…); linked worktrees watch their real git dir under the main repo. |
| Click + on a CHANGES row / − on a STAGED row / the section's + or − | The action runs, all action buttons disable meanwhile, the returned fresh status replaces the lists. |
| Section's + (stage-all) or − (unstage-all) | The agent enumerates the section from a status it reads fresh at action time (stage-all = worktree entries incl. untracked, never conflicts; unstage-all = index entries) and passes those paths explicitly — never a `.` pathspec — so conflict rows in MERGE CHANGES are never touched by a section action. |
| COMMIT with a message and a non-empty STAGED section | Label reads `COMMITTING…` with `aria-busy`; on success the message clears, the new short sha flashes in the status line, STAGED empties. |
| COMMIT while MERGE CHANGES has any unresolved row | Button disabled; if sent anyway the agent refuses with `GIT_UNMERGED_PATHS` — unresolved conflicts always block commit even when the index already differs from HEAD. |
| COMMIT when nothing is committable (index equals HEAD, no merge in progress) or the message is blank | Button disabled with hint "Stage changes to commit"; the agent additionally refuses with `GIT_NOTHING_TO_COMMIT`. A merge resolved to HEAD's tree remains committable. |
| COMMIT sent with the `expectedIndexTreeSha` from the last status the user reviewed, but the index moved since (another tool staged/committed) | Agent refuses with `SOURCE_CONTROL_STATE_CHANGED`; the commit box shows "STATE CHANGED — REVIEW AGAIN", the message is kept, and status auto-refreshes so the user reviews the new state before retrying. |
| COMMIT modal is open (or COMMIT was just clicked) while a background status refresh arrives | The modal froze `{ expectedIndexTreeSha, expectedHeadSha, message, acknowledgedOutsideRoot }` from the status rendered the moment it opened; Confirm sends exactly that frozen object — a later refresh never rebinds it, and the agent still refuses if its own re-read shows the index moved. |
| A pre-commit hook re-stages only paths already in the reviewed index list (e.g. lint-staged formatting the files being committed) | The commit is bound to both `expectedIndexTreeSha` and `expectedHeadSha`; the hook's changes are accepted and the result carries `hookChangedPaths` naming them, with no extra confirmation. |
| A pre-commit hook (or anything else) stages a path outside the reviewed index list | The commit lands and is then retracted (`git update-ref` moves the branch back to the previous HEAD, CAS-guarded; detached HEAD retargets `HEAD` itself) before it is visible; refused with `SOURCE_CONTROL_STATE_CHANGED` "a commit hook staged unreviewed paths: …", `effect: notApplied`. If the retraction itself fails, `effect: unknown` names the sha so the user knows exactly what state HEAD is in. |
| The index changes between the two reads `capture_status` takes to build one status object | The read retries (up to 3 times); still torn after that → `indexTreeSha` comes back as an empty string (no stable identity), and a commit against it is refused until the next status read. |
| Commit times out but the agent's HEAD re-check shows it landed | Reported to the UI as applied, with the new short sha (a slow post-commit hook, not a failed commit); never retried automatically. |
| `omitted.stagedOutsideRoot > 0` (repo entry rooted below the Git top level) | Warning row "N STAGED OUTSIDE THIS FOLDER WILL ALSO BE COMMITTED"; COMMIT asks for confirmation; COMMIT stays enabled because `status.committable` is Git's answer, not the listed rows. |
| `truncated` status (Git output over 8 MiB) | Degraded banner; STAGE ALL and COMMIT disabled. |
| Double-click a row | Diff opens in the shared workspace: index diff for STAGED rows, worktree diff for CHANGES rows, whole file as added for untracked; deleted rows do not open (no Open Diff either, from the context menu). |
| Discard Changes on a copied row | Confirm modal names the destination path only (`[path]`) and says it will be deleted; the copy's source file is never touched, so an unrelated edit already sitting in that source file survives. |
| Discard Changes on a renamed row | Confirm modal names both endpoints (`[originalPath, path]`) and says what happens to each — the current path is discarded, the original path is restored. |
| Discard Changes on a CHANGES row where the on-disk file changed since the last status read | The stale target's stamp mismatches; the agent refuses the whole action with `SOURCE_CONTROL_STATE_CHANGED` naming that path, and the row's confirm re-reads status rather than assuming the earlier list is still accurate. |
| Discard Changes on an existing tracked or untracked file | The target is atomically renamed into an operation-owned quarantine directory (`.git/intermediary-discard/<opId>/`) first, and bytes + `mtimeMs` + `mtimeNanos` are verified on the quarantined file; a mismatch renames it back and refuses with `SOURCE_CONTROL_STATE_CHANGED` (`notApplied` when no earlier target in the batch already succeeded); only then does `git restore --worktree` (tracked) or the delete (untracked) run. |
| Discard Changes on a row that was already missing at the last status read (target carries `expectedMissing: true`) | If the path now exists, refused with `SOURCE_CONTROL_STATE_CHANGED` — a newer file appeared and is preserved rather than restored over; otherwise `git restore --worktree` runs to bring the tracked file back. |
| Discard Changes on multiple targets, one restores successfully and a later one fails | `effect: unknown`, with the message listing what was already restored — never `notApplied` once any target's effect boundary was crossed. |
| Agent starts after a crash left quarantined discard targets behind | The leftovers under `.git/intermediary-discard/` are removed by a bounded startup sweep and logged; nothing is left wedging the repo. |
| Discard Changes generally | Confirm modal (destructive) lists every target path and what happens to it (restored from the index, or deleted); tracked files restore, untracked files are deleted; never directories. |
| Copy row action (stage/unstage/discard) | Acts on the destination path only; the copy's source is never staged, unstaged, or discarded by that action. |
| Cross-root rename (one endpoint inside the configured root, one outside) | The row shows a warning; the count of such rows adds to `omitted.stagedOutsideRoot` so COMMIT's confirmation names how many outside-root changes ride along. |
| PULL / PUSH | `git pull --ff-only`; `git push` to the upstream, or `push -u <remote> HEAD` when exactly one remote exists; failures surface Git's message. |
| Git missing / not a repository / older installed agent | `GIT NOT FOUND` / `NOT A GIT REPOSITORY` / `AGENT UPDATE REQUIRED` empty states. |
| Action rejected without a `GIT_*` code (socket closed, transport timeout, `effect: unknown`) | `COMMIT RESULT UNKNOWN — REFRESHING`: never "failed"; the UI reconciles by refetching status with backoff until `mutationInProgress` is false, then reports the outcome from that status. |
| App close/restart while a commit (or any mutation) is still running | The agent drains: it stops admitting new mutations and waits for the in-flight one to finish normally. `drained: false` never triggers an exit — the agent keeps waiting up to a 450 s emergency bound, then terminates its own owned process tree, logs `unknown` with the residue, and exits; nothing is killed mid-command inside that bound. |
| Host's WSL backend goes unavailable mid-shutdown while a mutation was forwarded to it | Counted as drained only when the host has no outstanding forwarded mutation request id to that backend; otherwise the host keeps waiting, up to the same emergency bound. |
| Supervisor stops/restarts the agent during a long mutation | Labels the stop drained only on an explicit `drained: true` ack; on `drained: false` or no ack it waits for the process up to 480 s before its kill path runs; the process disappearing without an ack is logged `unknown`, never `drained`, and WSL distro termination is skipped while finality is unknown. |
| Forced stop of the Windows Git process tree | Git's children run inside a Job Object with no kill-on-close limit: the tree is terminated on forced stop, drain expiry, and shutdown finalization, taking hooks, credential helpers, and `git-remote-*` descendants with it instead of detaching their pipes. Helpers that close their pipes outlive Git as on Unix. |
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
  body under the rail header, never as a second bracket tablist in the header.
- Rows use the stacked name-over-directory idiom and must fit the 300px workspace-mode rail.

## Acceptance

1. Counts and sections match `git status` for a WSL repo and a host repo, including `MM`, renames,
   untracked, and conflicts.
2. Stage/unstage single and all, commit, discard, diff, push, and pull work from the installed app.
3. An external commit in a terminal (main repo and linked worktree) refreshes the view without a manual
   refresh; a `cargo build` writing `target/` does not cause a refresh storm.
4. SOURCE survives a resize across the handset/standard band and an app restart.
5. Discarding a copied row deletes only the destination and leaves an unrelated edit already sitting in
   the copy's source file intact.
6. A commit sent against a stale `expectedIndexTreeSha` (index moved since the last review) is refused
   with `SOURCE_CONTROL_STATE_CHANGED`, never silently absorbs the newer state.
7. A discard sent against a stale on-disk stamp is refused with `SOURCE_CONTROL_STATE_CHANGED`, naming the
   path, rather than destroying newer content than the user confirmed.
8. Stage-all/unstage-all never touch MERGE CHANGES rows; conflicts stay unmerged until resolved per row.
9. Closing the app mid-commit drains the mutation: the agent keeps waiting on `drained: false` rather than
   exiting, up to the 450 s emergency bound, and never kills Git mid-command inside that bound.
10. A tracked file that lives under `target/` or another structurally-ignored folder still refreshes
    SOURCE when it changes, while untracked noise under those folders keeps producing no refresh.
11. A supplied pre-commit hook scenario either lands exactly the reviewed tree (accepting only paths the
    hook itself changed, reported via `hookChangedPaths`) or is retracted with `SOURCE_CONTROL_STATE_CHANGED`
    before HEAD is visibly moved — never a silently-widened commit.
12. Missing-then-recreated, same-length/same-mtime, and multi-path partial-failure discard scenarios all
    preserve the newer bytes on disk and never report `notApplied` once an effect has landed.
13. A host or WSL close/restart during an operation longer than 60 s leaves that operation owned to a
    terminal state, with no surviving `.git/index.lock` and no surviving Windows hook/helper process after
    the emergency stop.
