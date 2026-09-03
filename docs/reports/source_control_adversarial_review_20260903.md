# Intermediary Current-Tree Adversarial Review — Source Control

Updated on: 2026-09-03  
Review authority: `intermediary_context_20260903_092146_c8e3733.zip`  
Captured HEAD: `c8e3733fba861f892906cae49b016f9c605af2b8`  
Candidate index tree: `113136ffce347d6c598dcfc83db7009f18b55a9e`  
Verdict: **REJECT the current Source Control completion/release claim.**

## Review boundary

The bundle is complete for its selected tree: 137 selected changed paths, zero changed paths omitted, and complete Git evidence. The repository and bundle selection are both dirty. `scripts/build` is explicitly outside the bundle selection, so this review does not claim to validate the unchanged installer pre-build implementation. Everything else outside the bundle was treated as stale. See @BUNDLE_MANIFEST.json:1 and @BUNDLE_GIT_STATUS.txt:4-16.

I followed @docs/guide.md:8-54 into the Source Control design, architecture, PRD, roadmap, changelog, known issues, and applicable ADRs. There is no Source Control implementation/review report indexed under Reports; the only indexed report concerns bundle global excludes (@docs/guide.md:50-54).

This is a product/correctness review. Test and gate status were not used as acceptance evidence. Where a static defect depended on Git semantics, I reproduced it in a disposable repository; the captured outputs are bundled separately as review evidence.

## Executive verdict

The right-rail UI, root-routed host/WSL dispatch, shared Git facade, and typed command families are directionally coherent. The feature nevertheless does **not** yet own the noun it claims to own: a safe Git mutation transaction.

Today, UI intent, path identity, physical Git worktree identity, mutation serialization, child-process lifetime, transport lifetime, status reconciliation, outcome certainty, and app shutdown are separate lifecycles. They do not compose into one authoritative operation. That missing owner produces three P0 failures:

1. a copy-row discard can erase unrelated edits in the source file;
2. a commit can absorb newly staged outside-root files that the user never confirmed;
3. closing or restarting Intermediary can kill the agents while their supposedly non-cancellable Git mutation is still running.

The roadmap says Source Control is complete, and the 0.1.14 changelog says it shipped and that timeout handling prevents leftover index locks (@docs/roadmap.md:39-51; @docs/changelog.md:3-8). Those claims are not supported by the current tree. The feature should return to active work, and those closeout statements should not remain authoritative until the transaction architecture below is implemented and witnessed.

## Prioritized findings

| Priority | Finding | Consequence |
| --- | --- | --- |
| **P0-1** | Copy/rename records are flattened into an undifferentiated path list | Discarding one copied row can erase an unrelated source edit; cross-root renames can commit outside-root changes without warning |
| **P0-2** | Mutations are not bound to the status/index state the user reviewed | Commit can silently absorb newly staged files; discard can destroy newer content than the user confirmed |
| **P0-3** | App stop/restart kills the agent instead of draining active Git transactions | Closing the app mid-commit can orphan or kill Git and cannot guarantee index-lock cleanup |
| **P1-4** | The shared runner can wait forever after the direct Git process has already exited | A hook/helper retaining stdout/stderr can hold the repo mutation mutex forever after the commit landed |
| **P1-5** | The documented timeout ladder is arithmetically false and transport timeout erases the final response | WSL actions time out while still running; reconciliation can read an intermediate state and call it ready |
| **P1-6** | `GIT_*` is treated as proof that a mutation did not apply | A timed-out commit that changed HEAD is reported as failed; retry can duplicate it |
| **P1-7** | CHANGES bulk staging crosses the MERGE CHANGES boundary, while `committable` ignores unresolved conflicts | One click can mark conflict files resolved with conflict markers staged; COMMIT is enabled when Git refuses it |
| **P1-8** | Mutation serialization is keyed by `repoId`, not by the physical Git index/worktree | Two configured roots for one worktree can mutate the same index concurrently |
| **P2-9** | The event-driven watcher suppresses status-relevant tracked files and Git metadata | SOURCE can remain stale while focused even though Git status changed |
| **P2-10** | Several documented wire/UI/structure invariants are already false | Empty path actions succeed as no-ops, deleted rows still open diffs, counts misstate files, host reads are not cancellable, and new folders violate ADR-000 organization |

---

## P0-1 — Relational Git changes are modeled as a bag of paths, causing destructive copy semantics and false subroot boundaries

### Violated contract

A row action is presented as an action on the visible row. Discard copy names one file and says that file will be replaced from the index (@app/src/components/source_control/source_control_column.tsx:41-45, @app/src/components/source_control/source_control_column.tsx:170-179). The design likewise specifies discard on “a CHANGES row” and requires a warning before a commit carries staged paths outside a configured subroot (@docs/design/source_control_design.md:60-63).

### What the tree does

The status projector preserves an `originalPath` for both rename and copy records (@crates/im_agent/src/source_control/status_project.rs:148-165). The UI then expands **both** kinds into `[originalPath, path]` for every stage, unstage, and discard action, under the assumption that both are “the whole rename” (@app/src/components/source_control/source_control_column.tsx:36-39, @app/src/components/source_control/source_control_column.tsx:67-74, @app/src/components/source_control/source_control_column.tsx:170-179).

The backend has no relational identity left. It reclassifies every supplied string independently and restores/removes each matching path (@crates/im_agent/src/source_control/actions_discard.rs:21-43, @crates/im_agent/src/source_control/actions_discard.rs:53-74).

For a copy, the original is provenance, not the other half of a move. Therefore a copy-row action is allowed to mutate an unrelated, separately modified source file.

### Reproduced failure

`copy_discard.txt` captures a Git copy record for `copy.txt` with original `source.txt`, while `source.txt` also has a distinct unstaged edit. The UI/backend-equivalent discard restores both paths. Result:

- `SOURCE EDIT THAT MUST SURVIVE` disappears from `source.txt`;
- `copy.txt` becomes empty/intent-to-add;
- the confirmation named only `copy.txt`.

This is direct user-data loss.

### The same false model breaks subroot safety

Projection scopes the record only by `record.current`; an outside-root original is silently dropped, and only records whose **current** path is outside can increment `stagedOutsideRoot` (@crates/im_agent/src/source_control/status_project.rs:73-112, @crates/im_agent/src/source_control/status_project.rs:143-146).

`cross_root_rename.txt` demonstrates both directions for a configured `sub/` root:

- outside → inside: the row appears inside, its outside source is dropped, and no outside-root warning is possible, although commit deletes the outside source;
- inside → outside: the entire visible inside deletion disappears and only a generic omitted count remains.

### Required correction

Do not patch `entryPaths`. Replace the path-bag command contract with a structured Git change identity owned by the backend:

- distinguish `copy` destination from copy provenance;
- preserve both rename endpoints and each endpoint’s configured-root scope;
- resolve the identity against fresh status under the mutation transaction;
- enumerate every destructive target in confirmation copy;
- count/confirm any commit effect outside the configured root, including the outside endpoint of a cross-root rename.

This is the contract-level correction required by ADR-007 (@docs/compliance/adr_007_architecture_first_execution.md:10-22).

---

## P0-2 — Mutations silently retarget to whatever Git state exists at execution time

### Violated contract

The product explicitly coexists with terminal Git and coding agents. The user must know what a destructive action or commit will affect. For a configured subroot, staged outside-root paths require a warning and explicit confirmation (@docs/design/source_control_design.md:56-60; @docs/prd.md:138-146).

### What the tree does

The UI makes the confirmation decision from its current status snapshot, then sends only the commit message (@app/src/components/source_control/source_control_column.tsx:87-97, @app/src/components/source_control/source_control_column.tsx:183-191). The wire action contains no observed index tree, status generation, target identity, or acknowledgement token (@app/src/shared/protocol_source_control.ts:61-75).

The backend re-reads status, but uses that read only to decide whether anything is committable; it never checks that the current index is the index the user reviewed or confirmed (@crates/im_agent/src/source_control/actions.rs:127-147). A fresh read without an expected-state comparison is silent retargeting, not safety.

### Reproduced failure

`commit_snapshot_race.txt` records this sequence for configured root `sub/`:

1. the UI-observed index contains only `sub/a.txt`, with no staged outside-root path;
2. an external actor stages `outside.txt` before the commit executes;
3. the index tree changes;
4. the commit includes both `sub/a.txt` and `outside.txt`, with no outside-root confirmation.

Discard has the same time-of-check/time-of-use defect. Its modal confirms the displayed row, but the backend’s later status classification and restore/remove operate on the newest contents at that path (@crates/im_agent/src/source_control/actions_discard.rs:15-43). An agent can modify or replace the file after the user saw it and before discard reaches the backend; the newer content is what gets destroyed.

### Required correction

Every mutating request must carry an optimistic precondition derived from the displayed snapshot:

- commits carry the observed index-tree identity plus an explicit acknowledgement of the observed outside-root effects;
- row actions carry a stable change identity and expected area/state;
- discard carries a target/content generation sufficient to refuse when the file changed after confirmation;
- the backend returns a typed `state changed` result with fresh status instead of acting on a different tree.

The repository already understands a read-only candidate index-tree identity for bundle evidence; Source Control needs the same class of authority, attached to the user’s operation rather than merely displayed after the fact.

---

## P0-3 — “Non-cancellable mutations” end at socket cancellation, not at application/process shutdown

### Violated contract

The design says mutations are deliberately non-cancellable because killing Git can leave `.git/index.lock`, and acceptance explicitly requires no index lock after closing the app mid-commit (@docs/design/source_control_design.md:68-76, @docs/design/source_control_design.md:85-93). ADR-009 requires explicit cancellation and bounded task ownership (@docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md:19-30, @docs/compliance/adr_009_rust_concurrency_and_io_boundary_rules.md:34-39).

### What the tree does

At the request layer, actions are indeed `Passive`; disconnect cancellation does not stop them (@crates/im_agent/src/server/connection/request_cancellation.rs:13-40). But the request task is detached from the connection and owns the mutation lock/Git call (@crates/im_agent/src/server/connection.rs:162-193, @crates/im_agent/src/server/connection/source_control_commands.rs:58-81).

The application lifecycle has no corresponding operation owner or drain:

- browser unload fire-and-forgets `stop_agent` (@app/src/hooks/agent/use_agent_shutdown.ts:7-19);
- Tauri exit synchronously calls supervisor shutdown (@src-tauri/src/lib/mod.rs:116-143);
- restart and stop immediately stop host, then WSL processes (@src-tauri/src/lib/agent/supervisor/lifecycle.rs:35-48);
- live managed agents are terminated with `Child::kill()` and a five-second reap loop (@src-tauri/src/lib/agent/supervisor/managed_processes.rs:73-121, @src-tauri/src/lib/agent/supervisor/process_kill.rs:16-50);
- after stopping WSL, exit may terminate the whole distro if it appears idle (@src-tauri/src/lib/agent/supervisor/shutdown.rs:13-35, @src-tauri/src/lib/agent/supervisor/shutdown.rs:50-80).

Both agents only own a `ctrl_c` accept-loop break; they expose no authenticated “stop accepting mutations, drain active operations, then exit” protocol (@crates/im_agent/src/server/ws_server.rs:47-85; @crates/im_host_agent/src/server/ws_server.rs:50-90).

Killing the agent is not equivalent to gracefully stopping the Git process group. On Windows, child processes are not put in a kill-on-close job by this code. On Unix, the Git process group belongs to the runner, but an external kill of the agent never invokes the runner’s graceful stop path. The acceptance claim therefore has no owner.

### Required correction

One authenticated shutdown path must span UI → host agent → WSL agent → active Git operations:

1. stop admission of new mutations;
2. report and drain active operation IDs;
3. let ordinary mutations reach a known terminal state;
4. if an emergency bound is exceeded, terminate the owned process tree and record outcome certainty plus any lock residue;
5. only then kill agents or terminate WSL.

`Restart Agent` must use the same drain path. A separate explicit emergency-stop action may remain destructive, but routine close/restart cannot be that action.

---

## P1-4 — A successful direct Git exit can still wedge the mutation lock forever

### Violated contract

The architecture claims that process-group ownership plus bounded reader joins prevent a hook or `ssh` process holding inherited pipes from wedging a repo lock (@docs/architecture/source_control_architecture.md:67-69).

### What the tree does

The timeout/cancel loop bounds only the lifetime of the **direct** Git child (@crates/im_bundle/src/git_capture/command.rs:201-221). If that child exits normally, the runner switches to `Streams::collect(Wait::UntilDone)` (@crates/im_bundle/src/git_capture/command.rs:223-247). `UntilDone` is an unbounded blocking channel receive, waiting for pipe EOF (@crates/im_bundle/src/git_capture/command_child.rs:29-49, @crates/im_bundle/src/git_capture/command_child.rs:69-79).

The Unix process group is killed only through `stop_child`; normal child completion does not invoke it. On Windows, process-group creation and group termination are no-ops in this runner (@crates/im_bundle/src/git_capture/command_stop.rs:13-29, @crates/im_bundle/src/git_capture/command_stop.rs:71-79).

### Reproduced failure

`pipe_hold.txt` uses a post-commit hook that exits after spawning a background descendant retaining the inherited pipe. The direct Git process exits successfully in 0.011 s and HEAD contains the commit, but EOF arrives only after the descendant exits four seconds later. A long-lived descendant makes `UntilDone` unbounded.

Because `run_source_control_action` holds the mutation mutex until the runner returns, the repo then rejects every later mutation even though the commit already landed (@crates/im_agent/src/source_control/mod.rs:81-91).

### Required correction

The transaction owner must own a process **tree** on every platform (Unix process group; Windows Job Object or equivalent), and pipe drain must remain bounded after direct-child exit. If inherited pipes fail to close:

- terminate the surviving tree;
- preserve the direct child’s exit status;
- return an operation result that separates Git’s effect from cleanup/transport failure.

---

## P1-5 — Timeout nesting is false, and the first reconciliation read can race the still-running mutation

### Violated contract

Both design and architecture explicitly claim that outer timeout tiers cover the summed worst case of all Git commands in one request (@docs/design/source_control_design.md:68-76; @docs/architecture/source_control_architecture.md:72-75).

### Actual budgets

Agent-side bounds are 20 s read, 60 s index, 120 s commit, and 180 s remote (@crates/im_agent/src/source_control/runner.rs:27-35). A status capture is sequential prefix + porcelain status + cached-diff probe + optional `MERGE_HEAD` probe, up to 80 s (@crates/im_agent/src/source_control/status.rs:14-56).

The resulting request envelopes are therefore approximately:

| Action | Agent-side possible sequence | Possible total | Host→WSL bound | UI bound |
| --- | --- | ---: | ---: | ---: |
| status | 4 × 20 s reads | 80 s | 90 s | 120 s |
| stage / unstage | 60 s mutation + 80 s status | 140 s | **120 s** | 150 s |
| discard | 80 s pre-status + 60 s restore + 60 s reset + file removal + 80 s post-status | **280 s+** | **120 s** | **150 s** |
| commit (resolved merge case) | 80 s pre-status + 120 s commit + 20 s HEAD + 80 s post-status | **300 s** | **240 s** | **300 s** |
| push | 80 s pre-status + optional 20 s remotes + 180 s push + 80 s post-status | **360 s** | **300 s** | **360 s** |
| pull | 180 s pull + 80 s post-status | 260 s | 300 s | 360 s |

The outer constants and their asserted rationale are in @crates/im_host_agent/src/wsl/wsl_backend_client.rs:20-33, @crates/im_host_agent/src/wsl/wsl_backend_client.rs:131-146, and @app/src/lib/agent/agent_request_timeouts.ts:6-40.

### Why this is more than a timeout-message defect

When the host→WSL tier expires, the host removes the pending response and sends a cancel envelope (@crates/im_host_agent/src/wsl/wsl_backend_client.rs:108-123; @crates/im_host_agent/src/wsl/wsl_backend_connection.rs:54-79). The WSL action is `Passive`, so physical work continues while its final response has been discarded (@crates/im_agent/src/server/connection/request_cancellation.rs:17-40).

The UI classifies the transport failure as unknown and immediately requests status (@app/src/hooks/source_control/use_source_control_state.ts:153-172). Status reads do not take the mutation mutex, while actions do (@crates/im_agent/src/source_control/mod.rs:62-91). The first successful read clears reconciliation and marks the view ready (@app/src/hooks/source_control/use_source_control_state.ts:81-93), even if it observed discard between restore/reset/removal or commit while a hook still runs.

### Required correction

Replace stacked request timers with an agent-owned operation lifecycle:

- one operation ID and operation-level deadline;
- transport timeout detaches the caller but does not erase the operation/result;
- status either waits on the physical-worktree mutation barrier or explicitly returns `mutation in progress { operationId }`;
- UI asks for operation finality rather than inferring it from an unrelated status snapshot;
- outer timeouts protect transport responsiveness only, not physical Git finality.

---

## P1-6 — Error-code prefix is incorrectly used as mutation outcome certainty

### Violated contract

A timeout after a mutation starts is not proof that the mutation failed. The architecture correctly says socket/transport loss is unknown, but the same uncertainty exists when Git crosses its effect boundary and then times out in a hook/helper.

### What the tree does

The runner maps a timed-out mutation with no leftover index lock to `GIT_TIMEOUT` (@crates/im_agent/src/source_control/runner.rs:213-237). The frontend classifies **every** `GIT_*` code as a definitive rejection and displays `ACTION FAILED`; only non-`GIT_*` failures enter reconciliation (@app/src/hooks/source_control/source_control_failures.ts:6-17, @app/src/hooks/source_control/source_control_failures.ts:27-35; @app/src/components/source_control/source_control_copy.ts:42-48).

### Reproduced failure

`timeout_landed.txt` captures a commit whose post-commit hook outlives the command bound:

- timeout exit: 124;
- HEAD changed to the new commit;
- subject is `timed-out-commit`;
- no `index.lock` remains.

Under the current mapping, that user sees COMMIT FAILED. Retrying can create a second commit.

### Required correction

Mutation results need an explicit effect-certainty field independent of error namespace:

- `notApplied` only when the backend proves no effect;
- `applied` when final state is proven;
- `unknown` after timeout, forced stop, process-tree cleanup failure, or transport loss unless state comparison proves the result.

The operation journal described above should resolve uncertainty using observed before/after HEAD/index/remote identities. Never infer certainty from `GIT_`.

---

## P1-7 — Section staging violates the MERGE CHANGES boundary, and commit readiness is not Git’s answer

### Violated contract

The product presents STAGED CHANGES, CHANGES, and MERGE CHANGES as distinct sections, with per-file/per-section operations (@docs/design/source_control_design.md:18-20, @docs/design/source_control_design.md:57-59; @docs/design/intermediary_ui_overhaul_design.md:219-224). That separation must be behavioral, not decorative.

### What the tree does

The CHANGES section’s `+` calls `stageAll` (@app/src/components/source_control/source_control_body.tsx:90-103). `stageAll` sends `mode: "all"`, which runs `git add -A -- .` across the entire configured root (@app/src/components/source_control/source_control_column.tsx:67-74; @crates/im_agent/src/source_control/actions.rs:81-99). Conflict paths are not excluded. MERGE CHANGES, meanwhile, has no section-wide action at all (@app/src/components/source_control/source_control_body.tsx:105-112).

`status.committable` is described as whether Git would accept a commit, but it is only `(cached diff exists) OR (MERGE_HEAD exists)` (@crates/im_agent/src/source_control/status.rs:39-56; @crates/im_agent/src/protocol/responses_source_control.rs:54-69). The UI has no conflict guard when enabling COMMIT (@app/src/components/source_control/source_control_column.tsx:80-88).

### Reproduced failures

`stage_changes_resolves_conflicts.txt` starts with:

- `UU conflict.txt` in MERGE CHANGES;
- `ordinary.txt` modified in CHANGES;
- conflict markers in `conflict.txt`.

The CHANGES bulk implementation (`git add -A -- .`) stages both files, removes every unmerged index entry, and leaves the conflict markers staged. One click on the ordinary CHANGES section has silently declared the conflict resolved.

`unresolved_merge.txt` shows the readiness defect separately: the current heuristic reports `committable=true`, while `git commit --dry-run` exits 1 with “You have unmerged paths.”

### Required correction

- Section bulk actions must send the explicit identities belonging to that displayed section and snapshot, not pathspec `.`.
- MERGE CHANGES needs its own explicit resolution-stage operation if section-wide staging is intended.
- Commit readiness must expose structural reasons: no unmerged records anywhere in the physical worktree, plus an index/operation state that can form a commit.
- Do not label this “Git’s answer”; hooks, identity, and unresolved state are separate dimensions.

---

## P1-8 — The lock protects UI repo IDs, not Git’s physical mutation authority

### Violated contract

The intended invariant is serialization of mutations that share a Git index/worktree, not merely requests that share a UI label. ADR-007 forbids a parallel lifecycle that fails to protect the actual authority (@docs/compliance/adr_007_architecture_first_execution.md:16-22).

### What the tree does

`SourceControlLocks` creates one mutex per `repoId` string (@crates/im_agent/src/source_control/locks.rs:7-30), and every action selects it by command repo ID (@crates/im_agent/src/source_control/mod.rs:81-91). The UI rejects only exact duplicate configured-root keys; it permits a repository root and a subdirectory as two entries (@app/src/components/add_repo_button.tsx:47-61). Persisted config validates unique IDs, not unique/canonical Git worktrees (@src-tauri/src/lib/config/types/validation.rs:57-71).

The design explicitly supports configured roots below the Git top level, so this is reachable by ordinary configuration.

### Reproduced authority collision

`repo_id_lock_alias.txt` resolves the Git index for `/repo` and `/repo/sub`: both point to the same `/repo/.git/index`. Two repo IDs therefore obtain two mutexes while mutating one physical index. Concurrent stage/commit operations can race, hit `index.lock`, or commit each other’s staged set.

### Required correction

Resolve and canonicalize a physical Git mutation identity before lock acquisition—at minimum the worktree gitdir/index path, with linked worktrees remaining distinct. Parent/subroots and path aliases that share one index must share one coordinator and status barrier.

---

## P2-9 — Event-driven refresh intentionally ignores paths that can change Git status

### Violated contract

The PRD says SOURCE shows the active repo/worktree “as Git sees it” and stays current through watcher events without polling (@docs/prd.md:136-146).

### What the tree does

The detector declares that `node_modules` and `target` “never hold tracked files,” then unconditionally suppresses them and every configured `ignoreGlob` for worktree events (@crates/im_agent/src/repos/source_control_watch/detector.rs:10-18, @crates/im_agent/src/repos/source_control_watch/detector.rs:42-61, @crates/im_agent/src/repos/source_control_watch/detector.rs:74-96). Git has no such rule. The repository’s own known-issue history explicitly records legitimate source directories named `target` as a supported case (@docs/known_issues.md:77-82).

The Git metadata allowlist watches index/HEAD/selected refs but not `.git/config`, worktree config, or `.git/info/exclude` (@crates/im_agent/src/repos/source_control_watch/detector.rs:16-25, @crates/im_agent/src/repos/source_control_watch/detector.rs:105-110). Those files can change upstream display, remote behavior, or which untracked paths status reports.

Because architecture explicitly has no interval polling (@docs/architecture/source_control_architecture.md:38-43), a tracked file under an excluded path—or a status-relevant config change while the window remains focused—can leave SOURCE stale indefinitely until an unrelated accepted event occurs.

### Required correction

Make suppression Git-aware, not basename-aware:

- maintain a tracked-path set/trie from the index;
- always emit for tracked paths, even under structural/user excludes;
- suppress only known-untracked noise;
- refresh that authority on index changes;
- include the Git config/info metadata that changes the projected status contract.

This preserves the no-polling intent without a timing workaround.

---

## P2-10 — Smaller contract and intent mismatches already falsify the shipped architecture

These do not outrank the transaction defects, but they should be corrected as part of the same rebuild rather than documented away.

1. **Empty explicit paths.** TypeScript requires at least one path, and Rust protocol comments say the backend rejects an empty list (@app/src/shared/protocol_source_control.ts:61-75; @crates/im_agent/src/protocol/commands_source_control.rs:32-40). Stage/unstage and discard instead return successful no-ops (@crates/im_agent/src/source_control/actions.rs:81-124; @crates/im_agent/src/source_control/actions_discard.rs:21-24). This violates ADR-005 contract parity and ADR-008’s no-silent-recovery rule (@docs/compliance/adr_005_typescript_native_contracts_and_rails.md:21-35; @docs/compliance/adr_008_rust_runtime_contracts_and_error_handling.md:20-36).

2. **Deleted rows still have Open Diff.** Only Open File and Open Containing Folder are disabled; Open Diff is always present (@app/src/components/source_control/source_control_context_menu.ts:35-60). That conflicts with “deleted rows do not open” / “disable open actions” (@docs/design/source_control_design.md:62; @docs/design/intermediary_ui_overhaul_design.md:211-215; @docs/prd.md:142-143).

3. **SOURCE count is area rows, not changed files.** The hook/body sum index + worktree + conflicts (@app/src/hooks/source_control/use_source_control_state.ts:238-240; @app/src/components/source_control/source_control_body.tsx:63-74). An `MM` file counts twice, while staged outside-root paths count zero. A subroot can therefore show `NO CHANGES` and a hidden zero badge while still warning that hidden paths will be committed and enabling COMMIT.

4. **Host reads are not cancellable.** The host backend explicitly passes no cancel token because the host agent has no cancellation path (@crates/im_host_agent/src/runtime/local_host_source_control_backend.rs:37-62), although design/architecture/PRD say reads are cancellable (@docs/design/source_control_design.md:68-76; @docs/architecture/source_control_architecture.md:50-52; @docs/prd.md:145-146).

5. **New module layout is already over ADR-000’s folder threshold.** `@app/src/components/source_control/` has 10 flat sibling modules and `@crates/im_agent/src/source_control/` has 15; ADR-000 says folders with 10+ siblings should split by concern (@docs/compliance/adr_000_modular_file_discipline.md:19-27). `@crates/im_agent/src/source_control/runner.rs` is 282 LOC and `@app/src/hooks/source_control/use_source_control_state.ts` is 255 LOC, both above the 250-LOC target. The subsystem should be grouped around model/projection, operations, process lifetime, and UI operation state while the transaction owner is introduced.

---

## Required end state — one physical-worktree Git operation coordinator

Do not fix these as ten local patches. The compliant end state is one coordinator in the agent-owned Source Control domain, shared by host-root and WSL-root execution, whose authority is a canonical physical Git worktree/index identity.

It must own:

- **Snapshot identity:** status generation, index-tree identity, merge/conflict state, scoped relational records, and explicit omitted effects.
- **Operation identity:** stable operation ID, intended structured action, expected snapshot, affected endpoints, and user acknowledgements.
- **Serialization/barrier:** one mutation lane per physical Git worktree/index; status reads either observe a terminal snapshot or explicitly report the active operation.
- **Process tree:** Git plus hooks/SSH/helpers on Unix and Windows, bounded pipe drain, graceful/emergency cleanup, and lock-residue inspection.
- **Outcome certainty:** `notApplied`, `applied`, or `unknown`, derived from before/after repository identities—not error prefixes.
- **Transport survival:** UI or host→WSL timeout detaches the requester; it does not cancel, forget, or misclassify the operation. Reconnect queries the operation by ID.
- **Shutdown:** stop admission, drain coordinator, then terminate agents/WSL.
- **Watcher truth:** tracked-aware event suppression and all metadata necessary for the projected status contract.

This preserves the chosen host-routed architecture and UI intent while restoring the missing owner. It directly satisfies ADR-007’s contract-level ownership rule, ADR-008’s deterministic error semantics, and ADR-009’s bounded lifecycle requirements.

## What is already aligned

The review found several foundations worth keeping:

- Git remains outside Tauri/webview and runs in the agent that owns the root, preserving the host-routed architecture (@docs/architecture/source_control_architecture.md:24-35).
- Source Control uses the existing authenticated socket and introduces no new port/Tauri command surface (@docs/design/source_control_design.md:21-24).
- The Rust/TypeScript wire uses tagged, typed source-control command/result families rather than weakened generic payloads (@crates/im_agent/src/protocol/commands_source_control.rs:32-70; @app/src/shared/protocol_source_control.ts:61-86).
- Successful actions return a fresh status, and follow-up read failure is already distinguished from a pre-application Git failure in one narrow path (@crates/im_agent/src/source_control/actions.rs:16-79).
- The UI rail/workspace decomposition largely preserves the intended product shape rather than turning Intermediary into a full Git client.

Those pieces should be retained inside the operation-coordinator rebuild.

## Closeout actions

1. Remove Source Control from “Completed Features” and do not treat 0.1.14 as shipped from this tree until P0/P1 findings are resolved and witnessed.
2. Replace the current mutation request model with the single coordinator end state above; do not tune timeout constants as a substitute.
3. Add a Source Control implementation/review report to the docs guide only after the implementation truth exists. Its witness set should include the seven disposable-repo scenarios shipped with this review, plus installed-app close/restart during host and WSL commits.
4. Record any deliberately deferred P2s in `@docs/known_issues.md`; it currently contains no Source Control issue and predates this feature (@docs/known_issues.md:1-18).
5. Update architecture/changelog only from witnessed behavior. In particular, delete the unsupported claim that the runner guarantees no leftover index lock until shutdown and process-tree ownership actually prove it.

## Evidence files

The companion evidence archive contains:

- `copy_discard.txt`
- `cross_root_rename.txt`
- `commit_snapshot_race.txt`
- `pipe_hold.txt`
- `timeout_landed.txt`
- `unresolved_merge.txt`
- `stage_changes_resolves_conflicts.txt`
- `repo_id_lock_alias.txt`

All probes used disposable repositories and did not modify the bundled tree.
