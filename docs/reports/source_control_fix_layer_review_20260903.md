# Intermediary Source Control Fix-Layer Closure Review

Updated on: 2026-09-03  
Review authority: `intermediary_context_20260903_131046_1b75f0a.zip`  
Captured HEAD: `1b75f0a0a75908a85c06160f55281f15725d2849`  
Candidate index tree: `94ed29facc76b983f07970449e4c6008afde51da`  
Verdict: **HOLD only for two remaining P0 ownership defects. Accept the rest of this hardening layer.**

## Review boundary

The archive is complete for this fix layer: 105 selected changed paths, 74 tracked changes, 31 untracked additions, zero changed paths omitted, and complete status/diff evidence. It is a changeset bundle rather than a replacement full-repository authority, so I reviewed the supplied implementation delta against the authoritative Source Control design, architecture, ADRs, and the prior adversarial report contained in the bundle. See @BUNDLE_MANIFEST.json, @BUNDLE_GIT_STATUS.txt, @docs/guide.md:8-55, and @docs/reports/source_control_adversarial_review_20260903.md:366-381.

No dedicated active Quest document is present in the bundle. The prior review's required end state and the updated Source Control design/architecture therefore provide the relevant task intent. Test or gate success was not used as acceptance evidence. Where a defect depended on Git semantics, I reproduced it in disposable repositories and included the outputs in the companion evidence archive.

## Executive verdict

This is a **real, substantial correction**, not a cosmetic response to the earlier report. The implementation closes most of the original findings cleanly: copy provenance is no longer acted upon, section actions no longer use pathspec `.`, unresolved conflicts block commit, timeout tiers reflect whole requests, failure effect is typed independently of `GIT_*`, locks key on physical Git directories, pipe drain is bounded, and tracked files override structural watcher suppression.

I would not reopen those areas or hold this tree for secondary cleanup.

Two missing authorities remain, however, and both can still violate the central product promise under ordinary concurrent-agent/Git behavior:

1. the reviewed snapshot is checked before mutation, but it does not own the state at the **effect boundary**; and
2. shutdown reports that a mutation did not drain, then exits/kills its owners anyway.

Those are direct paths to an unreviewed commit or destruction of newer work, so they are the only reasons for this hold.

## Behaviour check

| Situation | Required visible/product behaviour | Current result |
| --- | --- | --- |
| A pre-commit hook or concurrent tool stages another path after preflight | Commit exactly the reviewed index, or refuse before HEAD moves | **Fails:** normal `git commit` consumes the later live index and includes the added path |
| A tracked file was missing at review, then another agent recreates it before Discard | Refuse and preserve the newer file | **Fails:** absent stamp skips verification and `git restore` overwrites it |
| A mutation is still active after the 60 s shutdown drain | Keep its owner alive until terminal, or perform owned process-tree emergency finalization | **Fails:** agents schedule exit regardless of `drained: false`; supervisor then enters the kill path |
| Discard a copied row whose source has an unrelated edit | Touch the copy destination only | **Passes** |
| Stage all ordinary CHANGES while conflicts exist | Stage only the ordinary section; do not resolve MERGE CHANGES | **Passes** |
| A tracked file changes under `target/` or another structural ignore | Refresh Source Control without admitting untracked noise | **Passes** |

---

## P0-1 — The reviewed snapshot is advisory; the live worktree/index still owns the effect boundary

### Violated contract

The design says a stale commit or discard is refused rather than silently retargeted, and acceptance requires that newer index/file state survive: @docs/design/source_control_design.md:65-72 and @docs/design/source_control_design.md:128-136. The architecture says mutations are bound to the snapshot the user reviewed: @docs/architecture/source_control_architecture.md:84-100.

The implementation has added preconditions, but each precondition remains separated from the destructive operation by an unowned live-state interval.

### A. A valid `expectedIndexTreeSha` does not constrain what `git commit` records

The backend re-reads status and compares `indexTreeSha` at @crates/im_agent/src/source_control/actions_commit.rs:34-42. It then launches ordinary `git commit` against the live index at @crates/im_agent/src/source_control/actions_commit.rs:55-59.

Nothing holds external Git or a commit hook to the preflight identity between those points. This is not only a timing race: a standard pre-commit hook runs **inside that same `git commit`** and may stage another file after the expected hash passed.

The companion witness `precommit_stages_unreviewed_path.txt` uses a configured `sub/` root:

- the reviewed index contains only `sub/reviewed.txt`;
- `outside.txt` is modified but unstaged and outside the configured root, so `stagedOutsideRoot` is zero;
- the pre-commit hook runs `git add -- outside.txt`;
- HEAD moves to a commit containing both paths;
- the committed tree differs from the reviewed tree.

The user receives no outside-root confirmation. This is the exact P0-2 failure the hash was meant to eliminate, now moved from “before the request” into the commit effect boundary.

### B. Status itself can pair an old list with a newer index identity

`capture_status` performs independent sequential reads: porcelain status, committability probes, then index-tree capture at @crates/im_agent/src/source_control/status.rs:34-50. There is no index identity before/after check and no retry when it changes.

An external `git add` between the porcelain read and index-tree capture therefore produces one wire object whose visible rows describe index A while `indexTreeSha` names index B. A later commit preflight sees B and passes. `torn_status_index_snapshot.txt` reproduces that shape: `outside.txt` is absent from the displayed porcelain capture but present in the identity and resulting commit.

### C. Outside-root confirmation rebinds to a later snapshot

The modal opens from the currently rendered status at @app/src/components/source_control/source_control_column.tsx:99-106, but its Confirm callback merely calls `commit(commitMessage)` at @app/src/components/source_control/source_control_column.tsx:192-200. The command then fetches the **current** SHA from a mutable ref at @app/src/hooks/source_control/source_control_commands.ts:25-29. Background/event/focus refreshes continuously replace that ref at @app/src/hooks/source_control/use_source_control_state.ts:76-84 and @app/src/hooks/source_control/use_source_control_state.ts:181-218.

Thus the modal can open for index A and authorize index B. A same-count substitution of outside-root paths requires no renewed confirmation.

### D. Discard does not preserve “missing at review,” content identity, or atomicity

A missing, symlink, directory, or other non-regular target all collapse to “no stamp” at @crates/im_agent/src/source_control/status_stamp.rs:10-21. The wire makes the stamp optional at @app/src/shared/protocol_source_control.ts:84-89. Verification then skips every absent stamp at @crates/im_agent/src/source_control/actions_discard.rs:81-100.

If a tracked file was deleted at review and another agent recreates it before Discard, fresh classification places it in the restore set and `git restore --worktree` overwrites it at @crates/im_agent/src/source_control/actions_discard.rs:47-59 and @crates/im_agent/src/source_control/actions_discard.rs:121-147. `missing_target_recreated_before_discard.txt` reproduces the newer content being erased.

For existing files, `{ bytes, mtimeMs }` is metadata, not content identity. The mtime is truncated to milliseconds at @crates/im_agent/src/source_control/status_stamp.rs:18-29; `stamp_collision.txt` reproduces a same-length rewrite with the same accepted stamp. The check also occurs before another status capture and before restore/removal, leaving a TOCTOU interval external writers do not share the app lock.

Finally, one multi-path `git restore` can restore an earlier path and fail on a later one. `partial_git_restore.txt` reproduces exit 255 after `a.txt` was already restored. The caller nevertheless defaults a non-zero restore exit to `effect: notApplied` at @crates/im_agent/src/source_control/actions_discard.rs:103-112. That is a false safety result after destructive work occurred.

### Consequence

The current tree can still:

- commit a path the user never reviewed or confirmed, including one outside the configured root;
- overwrite a file another agent created after the confirmation snapshot;
- erase same-length newer content that retained the accepted timestamp;
- report `notApplied` after a partial destructive restore.

These are direct correctness/data-loss defects, not acceptance-matrix omissions.

### Required closure — one retained reviewed-state transaction

Do not add another preflight comparison. Make the reviewed state the object the mutation consumes:

1. **Capture one retained snapshot token.** Build status from a private immutable index snapshot, not sequential reads of the live index. The token owns the exact index tree, HEAD/merge parents, scoped relational records, outside-root effects, and typed target states (`missing`, regular-file digest, symlink target, unsupported/refuse).
2. **Freeze intent at confirmation.** The commit modal stores `{ snapshotToken, message, outsideEffectAcknowledgement }`; Confirm sends that exact object. Background status may invalidate the modal, never silently substitute a new token.
3. **Publish the reviewed commit by compare-and-swap.** Construct the commit from the retained index tree and update the intended ref only if its reviewed HEAD still matches. Hooks execute inside the private transaction; if a pre-commit hook changes its tree, the operation refuses before ref publication or yields a new snapshot requiring review. A hook/live-index mutation cannot alter the tree already authorized.
4. **Claim discard targets atomically.** Existing files are atomically moved to operation-owned quarantine before verification; missing targets are installed only with no-replace semantics. Verify content digests/type from the claimed object, restore the original on mismatch, and never overwrite a path recreated after the claim. Any failure after the first effect boundary is `unknown` unless final-state comparison proves otherwise.

That is the missing snapshot/operation owner requested at @docs/reports/source_control_adversarial_review_20260903.md:366-378. It closes the commit, modal, missing-file, metadata-collision, TOCTOU, and partial-effect cases as one design rather than another set of local checks.

---

## P0-2 — Shutdown knowingly exits while mutations remain and still has no Windows process-tree finality

### Violated contract

The product says closing/restarting drains an active mutation and nothing is killed mid-command: @docs/design/source_control_design.md:79-90, @docs/prd.md:136-148, and @docs/architecture/source_control_architecture.md:53-62. The prior required end state was: stop admission, drain operation IDs, own emergency process-tree termination, then terminate agents/WSL: @docs/reports/source_control_adversarial_review_20260903.md:366-379.

### What the tree does

Agent command bounds are 60 s for index work, 120 s for commit, and 180 s for remote work at @crates/im_agent/src/source_control/runner.rs:26-30. Whole requests additionally perform sequential status/HEAD/remotes work.

The shutdown drain is nevertheless 60 s at @crates/im_agent/src/server/shutdown.rs:22-30. When that budget expires, `drain_source_control` truthfully returns `drained: false` and a non-zero residue at @crates/im_agent/src/server/shutdown.rs:53-82 — but `schedule_process_exit` unconditionally calls `process::exit(0)` at @crates/im_agent/src/server/shutdown.rs:85-102.

Both command paths schedule that exit regardless of outcome:

- WSL agent: @crates/im_agent/src/server/connection/shutdown_command.rs:14-24;
- host agent: @crates/im_host_agent/src/server/shutdown_dispatch.rs:23-36.

The signal paths also return after the same bounded drain even when mutations remain: @crates/im_agent/src/server/ws_server.rs:84-98 and @crates/im_host_agent/src/server/ws_server.rs:87-101.

The Tauri supervisor logs `ack.drained` but does not branch on it at @src-tauri/src/lib/agent/supervisor/graceful_stop.rs:59-97. If the agent process exits, it labels the route `Drained` at @src-tauri/src/lib/agent/supervisor/graceful_stop.rs:99-113. The ordinary stop then runs its kill/reconciliation path after any incomplete drain at @src-tauri/src/lib/agent/supervisor/managed_processes.rs:13-43 and @src-tauri/src/lib/agent/supervisor/managed_processes.rs:88-155.

The host also treats `WSL_BACKEND_UNAVAILABLE` as proof the WSL side is idle at @crates/im_host_agent/src/server/shutdown_dispatch.rs:55-108, even though a transport timeout/disconnect can coexist with a passive mutation still running there.

On Windows—the primary product platform—the Git runner creates no process-tree owner: `own_process_group`, group termination, and descendant cleanup are no-ops at @crates/im_bundle/src/git_capture/command_stop.rs:13-24 and @crates/im_bundle/src/git_capture/command_stop.rs:85-98. Post-exit pipe holders are detached rather than terminated at @crates/im_bundle/src/git_capture/command_drain.rs:21-45. This is already acknowledged as deliberately deferred at @docs/known_issues.md:78-81, but it was a P1 requirement of the prior end state and is load-bearing once shutdown/timeout claims process finality.

### Consequence

A commit/pull/push that legitimately lasts more than 60 seconds can receive this lifecycle:

1. shutdown stops admission and waits 60 seconds;
2. agent reports active residue;
3. that same agent exits one second later anyway;
4. supervisor calls the result drained if the process disappeared and proceeds through stop/WSL teardown;
5. Git is orphaned, directly killed, or killed with the WSL distro without an owned final state.

On Windows, hooks, SSH, credential helpers, and `git-remote-*` descendants may also survive a direct-child timeout after the operation lock has been released. The system cannot then prove repository effect, child completion, or index-lock cleanup.

### Required closure — operation finality before process finality

1. The operation coordinator owns every active mutation ID and its full Git process tree (Unix process group; Windows Job Object or equivalent).
2. `shutdownResult.drained: false` is an incomplete shutdown. Agents do **not** call `process::exit` from that result, and the supervisor never labels process disappearance as drained without a true acknowledgement.
3. Routine close may let the UI process go away, but an agent/guardian remains alive until every admitted operation reaches a terminal state. Restart waits on that same owner. WSL is not terminated while finality is unknown.
4. `WSL_BACKEND_UNAVAILABLE` means unknown unless port/process ownership proves the managed backend and its operations are absent.
5. At an explicit emergency deadline, the coordinator terminates and reaps the complete process tree, records `unknown` plus lock residue/final identities, and only then permits agent or distro termination.

Do not solve this by changing 60 to a larger number while retaining unconditional exit. The defect is that the reported drain result does not govern lifecycle authority.

---

## Accepted from this fix layer

The following previous findings are adequately addressed and should not be reopened in this lane:

- **Copy/rename semantics:** a copied row acts only on its destination; a rename carries both endpoints. @app/src/components/source_control/source_control_column.tsx:37-53 and @crates/im_agent/src/source_control/actions_discard.rs:20-34.
- **Section ownership:** stage-all/unstage-all enumerate fresh section paths and use NUL pathspec input; conflicts are excluded and unresolved records block commit. @crates/im_agent/src/source_control/actions_stage.rs:16-44 and @crates/im_agent/src/source_control/actions_stage.rs:63-101.
- **Physical serialization:** app-owned mutations sharing a Git directory use one lock rather than `repoId`-local locks. @crates/im_agent/src/source_control/locks.rs:52-70 and @crates/im_agent/src/source_control/locks.rs:138-155.
- **Timeout ladder:** host→WSL and UI tiers now cover multi-command request envelopes; discard has its own tier. @docs/design/source_control_design.md:92-101.
- **Outcome vocabulary:** errors carry typed `effect`; commit timeout re-checks HEAD; follow-up read failure is not mislabeled as a pre-application Git failure. @crates/im_agent/src/error/mutation_effect.rs:8-45, @crates/im_agent/src/source_control/actions_commit.rs:59-67, and @crates/im_agent/src/source_control/actions.rs:83-100.
- **Pipe/watcher/UI secondary fixes:** post-child pipe wait is bounded; tracked paths override structural ignores; empty explicit path actions, deleted-row diff affordance, and distinct-path counts were corrected. The Windows descendant gap is the sole process-drain exception described above.

## Documentation disposition

@docs/changelog.md:10 and @docs/roadmap.md:51 currently say the P0/P1 hardening landed. That is too strong while the reviewed state does not own the effect boundary and `drained: false` still leads to process exit. Do not unwind the feature or its other documentation; mark this hardening as still active until these two owners are true.

## Closeout decision

**Do not abandon or broadly re-review this fix layer.** It is close and most of it is good. Close only these two owners, then take the tree forward.

The minimum decisive witness set is:

1. the supplied pre-commit hook scenario either commits exactly the reviewed tree or refuses before HEAD moves;
2. missing-then-recreated, same-length/same-mtime, and multi-path partial-failure discard scenarios preserve newer bytes and never claim `notApplied` after an effect;
3. host and WSL close/restart during operations longer than 60 seconds leave the operation owned to terminal state, no `.git/index.lock`, and no surviving Windows hook/helper process after an emergency stop.

After those witnesses, I would accept this hardening lane without another full adversarial pass.

## Companion evidence

The evidence archive contains:

- `precommit_stages_unreviewed_path.txt`
- `torn_status_index_snapshot.txt`
- `missing_target_recreated_before_discard.txt`
- `stamp_collision.txt`
- `partial_git_restore.txt`
- `shutdown_budget_and_exit.txt`

All probes used disposable repositories and did not modify the bundled tree.
