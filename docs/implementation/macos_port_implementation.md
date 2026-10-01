# Native macOS Port Quest
Updated on: 2026-09-29
Owners: JL · Codex
Depends on: ADR-000, ADR-005, ADR-006, ADR-007, ADR-008, ADR-009, ADR-010, ADR-012

## Intent and witness

Port Intermediary to native Apple Silicon macOS while preserving Windows.
The delivered witness is the installed desktop app: repository discovery,
watcher updates, context bundles, native drag-out handoffs, and an interactive
terminal in the selected repository. Compilation alone is insufficient.
JL owns subjective acceptance and any external handoff that requires their hands.
On 1 October 2026 JL authorized committing and pushing the Mac work for recovery
before losing this laptop. Never delete outside this project.

## Live receipt

- Target: `/Users/johnlukefrancis/dev/intermediary`, `master`, baseline `7761f37`;
  initially clean. macOS 26.6.2, arm64, Apple M1 Pro.
- Toolchain: native Cargo checks pass with SDK 26.5. Native Node
  22.23.3, pnpm 10.34.6 and Rust 1.98.1 installed in ignored target/devtools.
- Decision: extend existing native host routing and Tauri-owned PTY lifecycle;
  retain Windows ConPTY/Job/WSL ownership. Platform adapters own shell,
  clipboard, process teardown and packaging differences; shared protocol and
  UI lifecycle stay shared. No new sockets, shell plugins or relaxed CSP.
- Packaging must produce a current arm64 host agent and app from this tree.
- Current candidate: review repairs installed at `~/Applications/Intermediary.app`,
  locally ad-hoc signed arm64 app and helper. Installed terminal confirmed native
  zsh, repo cwd, xterm-256color and profile tools. Core workflow was accepted by JL,
  including ZIP delivery into Finder. Header continuity and single-instance
  startup were previously verified; their implementation is unchanged.
- Follow-up owners: `header-stack` paints one continuous grain/scanline layer;
  individual repo/status bars retain their controls and separators. A Mac-only
  kernel file lock, retained until process exit, excludes duplicate startup
  before logging/auth/runtime writes. The persistent lock file is never deleted;
  native app activation/reopen restores the existing window. Windows startup
  remains unchanged. TypeScript, ESLint, Cargo, both lock tests and the release
  build pass. The desktop header is continuous; two duplicate executable
  launches exit 0, retaining app PID 16415, host PID 16422 and one startup log
  record. Updated the six installed bundle files after an in-project backup;
  deep/strict signing verification passes. Finder reopening restored the installed
  window with one unchanged startup record and one app/host pair.
- Review repairs: native input owns a side-effect-free duplicated FD. Import
  planning rejects every `.git` descendant before type branching or writes.
  Failed worker creation retains unresolved process ownership and its capacity
  slot. Shutdown collects first-pass receipts before starting its shared retry
  deadline. 32 terminal, 17 import and 31 worktree tests pass, including injected
  reader/waiter failures with a surviving HUP-resistant descendant, a 400 ms
  delayed first pass that still receives at least 250/300 ms retry budget, and
  real linked-worktree imports under Refuse/Replace. TypeScript, ESLint, Cargo
  and release build pass. Desktop tab close, fixture-repo removal and quit left
  typed-but-unsubmitted commands unexecuted; all three markers remain absent.
  Only the coordinator edited; one Luna explorer mapped import callers read-only.
- Remaining hands-on witness: Finder folder import containing a linked-worktree
  `.git` pointer must show a Git-metadata refusal with no files copied. The native cross-window
  drag could not be completed reliably with the available controller; fixture
  instructions are in `docs/commands/dev_macos.md`. This is a UI evidence gap;
  all four failure paths have code repairs and passing focused checks.
- Environment: JL installed Xcode and accepted its terms; default Apple Git now
  runs. This build still selects known-working CLT and SDK 26.5 per process.
  No agreement or global developer-setting change was made by the agent.
- Resolved: macOS PTY setsid owns terminal-attached processes across shell job
  groups; enumerate that session only at close, SIGHUP then bounded SIGKILL.
  Native AppKit pasteboard reads stay inside existing Tauri IPC.
- Packaged Mac helper is the executable authority; application-data copies
  from the first run are preserved. Supervisor sessions are established before
  exec and a retained stdin pipe requests drain if the app dies.
- Current native controls respond after reopening. The Sol High final review
  found lost process ownership after terminal natural-exit cleanup failure and
  supervisor tree cleanup failure; both are confirmed at their call sites.
  Both are repaired: failed owners and worker handles remain resident until an
  explicit retry succeeds. All 157 app tests pass (2 ignored).
- Legacy skill entrypoints named by AGENTS (Rust/TS rails, Tauri baseline,
  copy-safe commands, review-lens, workflow-closeout) are absent both from the
  installed skills and canonical global checkout. Apply their current ADR
  owners directly; current global skills remain available.

## Implementation ladder

1. [x] Install native development tools; establish baseline build failures.
2. [x] Challenge plan independently and adjudicate concrete findings.
3. [x] Repair native packaging and platform adapters at existing owners.
4. [x] Run TypeScript, ESLint, Cargo and focused invariant checks.
5. [x] Build release app and inspect stable candidate independently.
6. [x] Exercise real desktop workflow, repair task-blocking failures.
7. [x] Install locally, launch for JL, reconcile architecture/docs/ledger.
8. [x] Repair header texture continuity and Mac repeated-launch behavior;
   rebuild, inspect the real app, update local installation and reopen for JL.
9. [x] Close external-review terminal-input, Git-import, failed-open ownership,
   and shutdown-retry defects; verify the installed app and document coverage.
10. [ ] JL hands-on Finder linked-worktree import refusal witness.

## Verification and acceptance

- [x] Native repo discovery through ordinary app controls.
- [x] File edit observed in live stream/tree through the watcher.
- [x] Context ZIP built from selection and contents inspected.
- [x] Native drag-out starts with the generated file; receiving-app witness.
- [x] Terminal starts in repo with native profile, renders input/output, resizes,
  supports clipboard and control keys, survives switching, closes cleanly.
- [x] Final installed-app quit drains terminal and host runtimes.
- [x] Current release .app installed and opened for JL.
- [x] Windows platform paths preserved; unavailable Windows runtime proof stated.

## Evidence

Native Cargo check passes after implementing macOS exclusive rename. SDK 27.0
stubs fail with the installed linker; SDK 26.5 works without system mutation.
First release app opens and connects after its startup probe; the initial
offline view was transient, confirmed by spawn_ready/probe logs. The native
picker has added this checkout and populated its file tree. TERMINAL opens
the account's zsh prompt in intermediary. 28 terminal tests now pass, including
background job groups that ignore SIGHUP; five atomic rename tests pass.
Historical macOS parity report is evidence only.
Commands are in `docs/commands/dev_macos.md`.

Plan challenge (fresh Luna Max): accepted native tree ownership and clipboard
findings; both were independently confirmed in source. Windows preservation
requires focused review of the conditional branches and shared contracts; this
Mac cannot provide Windows desktop acceptance. A Windows build/runtime witness
remains an explicit limitation, never inferred from the Mac build.

First desktop witness (06:02–06:12): startup connected, current checkout in
the tree, live source/Quest edits rendered, zsh prompt in the correct cwd,
SHELL=/bin/zsh, TERM=xterm-256color, profile-resolved codex/claude, PTY 35x46.
Paste landed despite a Computer Use clipboard-read timeout. Rail switching
retained the shell. Quit logs: session_close outcome=exited, shutdown_all
still_alive=0/receipt_errors=0, host graceful_stop_ack drained=true followed by
exit status 0. PIDs 11573/11582/11676 are absent. Logs remain in app-local logs.

Current checks: TypeScript and ESLint pass, Cargo check passes (only upstream
block 0.1.6 future-compatibility warning), 157 app tests pass/2 ignored, including
29 terminal tests. Four earlier socket-test failures were sandbox denials and
passed with authorized loopback access. Test temporary files stay in target.

Second desktop witness (06:18–06:26): the signed app connected using its packaged
helper. BUILD BUNDLE with docs selected and root files excluded produced
`intermediary_context_20260929_051945_7761f37.zip` (353899 bytes, 74 entries).
ZIP CRC validation passed; entries are docs and the standard bundle metadata.
SHA256: `ab1267a71409d6d0bc7529278e5c0fbf974c868ed39197a8270b635a18a7d708`.
Native rail dragging resized the PTY from 35x46 to 35x68; Control-C interrupted
a live sleep; selecting output and Command-C/Command-V pasted the same text.
Quit again recorded zero still-alive sessions, no reader errors, and drained
host exit. The automated drag was inconclusive; JL completed the receiving-app
witness below using Finder's `target/macos_witness/received` folder.

Final review adjudication: the coordinator confirmed both P1 call paths.
Mac natural terminal cleanup now runs in the external reaper; failed cleanup
stores the unjoined reap bundle and StillAlive receipt, and app exit retries
before joining and releasing the slot. Supervisor reconciliation uses one
blocking tree-and-child route and restores ownership on tree errors even after
the direct child exits. Regressions cover natural exit with a HUP-resistant job
and an injected failed tree cleanup after direct child exit.

Installed-app witness (06:29–06:31): process inspection confirmed both app and
helper execute from `~/Applications/Intermediary.app`. Deep/strict codesign
verification passed. Executable SHA256:
`4d9f161c8b3b67a4e143e10f1c266cff4fe3714569179c93f9e344b4908b7c09`.
The installed app built `intermediary_context_20260929_053018_7761f37.zip`
(359791 bytes, 74 entries, CRC check passed). Terminal cwd is this checkout;
natural exit showed CODE 0, RESTART reopened zsh, and app quit recorded
still_alive=0, receipt_errors=0 and a drained host exit. App/host PIDs
14497/14503 were absent after quit. The app was reopened and CONNECTED with
the latest bundle visible. No notarization or Windows build/runtime was run.

Runtime state: the installed app is open for JL. Config backup is
`target/macos_witness/config_before_output_correction.json`; outputWindowsRoot
(the existing cross-platform persisted field name) now points to
`target/macos_witness/staging`. The native picker temporarily chose ~/dev;
its created `~/dev/state` is preserved, never removed. No commit, push or staging
has occurred. Installation copied to a previously absent target; no external
files were deleted. Computer Use had no exposed cancel API and refused access
to Codex's own controls; its kernel was reset and JL was informed.

Final handoff acceptance: JL reported that the app works and dragged the ZIP
into Finder. The received `intermediary_context_20260929_053018_7761f37.zip`
matches the staged source byte-for-byte (359791 bytes, 74 entries, CRC passed).
Both SHA256 values are
`9ade0bb26d185a45ea3c04b49c34a59e4fd47af57cb4d579ed58d7573ed7b6e3`.
The requested native core workflow is complete. Windows runtime regression
proof and distribution notarization remain explicit verification limits.

Header/instance follow-up (06:53–07:03): the repo/status texture previously
restarted at each bar's local origin. One parent overlay now spans both rows;
actual WebKit rendering shows the next portion of the texture in the status
row and the Options control opens/closes correctly. Both live executable
relaunches returned 0 (one after Command-M); app PID 16415 and host PID 16422
survived with one original startup record. Quit drained the host and recorded
zero still-alive terminals/receipt errors. Two focused lock tests, TypeScript,
ESLint, Cargo check, release build and diff whitespace checks passed. Existing
upstream block 0.1.6 and Vite chunk-size warnings remain; the full app suite was
not repeated for this focused follow-up.

The old installed app is copied to
`target/macos_witness/installed_before_header_instance_fix.app`. The bundle
layouts each contain the same six regular files and no symlinks; those six
installed files were overwritten and byte-checked without deleting any paths.
Deep/strict codesign verification passed. Installed executable SHA256:
`1e4429d6784bbeff8286f7b846dffe2212b9e24febc2bb891790ccc5ef6ef8fd`.
Finder reopening after Command-M left the installed app connected, with app
PID 16616, host PID 16624 and a single unchanged startup record. SOURCE was
restored for JL. The Computer Use controller still exposes no cancel method;
its visible indicator yielded no cancellation control, and the kernel was
reset. No cancellation claim, commit, push, staging or external deletion.

## External-review repair witness

The review's two P1 and two P2 findings were confirmed against the current source.
The terminal uses a plain duplicated Unix FD so input destruction cannot send
Enter/EOF; an already-queued write rechecks close state after acquiring its lock.
Import planning rejects Git control metadata before file-kind dispatch and copy
authorization. Worker-spawn failures retain a StillAlive transaction when process
cleanup fails, and the shared 300 ms retry budget begins after first-pass waits.

Focused checks: 32 terminal tests, 17 import tests and 31 worktree tests pass.
The latter initially exposed an existing fixture that conflated two source names
on case-insensitive APFS. It now keeps sources in separate folders and verifies
that aliased destinations cannot overwrite one another. The Replace import test
authorizes only the ordinary content collision, so source planning itself must
reject the Git pointer. TypeScript, ESLint, Cargo check and release build pass.
Full workspace tests and Windows build/runtime were not repeated. Existing
upstream block future-compatibility and Vite chunk-size warnings remain.

Desktop witness (08:36–08:44): app/helper initially ran from the fresh release
bundle. Three visible unsubmitted commands remained unexecuted after tab close,
fixture-repo removal and app quit. All marker files are absent; each shell exited
without reader errors. Quit logged still_alive=0 and receipt_errors=0; the host
acknowledged drained=true and exited 0. App configuration exactly matched its
backup after fixture removal. Logs: `target/macos_witness/review_candidate_runtime.log`.

Updated exactly six installed files after preserving the prior bundle at
`target/macos_witness/installed_before_review_repairs.app`; no external paths
were deleted or moved. Deep/strict signature verification and per-file hashes
pass. Installed executable SHA256:
`aa834acc9f0f0a8044eb123cd063bb7339d49c6663393299bd1512ed8d093f36`.
Reopened installed app/helper PIDs 19652/19658; CONNECTED, native zsh, repo cwd,
profile tools and submitted input verified in the actual desktop terminal.
The installed app built `intermediary_context_20260929_074507_7761f37.zip`
(13432118 bytes, 872 entries, ZIP CRC passed), preserving JL's current selection.
SHA256: `f8f7c57ea10bcc04ce7b3357562122c8e6dc3ba37cc3e811a4999f09c114cd44`.
Watcher updates appeared in the actual stream; the app remains open on ZIPS.
Computer Use exposes no controller cancel API; its visible indicator supplied
no cancellation control. The kernel was reset, without claiming cancellation
or closing Intermediary. No staging, active-repository commit, push or external deletion.

The external review ZIP omitted `scripts/build`; the coordinator inspected both
packaging scripts locally and the release build rebuilt its current helper.
Future external packaging review must include those authored scripts. Native
Finder import refusal remains a hands-on check; lower-level real-worktree tests
prove flat/nested refusal, zero destination writes and unchanged source Git data.

## Changed owners

| Paths | Purpose |
| --- | --- |
| `.gitignore`, `Cargo.lock`, `src-tauri/Cargo.toml`, `src-tauri/tauri.macos.conf.json` | Native dependencies, generated helper exclusions, platform resources and local signing. |
| `app/src/styles/{chrome,tab_bar,status_bar}.css`, `docs/design/intermediary_ui_overhaul_design.md` | One continuous header texture with existing controls and separators. |
| `src-tauri/src/lib/{mod,macos_instance}.rs`, `src-tauri/src/lib/commands/startup.rs` | Pre-state Mac instance exclusion and native activation/reopen. |
| `scripts/build/build_agent_bundle.mjs`, `scripts/build/ensure_agent_bundle.mjs` | Build the current native helper; refuse stale fallback artifacts. |
| `crates/im_bundle/src/{fs_atomic,lib,process_job,macos_process_session}.rs` | Exclusive Mac rename and shared native process-session ownership. |
| `src-tauri/src/lib/agent/{bundle_resources,install,install_runtime,install_tests,mod,process_control}.rs` | Packaged-helper authority and isolated native supervisor spawn. |
| `src-tauri/src/lib/agent/supervisor/{managed_processes,process_kill}.rs` | Preserve ownership after failed process-tree cleanup. |
| `src-tauri/src/lib/terminal/{clipboard,mod,reaper,registry,session,session_close,session_open,session_spawn,session_spawn_cleanup,session_spawn_tests,shell,start_dir,transaction}.rs` | Native shell/pasteboard, PTY lifecycle, retained cleanup failures and regression checks. |
| `src-tauri/src/lib/terminal/{reader_thread,waiter_thread,registry_shutdown,transaction_receipts,spawn_faults,session_recovery_tests}.rs` | Retained failed-open ownership, independent shutdown retry budget and injected-failure checks. |
| `crates/im_agent/src/repos/import/{sources,copy,mod,tests_refusals,tests_git_control}.rs`, `crates/im_agent/src/repos/worktree/tests_no_replace.rs` | Git-control import refusal and filesystem-correct regression fixtures. |
| `app/src/lib/terminal/{terminal_keys,terminal_session,terminal_theme,terminal_types}.ts`, `app/src/components/terminal/{terminal_copy.ts,terminal_exit_notice.tsx}` | Mac key bindings, shell labels and platform-correct xterm options. |
| `docs/{guide,prd,roadmap,system_overview,known_issues,changelog}.md`, `docs/architecture/terminal_architecture.md`, `docs/design/terminal_design.md`, `docs/compliance/adr_010_tauri_security_baseline.md` | Current platform behavior, ownership and acceptance limits. |
| `docs/design/zips_tree_write_surface_design.md` | Git-control import policy before all write/replacement paths. |
| `docs/commands/dev_macos.md`, this Quest, `docs/inventory/file_ledger.{md,json}` | Reproducible commands, durable receipt and regenerated inventory. |
