# Intermediary Source Control Hardening — Adversarial Review
Updated on: 2026-09-03
Owners: JL · External reviewer
Depends on: ADR-000, ADR-007, ADR-008, ADR-009, ADR-013

## Verdict

**Retain this tree as the foundation, but do not accept it as the completed hardening end state. Four concrete P0 defects remain.**

The candidate is materially better than the previous reviewed tree: index/HEAD preflight identity, typed mutation effects, per-target discard claims, a 450-second agent drain, WSL outstanding-request tracking, and Windows Job Object support are all real improvements. Ordinary single-writer use can therefore look correct. The remaining failures occur exactly at the boundaries this fix layer was meant to close: concurrent Git actors/hooks, destructive file replacement, process failure, and shutdown transport loss.

Authority reviewed:

- Bundle: `intermediary_context_20260903_180302_1b75f0a.zip`
- Captured HEAD: `1b75f0a0a75908a85c06160f55281f15725d2849`
- Candidate index tree: `7ee90974a4da9fcc58a34c157d01525b5f4c4652`
- Capture: complete; 174 selected changed paths; zero changed paths omitted (`@BUNDLE_MANIFEST.json`, `@BUNDLE_GIT_STATUS.txt`)
- Canon consulted first: `@docs/guide.md`, then `@docs/prd.md`, `@docs/design/source_control_design.md`, `@docs/architecture/source_control_architecture.md`, both 2026-09-03 review reports, and the relevant ADRs.

This review does not use gate or test status as evidence. The concrete failures below were verified from current source and, where noted, reproduced in disposable Git/filesystem probes.

## Priority summary

| Priority | Finding | Concrete impact |
| --- | --- | --- |
| P0 | Commit still has no retained reviewed-state transaction | A commit can land on an unreviewed ref, with an unreviewed merge parent, unreviewed same-path content, or a substituted message while all implemented preconditions pass. |
| P0 | Hook rejection is post-publication compensation mislabeled `notApplied` | The ref, index, reflog, hooks, and external observers already see effects before retraction. |
| P0 | Discard quarantine is not content-identity/no-replace safe | Newer file contents can be overwritten or swept away after a stamp collision, TOCTOU write, or crash. |
| P0 | Emergency process-tree finality dies with the agent | If the agent hangs or crashes before its finalizer, the supervisor kills only the agent and can orphan Git/hooks/helpers. |
| P1 | Confirmed WSL application errors leak into the outstanding ledger | A completed refusal can poison later shutdown and hold close/restart for the full 450-second bound. |
| P1 | `.git` quarantine cannot claim cross-volume linked-worktree files | Discard fails for an otherwise supported worktree placed on another drive/filesystem. |
| P2 | Canon declares closure and defers structure on facts the tree disproves | Future agents are directed to trust false effect semantics and a stale ADR-000 exception. |

## P0-1 — Commit still reconstructs intent from mutable live Git state

### Violated invariant

The PRD requires commit to remain bound to the snapshot the user reviewed at the moment of effect (`@docs/prd.md:136-148`). The closure report required one retained token owning the exact index tree, intended ref, HEAD/merge parents, outside-root effects, typed targets, message, and acknowledgement; it specifically required constructing the commit from the private tree and compare-and-swap publishing the intended ref (`@docs/reports/source_control_fix_layer_review_20260903.md:98-107`). ADR-007 requires restoring that owner rather than layering checks around live state (`@docs/compliance/adr_007_architecture_first_execution.md:16-22`).

### What the tree does

The UI and both wire contracts carry only `{ message, expectedIndexTreeSha, expectedHeadSha }` (`@app/src/hooks/source_control/source_control_types.ts:35-44`, `@app/src/shared/protocol_source_control.ts:104-121`, `@crates/im_agent/src/protocol/commands_source_control.rs:93-120`). They carry no intended ref/branch, merge parent set, exact retained index object/token, or outside-root acknowledgement.

The backend re-reads live status, compares only index tree and HEAD sha, then invokes ordinary `git commit` against the live checkout and live index (`@crates/im_agent/src/source_control/actions_commit.rs:39-95`). When hooks produce a different tree, the finalizer reduces authorization from exact content identity to a path allowlist: any changed path already named by the reviewed status is accepted regardless of its new blob (`@crates/im_agent/src/source_control/actions_commit_retract.rs:23-65`, `@crates/im_agent/src/source_control/actions_commit_retract.rs:121-139`).

The outside-root confirmation exists only as UI-local state. The modal stores a count and calls `commit(pendingCommit)`, but the serializer and Rust request discard that fact (`@app/src/components/source_control/source_control_column.tsx:64-71`, `@app/src/components/source_control/source_control_column.tsx:120-135`, `@app/src/components/source_control/source_control_column.tsx:232-241`, `@app/src/hooks/source_control/source_control_commands.ts:23-32`). The agent infers acknowledgement merely because its re-read reports a non-zero outside-root count.

### Verified failure constructions

1. **Wrong ref at identical HEAD/tree.** Review `main`, switch to `other` while both point at the same HEAD and retain the same index tree, then execute. Both implemented comparisons pass and the commit advances `other`, not `main`. Probe: `commit_wrong_branch_same_head_probe.txt`.
2. **Merge-parent substitution.** Replace `MERGE_HEAD` with a different commit having the same tree while HEAD and the index tree stay unchanged. Both comparisons pass; the resulting merge records the unreviewed second parent. Probe: `commit_merge_parent_substitution_probe.txt`.
3. **Arbitrary same-path hook rewrite.** A hook replaces the entire contents of `reviewed.txt`. The resulting tree differs, but `diff-tree` reports only the already reviewed path, so the path allowlist accepts the substituted blob. Probe: `allowed_path_hook_rewrite_probe.txt`.
4. **Message substitution.** `prepare-commit-msg` replaces the frozen message. Tree finalization accepts because it checks no commit-message identity. Probe: `commit_message_hook_substitution_probe.txt`.

### Consequence

The current implementation binds two observations, not the operation. It can produce the wrong history while claiming the reviewed snapshot owned the effect. The design's new allowance for same-path hook changes (`@docs/design/source_control_design.md:74-77`) is also a direct contradiction of the required closure, which said a changed hook tree must be refused before publication or presented as a new snapshot (`@docs/reports/source_control_fix_layer_review_20260903.md:102-105`). This P0 remains open.

### Required end state

Implement the retained transaction already specified by the closure report: private immutable index/tree and merge-parent state, explicit intended ref and outside-effect acknowledgement, frozen message, hook execution inside that transaction, and CAS publication of the intended ref. Do not add more live preflight fields or another post-hoc allowlist.

## P0-2 — Retraction occurs after publication and cannot truthfully return `notApplied`

### Violated invariant

Current canon says an unreviewed hook expansion is retracted “before it is visible” and returns `effect: notApplied` (`@docs/design/source_control_design.md:77`, `@docs/architecture/source_control_architecture.md:101-139`, `@docs/changelog.md:15`). The effect contract says `notApplied` is reserved for proof that nothing happened (`@app/src/shared/protocol_source_control.ts:139-149`).

### What the tree does

The backend first runs normal `git commit`, which updates the checked-out ref and executes commit hooks, and only after it returns reads the resulting tree (`@crates/im_agent/src/source_control/actions_commit.rs:80-95`, `@crates/im_agent/src/source_control/actions_commit_retract.rs:23-65`). On rejection, it performs only a CAS `git update-ref` back to the old HEAD and returns `MutationEffect::NotApplied` (`@crates/im_agent/src/source_control/actions_commit_retract.rs:158-193`).

A ref move cannot undo the already-written index, reflog, commit object, `post-commit` hook effects, filesystem/network effects from arbitrary hooks, or observation by another ref watcher/process.

### Reproduced failure

`commit_retraction_probe.txt` records:

- the branch moved to the new commit and a `post-commit` hook observed that SHA;
- retraction later moved the branch back;
- the index remained at the hook-expanded published tree, with both reviewed and outside files staged against the old HEAD;
- the reflog retained the transient commit.

That repository is not in the state it would have had if the action were never applied. `notApplied` is therefore factually false even when the CAS retraction succeeds.

### Consequence

A hook can publish externally during the interval, and the UI is instructed to treat a materially changed repository as untouched. This is the exact compensation pattern ADR-007 rejects; it is not an effect-boundary owner.

### Required end state

Reject before ref publication. Build the candidate commit privately, validate its exact resulting transaction state, and CAS the reviewed ref only after authorization. Once ordinary `git commit` has published, the strongest honest error is `unknown` unless complete final-state comparison proves every relevant effect—not merely the branch ref—was restored.

## P0-3 — Discard can still destroy content newer than the reviewed snapshot

### Violated invariant

The PRD requires a stale discard to refuse rather than overwrite a file recreated or changed after review (`@docs/prd.md:144-148`). The closure report required typed target states, a regular-file digest, symlink target or refusal, atomic claim, no-replace installation, and no overwrite after claim (`@docs/reports/source_control_fix_layer_review_20260903.md:98-107`).

### What the tree does

A “stamp” is only file length plus two encodings of the same mtime (`@app/src/shared/protocol_source_control.ts:25-35`, `@crates/im_agent/src/protocol/commands_source_control.rs:56-82`). `stamp_of` never reads file bytes or a digest (`@crates/im_agent/src/source_control/status_stamp.rs:27-45`). Symlinks, directories, permission failures, and rename endpoints collapse to no assertion.

For an existing file, discard renames it to `.git/intermediary-discard/<opId>/claimed`, then compares that metadata stamp (`@crates/im_agent/src/source_control/actions_discard_claim.rs:42-68`). For a missing target it checks absence once and later invokes `git restore`; for a claimed tracked target it leaves the original path empty and later invokes `git restore` (`@crates/im_agent/src/source_control/actions_discard_claim.rs:126-136`, `@crates/im_agent/src/source_control/actions_discard_target.rs:41-58`, `@crates/im_agent/src/source_control/actions_discard_target.rs:62-85`, `@crates/im_agent/src/source_control/actions_discard_target.rs:102-125`). Neither path uses a no-replace install or revalidates the destination at the write boundary.

The startup sweep assumes every file named `claimed` was already verified, but verification occurs *after* the rename. It deletes every operation directory without `unrestored` (`@crates/im_agent/src/source_control/discard_quarantine.rs:53-105`). Rollback and put-back use ordinary replacing `std::fs::rename`, not a no-replace primitive (`@crates/im_agent/src/source_control/actions_discard_claim.rs:85-101`, `@crates/im_agent/src/source_control/actions_discard_claim.rs:148-162`).

### Verified failure constructions

1. **Metadata collision:** different contents with the same length and restored nanosecond mtime produce exactly equal accepted stamps. Probe: `discard_stamp_collision_probe.txt`.
2. **Missing-target TOCTOU:** absence check passes, a newer file is created, then `git restore` overwrites it. Probe: `discard_missing_to_restore_toctou_probe.txt`.
3. **Post-claim TOCTOU:** after the old file is claimed, a newer file appears at the original path, then `git restore` overwrites it. Probe: `discard_recreate_after_claim_toctou_probe.txt`.
4. **Crash-before-verification:** newer contents are renamed to `claimed`, the process crashes before stamp comparison, and the next startup sweep deletes the only copy. Probe: `discard_crash_before_verify_probe.txt`.
5. **Replacing rollback:** on platforms where rename replaces the destination, a file recreated before rollback is overwritten by the quarantined mismatch. Probe: `discard_rollback_no_replace_violation_probe.txt`.

### Consequence

This is direct user-data loss under the product's declared multi-writer model. The quarantine mechanism is useful, but it lacks the content identity, transaction phase/journal, and no-replace publication boundary that make a quarantine safe. This P0 remains open.

### Required end state

Use a typed retained target identity (digest/type/symlink target or refuse), a same-filesystem claim with durable phase state distinguishing unverified from verified-authorized contents, and no-replace restoration/publication. A crash sweep may delete only claims durably marked verified and authorized; it must preserve unknown/unverified claims.

## P0-4 — Emergency process-tree ownership is still process-local and disappears with the agent

### Violated invariant

The closure report required the operation coordinator/guardian to remain alive until every mutation reached finality and to own complete emergency process-tree termination before agent or distro termination (`@docs/reports/source_control_fix_layer_review_20260903.md:148-156`). ADR-009 requires bounded tasks tied to a lifecycle owner (`@docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md:19-39`).

### What the tree fixed

The authenticated agent path is substantially improved: it stops admission, waits up to 450 seconds, and invokes its in-process Git-tree registry before scheduling exit (`@crates/im_agent/src/server/shutdown.rs:61-143`). Normal runner timeouts can terminate a Unix process group or Windows Job Object.

### What remains unowned

The only Job/process-group handle and registry live inside the disposable agent process. `GitProcessTree::drop` explicitly unregisters and terminates nothing (`@crates/im_bundle/src/git_capture/command_tree.rs:36-39`, `@crates/im_bundle/src/git_capture/command_tree.rs:86-103`). The Windows job deliberately has no kill-on-close behavior (`@crates/im_bundle/src/git_capture/command_job.rs:10-18`, `@crates/im_bundle/src/git_capture/command_job.rs:73-82`). Job creation failure silently returns `None` and Git is allowed to run without a tree owner (`@crates/im_bundle/src/git_capture/command_job.rs:41-53`), contrary to ADR-008's no-silent-recovery rule for required invariants. Assignment occurs only after the child has spawned and attach failure is also accepted (`@crates/im_bundle/src/git_capture/command.rs:158-180`, `@crates/im_bundle/src/git_capture/command_tree.rs:169-195`).

When no clean shutdown acknowledgement arrives, the supervisor waits up to 480 seconds and then falls through to reconciliation/kill (`@src-tauri/src/lib/agent/supervisor/graceful_stop.rs:11-19`, `@src-tauri/src/lib/agent/supervisor/graceful_stop.rs:129-155`). That path calls `Child::kill` on the direct agent process only (`@src-tauri/src/lib/agent/supervisor/managed_processes.rs:13-38`, `@src-tauri/src/lib/agent/supervisor/managed_processes.rs:98-165`, `@src-tauri/src/lib/agent/supervisor/process_kill.rs:16-50`).

### Concrete failure construction

Run a long Git mutation whose hook/helper remains active, then wedge, suspend, crash, or otherwise make the agent unable to execute `finalize_shutdown`. The supervisor receives no truthful ack and eventually kills the agent. The in-process registry and Job handle disappear; because the job is not kill-on-close and the supervisor owns no duplicate tree handle, Git/hooks/helpers can continue unowned. The same ownership hole exists for a Unix process group when the agent is externally killed.

### Consequence

The normal shutdown route is fixed, but the emergency route—the route process-tree ownership exists to make safe—still violates the stated invariant. The `docs/known_issues.md` retirement and changelog closure are premature.

### Required end state

Put finality in a lifecycle owner outside the agent process that may be killed: the supervisor/guardian must own the agent's complete descendant boundary or a duplicated per-operation tree handle from spawn through terminal receipt. Failure to create/attach that owner must refuse mutation admission, not silently downgrade.

## P1-5 — Confirmed WSL action errors never clear the outstanding-mutation ledger

The host tracks every forwarded `SourceControlAction` before send and clears it only when `outcome.is_ok()` (`@crates/im_host_agent/src/wsl/wsl_backend_client.rs:133-185`). A real backend `ResponseEnvelope::Error`—including a fully completed `SOURCE_CONTROL_STATE_CHANGED`, `GIT_NOTHING_TO_COMMIT`, or other application refusal—is mapped to `Err(AgentError)` (`@crates/im_host_agent/src/wsl/wsl_backend_messages.rs:54-76`). Therefore a confirmed terminal response remains “outstanding” forever.

If the WSL backend later goes offline during close/restart, shutdown treats that stale id as proof a mutation may still be running and retries until the 450-second emergency deadline (`@crates/im_host_agent/src/server/shutdown_dispatch.rs:74-159`). A normal, already-finished refusal can thus turn a later close into a 7.5-minute unknown-finality path.

The transport result must distinguish **confirmed application error** from **unconfirmed transport failure**. Both success and confirmed error clear the ledger; only a request that may still be running remains outstanding.

## P1-6 — Discard's `.git` quarantine is not atomic for cross-volume linked worktrees

The operation puts quarantine under the physical Git directory (`@crates/im_agent/src/source_control/actions_discard.rs:41-42`, `@crates/im_agent/src/source_control/discard_quarantine.rs:26-28`) and uses a single `std::fs::rename` from the worktree path (`@crates/im_agent/src/source_control/actions_discard_claim.rs:55-68`). A linked worktree can live on a different Windows drive/filesystem from the main repository's `.git/worktrees/<name>` admin directory—an explicitly supported worktree shape (`@docs/prd.md:73`, `@docs/design/source_control_design.md:68`). Cross-filesystem rename is not atomic and fails with `EXDEV`/the platform equivalent.

`cross_filesystem_discard_probe.txt` reproduced different source and git-dir device ids and `Invalid cross-device link`; the source survived, but Discard could not operate. The claim bytes must live on the worktree's filesystem, with durable operation metadata linking that claim to the physical Git owner.

## P2-7 — Active canon now records false closure and a stale ADR-000 exception

`@docs/changelog.md:14-15` and `@docs/roadmap.md:51` say both P0 owners are closed. `@docs/design/source_control_design.md:74-108` and `@docs/architecture/source_control_architecture.md:101-139` encode the current post-hoc path allowlist, ref retraction, metadata stamp, sweep, and process-local finalizer as the intended architecture. The P0 reproductions above disprove those claims. Because Intermediary treats docs as active agent memory, this is not harmless release-note optimism: it directs subsequent work to preserve the wrong model.

The ADR-000 deferral is also stale. ADR-000 requires folders with 10+ siblings to split by concern (`@docs/compliance/adr_000_modular_file_discipline.md:19-27`). `@docs/known_issues.md:64-72` records 15 direct Rust source-control siblings and defers the split until the tree-decoration/conflict/image-diff follow-up lands. In this bundle that follow-up is already present, while `crates/im_agent/src/source_control/` now contains 33 direct modules (22 production plus 11 test modules); the frontend folder remains at 10. The recorded count and reason are no longer true.

Do not rewrite canon around the current implementation again. Keep both P0s open until the retained transaction and external finality owner actually exist; then update design/architecture/changelog from witnessed source behavior. The folder split can be performed alongside that owner rewrite rather than as a separate campaign.

## Improvements that should be retained

These are real advances and are not reopened by the findings above:

- Copy rows act on the destination only; rename endpoint display/confirmation is no longer flattened into copy provenance.
- Commit preflight compares stable index-tree and HEAD identity, and torn status reads refuse rather than authorizing an empty identity.
- Mutation errors carry explicit effect certainty, and the UI reconciles unknown outcomes.
- Discard is processed per target with operation-owned claims and reports partial batches as unknown.
- Agent shutdown admission/drain behavior and timeout layering are materially stronger.
- Git process-tree abstraction, Windows Job primitives, physical-git-dir locking, tracked-file watcher overrides, conflict prominence, and image diffs are sound foundations for the corrected owners.

## Evidence files

The attached evidence archive contains the disposable probe outputs cited above:

- `commit_wrong_branch_same_head_probe.txt`
- `commit_merge_parent_substitution_probe.txt`
- `allowed_path_hook_rewrite_probe.txt`
- `commit_message_hook_substitution_probe.txt`
- `commit_retraction_probe.txt`
- `discard_stamp_collision_probe.txt`
- `discard_missing_to_restore_toctou_probe.txt`
- `discard_recreate_after_claim_toctou_probe.txt`
- `discard_crash_before_verify_probe.txt`
- `discard_rollback_no_replace_violation_probe.txt`
- `cross_filesystem_discard_probe.txt`
