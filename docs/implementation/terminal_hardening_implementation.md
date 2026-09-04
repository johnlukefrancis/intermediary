# Integrated Terminal Hardening Implementation
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010, ADR-013
Status: **Complete** — the hardened 0.1.16 installer is built, installed, and running for JL.

## Intent and witness

Preserve the accepted integrated terminal while closing the lifecycle, WSL-entry, output-bound,
process-tree, retained-resource, and flow-credit defects found in the 2026-09-04 promotion review.
The ordinary installed-app witness remains `docs/commands/verify_terminal.md`: exact host/WSL entry,
interactive TUIs, bounded flood and close behaviour, retained tabs, and terminal shutdown before the
agent supervisor's WSL idle probe.

## Owner decision

Verdict: **rebuild** the backend lifecycle and Windows spawn routes, **extend** the shared WSL-control
boundary, and **tune** the frontend cap and acknowledgement protocol.

- `TerminalRegistry` owns one terminal transaction from atomic admission through `Opening`, `Running`,
  `Closing`, `Reaping`, and a joined terminal receipt. Admission freezes with app exit, and the twelve
  slots count every non-terminal transaction. A short external reaper—not a terminating worker—joins
  the reader, waiter, PTY close, and any close worker before final removal; app exit waits for opening
  transactions and takes over every already-closing or reaping transaction.
- One Rust output-sink owner marks the sink detached without waiting behind channel publication.
  At most the already-started bounded frame can finish; no later publish can begin. Explicit closure
  detaches before releasing flow credit, then drains ConPTY privately to EOF.
- Flow credit is a monotonic cumulative watermark: Rust records bytes sent, the frontend confirms the
  highest bytes consumed, rejects an impossible watermark beyond bytes sent, ignores stale/duplicate
  watermarks, and the frontend advances its confirmed watermark only after success while coalescing and
  retrying failures.
- Native WSL roots are validated, before interactive spawn, in the resolved distro through the shared
  non-login stdin-script control boundary. An implicit default is resolved once to its actual distro
  name, then that explicit identity is used for both validation and interactive entry.
- The Windows PTY spawn seam attaches the new shell to its Job Object at process creation. No user
  profile code can execute first. Reader/writer setup precedes process creation where possible; every
  remaining post-spawn failure terminates the Job and observes the child and PTY teardown.
- The frontend twelve-tab cap counts every retained xterm/WebGL owner, including exited and failed tabs.
- A configured repo retaining its id but changing root closes the old group, so restart can never enter
  the previous root.

The terminal's profile fidelity, console-first close, raw-byte channel, renderer parking, twelve-tab
product bound, Tauri-only IPC surface, CSP/capability shape, and installed-app UX remain locked.

## Implementation route

1. Replace live-map removal with atomic transaction admission, close requests, retained worker handles,
   externally joined reaping, and one terminal receipt. Move the exit frame after joined reader/waiter
   finality so restart cannot race an occupied backend slot. Make navigation non-blocking without
   surrendering ownership; make app exit close and join every admitted transaction before the agent
   supervisor runs.
2. Split output publication from PTY drain, detach the sink on explicit close, and convert the Rust flow
   gate plus TypeScript acknowledgement coalescer to cumulative monotonic watermarks.
3. Move the reusable WSL stdin-script command boundary to its smallest shared owner and validate the
   exact native WSL root and resolved distro before opening a pseudoconsole.
4. Replace post-spawn Job assignment with the one exclusive narrow Windows ConPTY spawn adapter, using
   one creation attribute list containing `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` and
   `PROC_THREAD_ATTRIBUTE_JOB_LIST`. Retain the portable Unix PTY route for local lifecycle tests.
5. Count all retained frontend tabs, close groups on same-id root replacement, add only owner-level
   regression tests, update current architecture and release truth, regenerate required inventories,
   run repository gates, build/install the Windows installer, and open the worktree repair candidate
   for JL while preserving the requested staged-baseline/unstaged-repair boundary.

## Plan challenge adjudication

One fresh Luna adversarial challenge accepted ten findings into this same plan: continuous discoverable
close ownership; externally joined worker/PTY receipts; complete post-spawn Job cleanup; deadlock-safe
bounded sink detach; explicit default-distro pinning; one exclusive at-creation Job seam; full cumulative
ACK semantics; atomic and resource-symmetric capacity; same-id repo-root replacement; and focused proof
for each latent race. No finding changed the requested noun or replaced the owner decision.

The fresh final challenge then found four accepted owner edges: Job observation had no deadline; Job API
errors could be mistaken for whole-tree finality; a worker could fail to create its own reaper thread;
and restart relied on passive root reconciliation. The repair now bounds and arms forced Job cleanup,
retains and retries `StillAlive` trees while preventing the supervisor route, schedules reapers on the
runtime blocking pool, and validates current root identity at restart itself.

Two final-challenge items did not describe defects in the accepted product boundary. Tauri 2.9.5's
production `Channel::send` for these 16 KiB frames inserts the payload into its IPC fetch queue and posts
an event-loop evaluation request; it never invokes or waits for the JavaScript callback on the reader
thread. The adversarial blocking callback is available only through the test constructor, so adding a
second publisher queue would duplicate and weaken the existing byte bound. Separately, current `RepoRoot`
law has one native Linux path plus the app's configured/default WSL distro; a UNC distro segment is input
transport, not retained per-repo multi-distro authority. The terminal validates and enters the same one
resolved distro exactly; multi-distro routing remains outside this product model.

## Live receipt

- Target: `master` at `c35f45c`; the 92-path reviewed terminal/UI bundle is staged as the baseline.
- Requested noun: accepted integrated terminal with the promotion review defects repaired correctly.
- Final state: frontend and Rust owners are integrated. The candidate has one retained transaction map,
  joined external reaping, private close-time drain, cumulative credit, pinned WSL preflight, an
  exclusive Windows ConPTY/Job creation seam, and bounded unresolved-tree retention/retry. The native
  owner proof, Rust-warning-clean installer build, silent install, and installed-window launch all pass.
- Final evidence: 29 focused terminal tests and the full 561-test Rust workspace pass; final TypeScript,
  ESLint, Cargo check, and diff checks pass; the Windows-only Job-at-creation test passes against the
  real ConPTY adapter; headers and the 612-file generated ledger are current.
- Next evidence roots: terminal registry/session workers, Windows ConPTY creation seam, shared WSL
  command runner, frontend registry/flow owner, terminal architecture and witness route.
- Resolved interface: `terminal_ack { sessionId, consumedTotal }` is the only flow-credit wire shape;
  frontend and backend implement it independently against monotonic totals.
- Resolved module route: `windows_pty.rs` owns ConPTY, `windows_process.rs` owns the one extended process
  creation call, and `windows_command_line.rs` owns UTF-16 argv/environment encoding. No post-spawn Job
  assignment route remains.

## Verification checklist

- [x] Focused Rust lifecycle, flow-gate, WSL-control, and spawn-owner tests pass (29 tests).
- [x] Atomic concurrent admission, Opening-versus-app-exit, joined receipt, detached publication, and
  impossible/stale/duplicate ACK regressions are covered at their smallest stable Rust owners; same-id
  root replacement is a direct frontend owner check (the repository has no frontend unit-test runner).
- [x] Windows proof exercises Job membership for a process created through the actual ConPTY adapter.
- [x] Frontend typecheck and lint pass.
- [x] Workspace Rust check and tests pass on the WSL toolchain (561 passed, 2 ignored).
- [x] Windows installer build compiles the actual ConPTY branch and produces the 0.1.16 NSIS package.
- [x] File headers and generated ledger are current (612/612 files, 100% non-TBD).
- [x] Installed app is launched for JL with the terminal witness route ready.
