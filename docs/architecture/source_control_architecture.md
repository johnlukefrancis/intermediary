# Source Control Architecture
Updated on: 2026-09-03
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010

---

## Ownership

| Concern | Owner | Notes |
| --- | --- | --- |
| Running Git (bounded, cancellable, stderr-preserving) | `crates/im_bundle/src/git.rs` facade over `git_capture/{command,command_child,porcelain,path,prefix,diff}.rs` | One runner for bundle evidence and source control. `GitCommandFailure { kind, exit_code, stdout, stderr }`; `KillPolicy::{Immediate, Graceful}`. |
| Status projection, diff, actions | `crates/im_agent/src/source_control/` | `source_control_status`, `source_control_diff`, `run_source_control_action`; `SourceControlLocks` (per-repo mutation mutex) lives on `AgentRuntime`. |
| Wire types | `crates/im_agent/src/protocol/{commands,responses}_source_control.rs` and `app/src/shared/protocol_source_control.ts` | Hand-kept in sync; serde `camelCase`, tagged unions (`kind`, `mode`). |
| WSL-agent dispatch | `crates/im_agent/src/server/connection/source_control_commands.rs` | Reads get a `SourceControlRead` cancel token; actions are `Passive`. |
| Host-agent dispatch | `crates/im_host_agent/src/server/dispatch.rs` (`dispatch_source_control`) + `runtime/local_host_source_control_backend.rs` | Backend resolved under a short read lock; host repos run Git with no runtime lock held; WSL repos are forwarded. |
| Refresh signal | `crates/im_agent/src/repos/source_control_watch/` (detector, coalescer, git dirs) wired into `repo_watcher*.rs` | Emits `AgentEvent::SourceControlChanged { repoId }`, coalesced to one event per 250 ms with a trailing emit. |
| UI state | `app/src/hooks/source_control/use_source_control_state.ts`, `app/src/hooks/use_deck_section.ts` | One hook instance per active repo feeds the rail count and the column. |
| UI surface | `app/src/components/layout/{deck_section_switcher,repo_rail}.tsx`, `app/src/components/source_control/*`, `app/src/components/diff_workspace.tsx` | Rail switch, column, rows, commit box, diff kind of the shared workspace. |
| Persisted choice | `uiState.activeRail` in `app/src/shared/config/persisted_config.ts` | Global; defaulted, no migration. |

## Routing and lifecycle

1. The UI sends `sourceControlStatus` / `sourceControlDiff` / `sourceControlAction` to the host agent
   over the existing token-authenticated socket on `127.0.0.1:3141` (no new port; ADR-010 unchanged).
2. `server/dispatch.rs` intercepts the three commands before the `&mut self` catch-all. It resolves the
   repo backend under a read lock and drops it. Host-rooted repos: `LocalHostBackend::source_control_context`
   clones the root path and the lock registry, then `execute_host_source_control` runs Git with no runtime
   lock held. WSL-rooted repos: the command is forwarded verbatim to `im_agent` with the per-kind timeout
   from `wsl_backend_client.rs::timeout_for_command`.
3. In either agent, `im_agent::source_control` resolves the repo prefix (`git rev-parse --show-prefix`),
   runs the Git command on `spawn_blocking` from `common_git_args()` (always `--literal-pathspecs`), and
   maps failures to `AgentError` codes: `GIT_UNAVAILABLE`, `GIT_NOT_REPOSITORY`, `GIT_TIMEOUT`,
   `GIT_ABORTED`, `GIT_NOTHING_TO_COMMIT`, `GIT_COMMAND_FAILED` (Git's own text), `GIT_UNSUPPORTED_VERSION`,
   `INVALID_PATH`, `INVALID_COMMIT_MESSAGE`, `INVALID_REPO`.
4. Every action re-reads status after the Git command and returns it, so the view is never stale after
   its own mutation.
5. The watcher marks the coalescer dirty for `.git` metadata writes (`index`, `HEAD`, `ORIG_HEAD`,
   `MERGE_HEAD`, `FETCH_HEAD`, `packed-refs`, `refs/**`, and their `.lock` files) and for working-tree
   events outside the detector's own structural matcher (`node_modules`, `target`, plus the repo's
   `ignoreGlobs`); linked worktrees get a second watch on their real git dir and the common `refs`.
   The UI debounces the event 300 ms, drops refetches that predate its own last mutation, and also
   refetches on window focus, hello, and rehydrate. There is no interval polling.

## Invariants

- The UI never runs Git; every read and mutation is an agent request routed by repoId.
- Mutations on one repo are serialized (`SourceControlLocks`); the per-repo lock is cloned out and awaited
  with no runtime guard held.
- Mutations are never killed mid-command by cancellation. On timeout they are stopped gracefully
  (SIGTERM then wait on Unix; TerminateProcess on Windows) and report `GIT_ABORTED` naming a leftover
  `.git/index.lock`. Reads are cancellable and killed immediately.
- No source-control command reaches `HostRuntime::dispatch_command` (`&mut self`); that arm is a release
  guard that returns an internal error.
- `UiCommand::repo_id()` is exhaustive; a new repo-scoped command that is not listed fails to compile.
- Section-wide actions use pathspec `.` (inside the configured root); an empty explicit path list is
  rejected before any process spawns; `git commit` always commits the whole index and the UI surfaces
  `omitted.stagedOutsideRoot` with a confirm.
- Committability is Git's answer, not the projected list: `status.committable` is true when
  `git diff --cached --quiet` reports a difference from HEAD or `MERGE_HEAD` exists, so a merge resolved
  to HEAD's tree and a commit whose staged paths sit above the configured root both stay committable.
- A mutation that ran but whose follow-up status read failed is reported as
  `ACTION_APPLIED_STATUS_UNAVAILABLE` (never a `GIT_*` code), which the UI treats as an unknown outcome
  and reconciles by refetching.
- Discard classifies paths from a fresh status: untracked → file removed; intent-to-add → index entry
  reset then file removed; anything else listed → `git restore --worktree`; unlisted → no-op.
- Git children run in their own process group on Unix; a forced stop signals the whole group and the
  reader threads are joined with a bounded wait, so a hook or `ssh` holding the pipes cannot wedge the
  per-repo lock.
- Protocol paths are UTF-8 strings of raw Git path bytes with the repo prefix stripped; C-quoted display
  forms never cross the wire.
- Timeout ladder is strictly nested (per Git command < host→WSL request < UI request); a request may run
  several Git commands, so the outer budgets cover the summed worst case: status/diff 20 s per command,
  90 s host→WSL, 120 s UI; stage/unstage/discard 60/120/150 s; commit 120/240/300 s; push/pull
  180/300/360 s. A UI timeout cancels nothing agent-side.
- Bounds: status output 8 MiB (`truncated`), diff 2 MiB (`truncated`), other outputs 1 MiB, stderr 64 KiB.

## Failure modes

| Failure | Behaviour |
| --- | --- |
| Git not on PATH | `GIT_UNAVAILABLE` → `GIT NOT FOUND` empty state. Repo root missing is `INVALID_REPO`, not confused with a missing binary. |
| Not a Git repository | `GIT_NOT_REPOSITORY` → `NOT A GIT REPOSITORY`. |
| Older installed agent | `UNKNOWN_COMMAND` → `AGENT UPDATE REQUIRED`. |
| Nothing committable (index equals HEAD and no merge in progress) | Button disabled; agent refuses with `GIT_NOTHING_TO_COMMIT`. |
| Hook or identity failure on commit | `GIT_COMMAND_FAILED` with Git's stderr/stdout text inline. |
| Push without upstream and ≠1 remote | `GIT_COMMAND_FAILED` "No upstream; configure one remote or set an upstream". |
| Socket closes or transport times out during an action | UI enters `reconciling` ("COMMIT RESULT UNKNOWN — REFRESHING"), never auto-retries, reports the outcome from the refetched status. |
| Status over 8 MiB | `truncated`; STAGE ALL and COMMIT disabled with a degraded banner. |
| Burst of thousands of file events | Coalescer emits at most one event per 250 ms and guarantees a trailing emit; `target/`, `node_modules/`, and the repo's ignore globs never fire. |
| WSL backend offline | Existing transport errors and `wslBackendStatus` handling; status/diff retry with the transient backoff, actions do not. |

## Related docs

- [docs/design/source_control_design.md](../design/source_control_design.md) — goals, behaviour table, acceptance
- [docs/architecture/bundle_format_architecture.md](bundle_format_architecture.md) — the other consumer of the shared Git runner
