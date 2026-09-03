# Source Control Architecture
Updated on: 2026-09-03
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

---

## Ownership

| Concern | Owner | Notes |
| --- | --- | --- |
| Running Git (bounded, cancellable, stderr-preserving) | `crates/im_bundle/src/git.rs` facade over `git_capture/{command,command_child,porcelain,path,prefix,diff}.rs` | One runner for bundle evidence and source control. `GitCommandFailure { kind, exit_code, stdout, stderr }`; `KillPolicy::{Immediate, Graceful}`. |
| Status projection, diff, actions | `crates/im_agent/src/source_control/`, `source_control/{image_diff,image_diff_sides}.rs` | `source_control_status`, `source_control_diff`, `source_control_image_diff`, `run_source_control_action`; `SourceControlLocks` (per-repo mutation mutex) lives on `AgentRuntime`. |
| Mutation locks | `crates/im_agent/src/source_control/locks.rs` | Keyed by the canonical absolute git dir (`git rev-parse --absolute-git-dir`, resolved once per repo root and cached), not by UI repo id — two configured roots over the same physical worktree serialize on the same lock; linked worktrees stay distinct. |
| Status identity | `crates/im_agent/src/source_control/{status_index_tree,status_stamp}.rs` | `indexTreeSha` (`git ls-files --stage -z` through `im_bundle::git::index_tree_sha`, read before and after the porcelain read with up to 3 retries on a torn identity — still torn → empty string), `mutationInProgress` (`SourceControlLocks::is_busy`), and per-entry `worktreeStamp` (`{ bytes, mtimeMs, mtimeNanos }` from `fs::metadata`) — the snapshot identity a commit or discard is bound to. A worktree/conflict entry whose file is absent on disk carries `worktreeMissing: true`. |
| Discard quarantine | `crates/im_agent/src/source_control/actions_discard.rs` (`.git/intermediary-discard/<opId>/`) | Each existing target is atomically claimed into an operation-owned quarantine directory before verification and restore/delete; a startup sweep removes leftovers from a crashed operation (bounded, logged). |
| Wire types | `crates/im_agent/src/protocol/{commands,responses}_source_control.rs` and `app/src/shared/protocol_source_control.ts` | Hand-kept in sync; serde `camelCase`, tagged unions (`kind`, `mode`). |
| WSL-agent dispatch | `crates/im_agent/src/server/connection/source_control_commands.rs` | Reads get a `SourceControlRead` cancel token; actions are `Passive`. |
| Host-agent dispatch | `crates/im_host_agent/src/server/dispatch.rs` (`dispatch_source_control`) + `runtime/local_host_source_control_backend.rs` | Backend resolved under a short read lock; host repos run Git in-process with no runtime lock held; WSL repos are forwarded. |
| Runner post-exit drain | `crates/im_bundle/src/git_capture/command_drain.rs` | After the direct Git child exits, pipe drain is bounded (2 s grace); on expiry the runner kills the child's process group on Unix and joins the readers, and on Windows terminates the job (below) instead of detaching them. Nothing is terminated inside the grace, so a helper that closed the pipes on purpose survives. |
| Windows process-tree ownership | `crates/im_bundle/src/git_capture/{command_tree,command_job}.rs` | Git's children are placed in a Job Object that carries no kill-on-close limit, mirroring the Unix process group: the tree is terminated on forced stop, drain expiry, and shutdown finalization, taking hooks, credential helpers, and `git-remote-*` descendants with it; helpers that close their pipes outlive Git as on Unix. |
| Shutdown drain | `crates/im_agent/src/source_control/locks.rs` (`set_draining` / `wait_idle`) + host `dispatch.rs` + Tauri supervisor | `Shutdown` command in both agents; `drained: false` never exits an agent — it keeps waiting up to a 450 s emergency bound, then terminates its owned process tree and logs `unknown`; host agent forwards to the WSL backend first and treats `WSL_BACKEND_UNAVAILABLE` as drained only with no outstanding forwarded mutation; Tauri supervisor waits up to 480 s on an explicit `drained: true` ack before its existing kill path, never labelling process disappearance `drained`, and skips WSL distro termination while finality is unknown. |
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
   `INVALID_PATH`, `INVALID_COMMIT_MESSAGE`, `INVALID_REPO`.
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
- Mutations are bound to the snapshot the user reviewed, not to whatever Git state exists when they run,
  and that binding is enforced at the effect boundary, not only as a precondition:
  - `commit { message, expectedIndexTreeSha, expectedHeadSha }` (`expectedHeadSha` is the reviewed
    `status.headSha`, `null` on an unborn branch) is refused under the lock with
    `SOURCE_CONTROL_STATE_CHANGED` when the index tree or HEAD differs from expected, and with
    `GIT_UNMERGED_PATHS` when unmerged records exist. `git commit` then runs (hooks included). Afterward
    the agent compares `HEAD^{tree}` with the expected tree: equal → applied. Different: it lists the
    differing paths (`git diff-tree -r --name-only <expected> HEAD^{tree}`); if every one was already in
    the reviewed index list (in-root, or counted outside-root when `omitted.stagedOutsideRoot` was
    acknowledged) → applied, and the result carries `hookChangedPaths` naming them; otherwise the commit is
    retracted (`git update-ref refs/heads/<branch> <previousHead> <newHead>`, CAS-guarded; `HEAD` itself for
    a detached checkout) and returned as `SOURCE_CONTROL_STATE_CHANGED` "a commit hook staged unreviewed
    paths: …", `effect: notApplied`; if the retraction itself fails, `effect: unknown` names the sha.
  - The commit modal freezes `{ expectedIndexTreeSha, expectedHeadSha, message, acknowledgedOutsideRoot }`
    from the status rendered when it opened; Confirm sends exactly that object, and a status refresh while
    the modal is open never rebinds it — the agent's own re-read still governs.
  - `capture_status` reads the index identity before and after the porcelain read and retries up to 3 times
    on a mismatch; still torn → `indexTreeSha: ""` (no stable identity), and a commit against it is refused
    until the next read.
  - `discard { targets: [{ path, expectedStamp?, expectedMissing? }] }` processes one target at a time
    under the lock. An existing target is atomically renamed into an operation-owned quarantine directory
    (`.git/intermediary-discard/<opId>/`) and its bytes/`mtimeMs`/`mtimeNanos` are verified there before
    `git restore --worktree` (tracked) or delete (untracked) runs; a mismatch renames it back and refuses
    with `SOURCE_CONTROL_STATE_CHANGED` (`notApplied` only when nothing earlier in the batch already
    succeeded). `expectedMissing: true` refuses with `SOURCE_CONTROL_STATE_CHANGED` if the path now exists
    (a newer file appeared) rather than restoring over it; otherwise it only runs `git restore --worktree`.
    Any failure after the first successful target's effect boundary is `effect: unknown`, naming what was
    restored. Quarantine leftovers from a crash are swept at agent start (bounded, logged).
  - On a commit timeout the agent re-reads HEAD: moved → the commit is reported as applied with its sha
    (the post-commit hook overran, the commit itself did not fail).
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
| Commit or discard reviewed against a stale snapshot (index moved, or a target's on-disk stamp changed since the last read) | `SOURCE_CONTROL_STATE_CHANGED`, `effect: notApplied`; UI shows "STATE CHANGED — REVIEW AGAIN" and auto-refreshes status. |
| A pre-commit hook stages only paths already in the reviewed index list | Accepted; the result carries `hookChangedPaths` naming the paths the hook itself changed. |
| A pre-commit hook stages a path outside the reviewed index list | The commit is retracted (ref moved back to the previous HEAD, CAS-guarded) before it is visible; `SOURCE_CONTROL_STATE_CHANGED` "a commit hook staged unreviewed paths: …", `effect: notApplied`; if the retraction itself fails, `effect: unknown` naming the sha. |
| The index changes between `capture_status`'s two identity reads | Retried up to 3 times; still torn → `indexTreeSha: ""`, and a commit against it is refused until the next read. |
| Discard target recreated after being missing at the last status read (`expectedMissing: true`) | Refused `SOURCE_CONTROL_STATE_CHANGED` (a newer file appeared) instead of restoring over it. |
| A quarantined discard target's bytes/`mtimeMs`/`mtimeNanos` mismatch the live file | Renamed back out of quarantine, refused `SOURCE_CONTROL_STATE_CHANGED`, `effect: notApplied` only when no earlier target in the batch already succeeded. |
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
- [docs/reports/source_control_fix_layer_review_20260903.md](../reports/source_control_fix_layer_review_20260903.md) — the round-2 closure review (commit/discard effect-boundary binding, governed shutdown) this pass answers
