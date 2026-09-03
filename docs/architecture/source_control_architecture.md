# Source Control Architecture
Updated on: 2026-09-03
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

---

## Ownership

| Concern | Owner | Notes |
| --- | --- | --- |
| Running Git (bounded, cancellable, stderr-preserving) | `crates/im_bundle/src/git.rs` facade over `git_capture/{command,command_child,porcelain,path,prefix,diff}.rs` | One runner for bundle evidence and source control. `GitCommandFailure { kind, exit_code, stdout, stderr }`; `KillPolicy::{Immediate, Graceful}`. |
| Status projection, diff, actions | `crates/im_agent/src/source_control/` — root modules (`mod.rs`, `paths.rs`, `tests_support.rs`) plus the `status/`, `commit/`, `discard/`, `actions/`, `diff/`, `locks/`, and `runner/` owner folders | `source_control_status`, `source_control_diff`, `source_control_image_diff`, `run_source_control_action`; `SourceControlLocks` (per-repo mutation mutex) lives on `AgentRuntime`. |
| Mutation locks | `crates/im_agent/src/source_control/locks/mod.rs` | Keyed by the canonical absolute git dir (`git rev-parse --absolute-git-dir`, resolved once per repo root and cached), not by UI repo id — two configured roots over the same physical worktree serialize on the same lock; linked worktrees stay distinct. |
| Status identity | `crates/im_agent/src/source_control/status/` | `indexTreeSha` (`git ls-files --stage -z` through `im_bundle::git::index_tree_sha`, read before and after the porcelain read with up to 3 retries on a torn identity — still torn → empty string); `snapshotId`, the hex SHA-256 over NUL-joined `detached ? "detached" : branch`, `headSha ?? "unborn"`, `indexTreeSha`, and the raw contents of `MERGE_HEAD`, `CHERRY_PICK_HEAD`, `REVERT_HEAD` under the physical git dir (absent file = empty), which is the one identity a commit is bound to and is `""` when the index read was torn or any component was unreadable for a reason other than NotFound; `mutationInProgress` (`SourceControlLocks::is_busy`); and per-entry `worktreeStamp` (`{ bytes, mtimeMs, mtimeNanos }` from `fs::metadata`) — the identity a discard target is bound to. A worktree/conflict entry whose file is absent on disk carries `worktreeMissing: true`. |
| Discard quarantine | `crates/im_agent/src/source_control/discard/` (`<git_dir>/intermediary-discard/<opId>-<targetIndex>/`) | Each existing target is atomically claimed into its own quarantine directory — one per target of the action, because the phase files below are fixed names — before verification and restore/delete. Phase files say what the directory is: `claimed` (moved, nothing proven), `verified` (`<path>\n<plan>\n`, written after the stamp matched and before anything is destroyed), `retained` (the destruction ran; bytes kept until the next agent start), `unrestored` (bytes that could not be put back, including anything that failed after the `verified` marker was written). The first status read per git dir sweeps: a directory owned by a discard running right now → skipped; `verified` with no `unrestored` → removed and logged with its path and plan; anything else → held and logged. One directory's failure is logged and never aborts the sweep, which reports removed/held/failed counts. |
| No-replace rename | `crates/im_bundle/src/fs_atomic.rs` (`rename_no_replace`) | Linux `renameat2(RENAME_NOREPLACE)`, Windows `MoveFileExW` with no flags. `AlreadyExists` when the destination is occupied, `Unsupported` when the filesystem rejects the flag (`EINVAL`/`ENOSYS`/`ENOTSUP`); every other failure keeps its own kind. Used for discard rollback and put-back, which must never destroy anything. |
| Wire types | `crates/im_agent/src/protocol/{commands,responses}_source_control.rs` and `app/src/shared/protocol_source_control.ts` | Hand-kept in sync; serde `camelCase`, tagged unions (`kind`, `mode`). |
| WSL-agent dispatch | `crates/im_agent/src/server/connection/source_control_commands.rs` | Reads get a `SourceControlRead` cancel token; actions are `Passive`. |
| Host-agent dispatch | `crates/im_host_agent/src/server/dispatch.rs` (`dispatch_source_control`) + `runtime/local_host_source_control_backend.rs` | Backend resolved under a short read lock; host repos run Git in-process with no runtime lock held; WSL repos are forwarded. |
| Runner post-exit drain | `crates/im_bundle/src/git_capture/command_drain.rs` | After the direct Git child exits, pipe drain is bounded (2 s grace); on expiry the runner kills the child's process group on Unix and joins the readers, and on Windows terminates the job (below) instead of detaching them. Nothing is terminated inside the grace, so a helper that closed the pipes on purpose survives. |
| Windows process-tree ownership (per Git command) | `crates/im_bundle/src/process_job.rs` (`JobHandle`) + `git_capture/command_tree.rs` | Git's children are placed in a Job Object that carries no kill-on-close limit, mirroring the Unix process group: the tree is terminated on forced stop, drain expiry, and shutdown finalization, taking hooks, credential helpers, and `git-remote-*` descendants with it; helpers that close their pipes outlive Git as on Unix. A mutation that cannot be given a job is refused with `GitCommandFailureKind::NoProcessTreeOwner` before the spawn (`spawned: false`) or after killing the child when the attach failed (`spawned: true`); reads run unowned and log once. |
| Windows process-tree ownership (the host agent) | `src-tauri/src/lib/agent/process_control.rs` + `supervisor/process_kill.rs` | The supervisor creates a Job Object before spawning the host agent and assigns the child immediately after; the handle travels with the managed child and is terminated on the emergency kill path, so a hung agent's Git descendants go with it. A create/assign failure kills the child and returns the spawn error. An adopted agent (reclaimed, not spawned) has no job and is stopped by binary identity, logged `outcome=no_tree_owner`. |
| Shutdown drain | `crates/im_agent/src/source_control/locks/mod.rs` (`set_draining` / `wait_idle`) + host `dispatch.rs` + Tauri supervisor | `Shutdown` command in both agents; `drained: false` never exits an agent — it keeps waiting up to a 450 s emergency bound, then terminates its owned process tree and logs `unknown`; host agent forwards to the WSL backend first and treats `WSL_BACKEND_UNAVAILABLE` as drained only with no outstanding forwarded mutation; Tauri supervisor waits up to 480 s on an explicit `drained: true` ack before its existing kill path, never labelling process disappearance `drained`, and skips WSL distro termination while finality is unknown. |
| Refresh signal | `crates/im_agent/src/repos/source_control_watch/` (detector, coalescer, tracked-path set, git dirs) wired into `repo_watcher*.rs` | Emits `AgentEvent::SourceControlChanged { repoId }`, coalesced to one event per 250 ms with a trailing emit. |
| UI state | `app/src/hooks/source_control/use_source_control_state.ts`, `app/src/hooks/use_deck_section.ts`, `app/src/lib/source_control/conflict_count.ts` | One hook instance per active repo feeds the rail count, the conflict alert, and the column. |
| UI surface | `app/src/components/layout/{deck_section_switcher,deck_section_icons,repo_rail}.tsx`, `app/src/components/source_control/*`, `app/src/components/diff_workspace.tsx`, `app/src/components/image_diff_workspace.tsx`, `app/src/hooks/repo_workspace_diff_loaders.ts`, `app/src/hooks/use_image_blob_url.ts` | Rail switch (segmented icon rocker with a `DeckSectionAlert` form for conflicts), column, rows, commit box, diff kind of the shared workspace (`conflict` flag flags marker lines); `image_diff_workspace.tsx` renders the two-pane image diff from Blob URLs, `repo_workspace_diff_loaders.ts` owns the text and image diff loaders, `use_image_blob_url.ts` owns the Blob-URL lifecycle shared with the image previewer. |
| Tree decorations | `app/src/lib/source_control/{change_badges,tree_decorations}.ts`, `app/src/hooks/source_control/use_tree_decorations.tsx`, `app/src/hooks/bundles/use_directory_listings.ts` | Pure projection of the status onto the ZIPS explorer tree via a React context; the listing hook re-lists expanded directories on `sourceControlChanged`. |
| Persisted choice | `uiState.activeRail` in `app/src/shared/config/persisted_config.ts` | Global; defaulted, no migration. |

## Routing and lifecycle

1. The UI sends `sourceControlStatus` / `sourceControlDiff` / `sourceControlImageDiff` / `sourceControlAction`
   to the host agent over the existing token-authenticated socket on `127.0.0.1:3141` (no new port; ADR-010
   unchanged).
2. `server/dispatch.rs` intercepts the four commands before the `&mut self` catch-all. It resolves the
   repo backend under a read lock and drops it. Host-rooted repos: `LocalHostBackend::source_control_context`
   clones the root path and the lock registry, then `execute_host_source_control` runs Git with no runtime
   lock held. WSL-rooted repos: the command is forwarded verbatim to `im_agent` with the per-kind timeout
   from `wsl_backend_client.rs::timeout_for_command`.
3. In either agent, `im_agent::source_control` resolves the repo prefix (`git rev-parse --show-prefix`),
   runs the Git command on `spawn_blocking` from `common_git_args()` (always `--literal-pathspecs`), and
   maps failures to `AgentError` codes: `GIT_UNAVAILABLE`, `GIT_NOT_REPOSITORY`, `GIT_TIMEOUT`,
   `GIT_ABORTED`, `GIT_NOTHING_TO_COMMIT`, `GIT_UNMERGED_PATHS`, `GIT_COMMAND_FAILED` (Git's own text), `GIT_UNSUPPORTED_VERSION`,
   `INVALID_PATH`, `INVALID_COMMIT_MESSAGE`, `INVALID_REPO`, plus the source-control refusals
   `SOURCE_CONTROL_STATE_CHANGED` and `SOURCE_CONTROL_UNSUPPORTED_LAYOUT`.
4. Every action re-reads status after the Git command and returns it, so the view is never stale after
   its own mutation.
5. The watcher marks the coalescer dirty for `.git` metadata writes (`index`, `HEAD`, `ORIG_HEAD`,
   `MERGE_HEAD`, `FETCH_HEAD`, `packed-refs`, `refs/**`, and their `.lock` files) and for working-tree
   events outside the detector's own structural matcher (`node_modules`, `target`, plus the repo's
   `ignoreGlobs`); linked worktrees get a second watch on their real git dir and the common `refs`.
   The UI debounces the event 300 ms, drops refetches that predate its own last mutation, and also
   refetches on window focus, hello, and rehydrate. There is no interval polling.
6. Shutdown routes UI/supervisor → host agent → WSL agent: the Tauri supervisor (stop/restart/exit, or
   SIGTERM/ctrl-c) connects to the host agent and sends `shutdown`; the host agent forwards `shutdown` to
   the WSL backend first, then drains itself. Neither agent exits on `drained: false` — each keeps waiting
   up to a 450 s emergency bound, then terminates its owned process tree and logs `unknown` with the
   residue. The supervisor labels a stop drained only on an explicit `drained: true` ack; on `drained:
   false` or no ack it waits for the process up to 480 s before falling back to its existing kill path,
   and skips WSL distro termination while finality is unknown.

## Invariants

- The UI never runs Git; every read and mutation is an agent request routed by repoId.
- The ZIPS tree never runs Git; decorations are a pure projection of the status snapshot, directory counts
  are distinct paths, and deleted files count toward their directory without a row of their own.
- Mutations on one repo are serialized (`SourceControlLocks`, keyed by the physical git dir); the per-repo
  lock is cloned out and awaited with no runtime guard held.
- Mutations are never killed mid-command by cancellation. On timeout they are stopped gracefully
  (SIGTERM then wait on Unix; TerminateProcess on Windows) and report `GIT_ABORTED`, naming any leftover
  `.git/index.lock` it can see. WSL-routed reads carry a `SourceControlRead` cancel token and are killed
  immediately on cancel. Host in-process reads are bounded by their Git timeouts only and are **not**
  cancellable — no UI cancel path exists to serve one.
- No source-control command reaches `HostRuntime::dispatch_command` (`&mut self`); that arm is a release
  guard that returns an internal error.
- `UiCommand::repo_id()` is exhaustive; a new repo-scoped command that is not listed fails to compile.
- Section-wide actions (`{ scope: { mode: "all" } }`) enumerate the section from a fresh status taken at
  action time — stage-all = worktree entries including untracked, never conflicts; unstage-all = index
  entries — and pass the paths explicitly via `--pathspec-from-file=- --pathspec-file-nul`; pathspec `.` is
  never used, so a section action can never reach outside the section it names. A section action therefore
  reaches only the paths the section listed: a path counted in `omitted.unrepresentablePath` is never
  listed, so it lies outside every section action and can only be staged from a terminal. An explicit
  `{ mode: "paths", paths: [] }` is rejected with `INVALID_PATH` before any process spawns. `git commit`
  always commits the whole index and the UI surfaces `omitted.stagedOutsideRoot` with a confirm.
- Copy and rename rows carry both endpoints, never a flattened path bag: a copy-row action (stage, discard)
  acts on the destination path only and never touches the source; a rename-row action acts on both
  `originalPath` and `path`. A cross-root rename (one endpoint inside the configured root, one outside) is
  projected as `renamed`/`copied` without `originalPath` plus `omitted.stagedOutsideRoot += 1` when the
  outside endpoint is the deletion side (the deletion travels with the commit; a copy leaves the outside
  source alone), or as `{ path: original, area: index, change: deleted }` plus the same counter when the
  outside endpoint is the index-rename origin.
- Committability is Git's answer, not the projected list: `status.committable` is true when no unmerged
  record exists anywhere in the porcelain output **and** (`git diff --cached --quiet` reports a difference
  from HEAD **or** `MERGE_HEAD` exists) — so unresolved conflicts always block commit even when the index
  differs from HEAD, while a merge resolved to HEAD's tree or a commit whose staged paths sit above the
  configured root both stay committable.
- Unmerged paths outrank every other state in the UI: `conflictCount` (listed conflicts plus
  `omitted.unmergedOutsideRoot`, unmerged paths above a subdirectory root that cannot be listed) from the
  one status hook drives the rail alert, the first-row banner, and the COMMIT gate; the MERGE CONFLICTS
  section lists only in-root conflicts. The agent refuses `commit` with `GIT_UNMERGED_PATHS` on the
  repository-wide unmerged flag — the same records `conflictCount` sums — so the UI gate and the agent's
  own precondition agree, and `committable` is already false for exactly that state.
- Mutations are bound to the snapshot the user reviewed, not to whatever Git state exists when they run.
  The binding is a precondition checked under the mutation lock; what a mutation actually landed is then
  reported, never undone:
  - `commit { message, expectedSnapshotId }` is refused under the lock in this order: blank message →
    `INVALID_COMMIT_MESSAGE`; unmerged records → `GIT_UNMERGED_PATHS` (an unmerged index has no candidate
    tree, so its snapshot reads empty and would otherwise report the wrong remedy); `expectedSnapshotId`
    empty → `SOURCE_CONTROL_STATE_CHANGED` "the review did not capture a stable snapshot" (two empties must
    never compare equal); a different `snapshotId` from the fresh capture → `SOURCE_CONTROL_STATE_CHANGED`
    "the repository changed since it was reviewed: branch, HEAD, index, or merge state"; nothing
    committable → `GIT_NOTHING_TO_COMMIT`. All are `effect: notApplied`. One identity covers the ref the
    commit would move, where it points, the tree it would record, and the sequencer state it would
    conclude, so a same-tree commit on a different branch and a swapped `MERGE_HEAD` are refused like any
    other move.
  - `git commit` then runs (hooks included) and whatever Git publishes stands. A ref rewind after
    publication would be a second unreviewed mutation, not a cancellation, so there is none. When the
    landed tree differs from the reviewed `indexTreeSha`, `finalize_commit` reports the difference:
    `reviewedPaths = diff-tree -r --name-only <reviewedHead | empty tree> <reviewedTree>`,
    `changed = diff-tree -r --name-only <reviewedTree> HEAD^{tree}`, then
    `hookChangedPaths = changed ∩ reviewedPaths` (content a hook rewrote in paths the user saw) and
    `hookAddedPaths = changed \ reviewedPaths` (paths nobody reviewed). Both are optional on the wire and
    absent when empty; reviewed paths come from the two immutable objects, never from the live index,
    which by then equals HEAD. A read that fails here is `ACTION_APPLIED_STATUS_UNAVAILABLE`,
    `effect: unknown`.
  - The commit request freezes `{ message, expectedSnapshotId }` from the status rendered at the COMMIT
    click (the outside-root modal carries the same object plus the count it displays); Confirm sends
    exactly that object, and a status refresh while it is pending never rebinds it — the agent's own
    re-read still governs.
  - `capture_status` reads the index identity before and after the porcelain read and retries up to 3 times
    on a mismatch; still torn → `indexTreeSha: ""` and therefore `snapshotId: ""`, which disables COMMIT in
    the UI and is refused by the agent until the next read.
  - `discard { targets: [{ path, expectedStamp?, expectedMissing? }] }` processes one target at a time
    under the lock, and each target's assertion decides what may happen to it. `expectedStamp` → the file
    is atomically renamed into that target's own quarantine directory as `claimed` and its
    bytes/`mtimeMs`/`mtimeNanos` verified there; a mismatch rolls it back and refuses with
    `SOURCE_CONTROL_STATE_CHANGED` (`notApplied` only when nothing earlier in the batch succeeded). A match
    writes the `verified` marker before anything is destroyed, then `git restore --worktree` (tracked) or
    the delete (untracked) runs, and the claim becomes `retained`; anything failing after that marker (the
    Git command, or the retention itself) holds the claim as `unrestored` and names it, because the
    worktree path is already empty and those bytes are the only copy left. `expectedMissing: true` → refused if the
    path now exists, otherwise restore only. Neither assertion → absent is restore-only (the rename
    origin); present is refused with `SOURCE_CONTROL_STATE_CHANGED` "cannot identify <path> before
    discarding it (not a regular file the review could stamp)", because nothing can prove a discard would
    destroy what the user looked at. A claim rename that fails with `CrossesDevices` is
    `SOURCE_CONTROL_UNSUPPORTED_LAYOUT`, `notApplied`. Any failure after the first successful target's
    effect boundary is `effect: unknown`, naming what was restored.
  - Rollback and put-back use `rename_no_replace` and are never allowed to destroy anything: an occupied
    destination (`AlreadyExists`) or a filesystem with no such rename (`Unsupported`) holds the bytes as
    `unrestored` in the quarantine directory, reports `effect: unknown`, and names where they are. WSL's
    9p mount of a Windows drive is the known `Unsupported` case.
  - Quarantine directories are swept once per git dir on the first status read: a directory whose `<opId>`
    names a discard running right now is skipped (a sibling configured root can start one at any moment);
    `verified` with no `unrestored` → removed (this is also how the previous session's `retained` bytes are
    released); everything else — `unrestored`, or a directory with no `verified` marker at all — is held
    and logged. One directory that cannot be read or removed is logged and never aborts the sweep, which
    reports removed/held/failed counts.
  - On a commit timeout the agent re-reads HEAD: moved → the commit is finalized and reported as applied
    with its sha and hook lists (the post-commit hook overran, the commit itself did not fail).
- A mutation that ran but whose follow-up status read failed is reported as
  `ACTION_APPLIED_STATUS_UNAVAILABLE` (never a `GIT_*` code), which the UI treats as an unknown outcome
  and reconciles by refetching.
- Every mutation error carries `details.effect: "notApplied" | "unknown"`, and outcome certainty is never
  inferred from a `GIT_*` code prefix. `notApplied` only when the agent proves no effect occurred
  (pre-flight refusals, a Git non-zero exit before its effect boundary, `SOURCE_CONTROL_STATE_CHANGED`,
  `INVALID_*`, `AGENT_DRAINING`); `unknown` after timeout, forced stop, or process-tree cleanup failure.
  Success means applied. The UI classifies purely on `details.effect`; a missing effect (transport
  failure, older agent) is treated as `unknown` and reconciles.
- After an `unknown` outcome or a transport failure the UI enters `reconciling` and refetches status with
  backoff (1 s, 2 s, 4 s, capped 8 s, up to the action's UI budget) until a status with
  `mutationInProgress === false` arrives; only then does the view return to `ready`, with the outcome
  reported from before/after HEAD (commit) or from the refreshed lists.
- Discard classifies paths from a fresh status: untracked → file removed; intent-to-add → index entry
  reset then file removed; anything else listed → `git restore --worktree`; unlisted → no-op.
- A Git mutation always has a process-tree owner or does not run: on Windows a `KillPolicy::Graceful`
  command whose Job Object cannot be created is refused before the spawn, and one whose job cannot be
  attached after the spawn kills the child and reports an unknown effect. Reads are allowed to run
  unowned (a detached reader is honest about what it owns) and say so once in the log.
- The host agent itself has an owner on Windows: the Tauri supervisor spawns it into a supervisor-owned
  Job Object and terminates that job on the emergency kill path, so Git and its hooks under a hung agent
  are reclaimed with it. An adopted agent has no job and is stopped by binary identity instead.
- The host agent's ledger of forwarded WSL mutations untracks a request id at the decode site, for `Ok`
  and `Error` envelopes alike and including a response that arrives after its timeout. Only transport
  loss and timeout leave an id outstanding, so a confirmed refusal can never hold a shutdown for the full
  emergency bound.
- Git children run in their own process group on Unix; on a runner timeout the process group is signalled
  and the reader threads are joined with a bounded post-exit drain, so a hook or `ssh` holding the pipes
  cannot wedge the per-repo lock past that bound.
- Protocol paths are UTF-8 strings of raw Git path bytes with the repo prefix stripped; C-quoted display
  forms never cross the wire.
- The watcher's detector keeps a tracked-path set loaded from `git ls-files -z` at watcher start and
  refreshed (on `spawn_blocking`, bounded) whenever `.git/index` changes; a worktree event for a tracked
  path always emits even under `node_modules`/`target`/ignore globs, while untracked noise under those
  globs stays suppressed. The metadata allowlist adds `.git/config`, `.git/info/exclude`, and
  `.git/worktrees/*/config` to the existing `index`/`HEAD`/`ORIG_HEAD`/`MERGE_HEAD`/`FETCH_HEAD`/
  `packed-refs`/`refs/**` set.
- Timeout ladder (agent per Git command < host→WSL request < UI request); a request may run several Git
  commands, so the outer tiers cover the summed worst case, not one command's raw budget:
  - Agent per Git command, unchanged: read 20 s, index (stage/unstage/discard) 60 s, commit 120 s, remote
    (push/pull) 180 s.
  - Host→WSL: status/diff 120 s, stage/unstage 280 s, discard 340 s, commit 380 s, push/pull 420 s.
  - UI: status/diff 150 s, stage/unstage 310 s, discard 370 s, commit 410 s, push/pull 450 s.
  - Discard is its own timeout class at every tier — it is no longer folded into stage/unstage. A UI
    timeout cancels nothing agent-side.
- Bounds: status output 8 MiB (`truncated`), diff 2 MiB (`truncated`), other outputs 1 MiB, stderr 64 KiB.
- Image diff sides are read by the agent with `git show` (`HEAD:`, `:0:`, `:2:`, `:3:`) or the worktree
  reader, never the webview; each side is bounded independently (12 MiB raw) and a side over the bound is
  flagged `truncated` rather than failing the whole request. A side with no blob at its snapshot (added,
  deleted, or a stage absent from a conflict) is `null`, never an error. MIME type for every side derives
  from the one extension map shared with `readImageFile`; the UI routes a path to the image diff only when
  `isPreviewImagePath(path)` is true, so unsupported extensions and non-image binaries stay on the text diff
  path and keep `BINARY FILE`.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Git not on PATH | `GIT_UNAVAILABLE` → `GIT NOT FOUND` empty state. Repo root missing is `INVALID_REPO`, not confused with a missing binary. |
| Not a Git repository | `GIT_NOT_REPOSITORY` → `NOT A GIT REPOSITORY`. |
| Older installed agent | `UNKNOWN_COMMAND` → `AGENT UPDATE REQUIRED`. |
| Nothing committable (index equals HEAD and no merge in progress) | Button disabled; agent refuses with `GIT_NOTHING_TO_COMMIT`. |
| Unmerged paths (in root or above a subdirectory root) | Button disabled with "Resolve N merge conflicts to commit"; agent refuses with `GIT_UNMERGED_PATHS`. |
| Hook or identity failure on commit | `GIT_COMMAND_FAILED` with Git's stderr/stdout text inline. |
| Push without upstream and ≠1 remote | `GIT_COMMAND_FAILED` "No upstream; configure one remote or set an upstream". |
| Commit or discard reviewed against a stale snapshot (branch, HEAD, index tree or merge state moved, or a target's on-disk stamp changed since the last read) | `SOURCE_CONTROL_STATE_CHANGED`, `effect: notApplied`; UI shows "STATE CHANGED — REVIEW AGAIN" and auto-refreshes status. |
| Commit sent while the reviewed status carried `snapshotId: ""` | COMMIT is already disabled with "Review did not capture a stable snapshot; refresh before committing"; the agent refuses an empty id rather than comparing it, `SOURCE_CONTROL_STATE_CHANGED`, `notApplied`. |
| A pre-commit hook rewrites content in paths the reviewed tree already touched | The commit stands; the result carries `hookChangedPaths` naming them, shown as an informational notice. |
| A pre-commit hook stages a path the reviewed tree did not touch | The commit stands — Git published it. The result carries `hookAddedPaths` naming those paths and the column shows a warning-tone "COMMIT HOOK ADDED UNREVIEWED FILES" notice saying the last commit can be undone with a soft reset. |
| The index changes between `capture_status`'s two identity reads | Retried up to 3 times; still torn → `indexTreeSha: ""` and `snapshotId: ""`, and a commit against it is refused until the next read. |
| Discard target recreated after being missing at the last status read (`expectedMissing: true`) | Refused `SOURCE_CONTROL_STATE_CHANGED` (a newer file appeared) instead of restoring over it. |
| Discard target that exists but carries no assertion (a directory, a symlink, a file behind an unreadable parent) | Refused `SOURCE_CONTROL_STATE_CHANGED` "cannot identify <path> before discarding it (not a regular file the review could stamp)", `notApplied` when nothing earlier succeeded. |
| A quarantined discard target's bytes/`mtimeMs`/`mtimeNanos` mismatch the live file | Rolled back out of quarantine with `rename_no_replace`, refused `SOURCE_CONTROL_STATE_CHANGED`, `effect: notApplied` only when no earlier target in the batch already succeeded. |
| A rollback or put-back cannot return the claimed bytes (destination occupied, or the filesystem has no no-replace rename — WSL's 9p mount of a Windows drive) | Nothing is overwritten: the claim is held as `unrestored` in its quarantine directory, `effect: unknown`, and the message names that path. The next sweep keeps that directory rather than deleting it. |
| A discard step fails after its `verified` marker was written (the Git restore/reset, or the retention rename) | The claim is held as `unrestored` before the failure returns, `effect: unknown`, and the message names that path — the worktree path is already empty, so the sweep must not finish a destruction that never happened. |
| A sibling configured root's first status read sweeps while a discard is running over the same git dir | The sweep skips every directory whose `<opId>` is registered live, so an in-flight claim is never removed by another root's startup sweep. |
| One quarantine directory cannot be removed by the sweep | Logged with its path; the sweep continues and still releases every other finished directory, and reports removed/held/failed counts. |
| The worktree and its repository sit on different volumes, so no claim rename can ever succeed | `SOURCE_CONTROL_UNSUPPORTED_LAYOUT` ("the worktree and its repository live on different volumes"), `effect: notApplied`; UI heading "UNSUPPORTED REPOSITORY LAYOUT". |
| A quarantine directory survives a crash with no `verified` marker | Held and logged by the startup sweep, never removed — nothing proved those bytes were the ones the user confirmed. |
| A Git mutation on Windows cannot be given a Job Object | Refused before the spawn (`effect: notApplied`); an attach failure after the spawn kills the child and reports `effect: unknown`. Reads proceed unowned and log once. |
| Discard batch: a later target fails after an earlier one's effect landed | `effect: unknown`, message lists what was already restored — never `notApplied` after an effect. |
| New mutation requested while the agent is draining for shutdown | `AGENT_DRAINING`, `effect: notApplied`; reads are still served. |
| Commit times out but HEAD moved when the agent re-checks | Reported as applied, with the new sha (the post-commit hook overran; the commit itself landed). |
| A hook, `ssh`, or other descendant keeps holding Git's stdout/stderr pipes after the direct child exits | Bounded post-exit drain (2 s grace); on expiry the process group is killed (Unix) or the Windows Job Object is terminated, reclaiming every descendant instead of detaching the readers; the per-repo lock is released either way. |
| App close/restart/SIGTERM during an active mutation, still running past the 60 s per-request bound | The agent sets draining and keeps waiting — `drained: false` never exits it — up to a 450 s emergency bound, then terminates its owned process tree, logs `unknown` with the residue, and exits; the host agent drains the WSL backend first and treats `WSL_BACKEND_UNAVAILABLE` as drained only with no outstanding forwarded mutation. |
| Supervisor stop/restart: no `drained: true` ack, or the process disappears without one | Never labelled `drained`; the supervisor waits up to 480 s before its kill path, logs `unknown`, and skips WSL distro termination while finality is unknown. |
| Socket closes or transport times out during an action | UI enters `reconciling` ("COMMIT RESULT UNKNOWN — REFRESHING"), never auto-retries, reports the outcome from the refetched status once `mutationInProgress` is false. |
| Status over 8 MiB | `truncated`; STAGE ALL and COMMIT disabled with a degraded banner. |
| Burst of thousands of file events | Coalescer emits at most one event per 250 ms and guarantees a trailing emit; `target/`, `node_modules/`, and the repo's ignore globs never fire for untracked paths — a tracked path under those globs still emits. |
| WSL backend offline | Existing transport errors and `wslBackendStatus` handling; status/diff retry with the transient backoff, actions do not. |

The runner does not guarantee zero leftover `.git/index.lock` in every case — it guarantees a graceful
stop on timeout (SIGTERM/TerminateProcess, not a kill), a drain on shutdown (mutation finishes or the
agent waits up to its drain budget before exiting), and that any lock residue it can observe is named in
the `GIT_ABORTED` message rather than silently discarded.

## Related docs

- [docs/design/source_control_design.md](../design/source_control_design.md) — goals, behaviour table, acceptance
- [docs/architecture/bundle_format_architecture.md](bundle_format_architecture.md) — the other consumer of the shared Git runner
- [docs/reports/source_control_adversarial_review_20260903.md](../reports/source_control_adversarial_review_20260903.md) — the adversarial review the first hardening pass addresses
- [docs/reports/source_control_fix_layer_review_20260903.md](../reports/source_control_fix_layer_review_20260903.md) — the round-2 closure review (commit/discard effect-boundary binding, governed shutdown)
- [docs/reports/source_control_hardening_review_20260903.md](../reports/source_control_hardening_review_20260903.md) — the round-3 review this document's snapshot identity, hook reporting, quarantine retention, and process-tree ownership answer; its rejected findings are recorded as accepted boundaries in the design doc
