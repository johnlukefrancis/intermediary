# Integrated Terminal Architecture
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-007, ADR-008, ADR-009, ADR-010, ADR-013

---

The terminal is a Tauri IPC surface. A terminal transaction lives in the Tauri process from admission
until its process, pseudoconsole, reader, waiter, and close worker have all produced one final receipt.
The webview owns presentation; neither agent owns a terminal session.

## Ownership

| Concern | Owner | Contract |
| --- | --- | --- |
| Wire shapes | `src-tauri/src/lib/terminal/frames.rs` ↔ `app/src/lib/terminal/terminal_ipc.ts` | Camel-case open/result/exit/close shapes plus raw input and output. Flow credit is only `terminal_ack { sessionId, consumedTotal }`. |
| Transaction table | `src-tauri/src/lib/terminal/registry.rs`, `registry_shutdown.rs`, `transaction.rs`, `reaper.rs` | One map contains every `Opening`, `Running`, `Closing`, or `Reaping` transaction. Admission, generation checks, the twelve-transaction cap, and the app-exit freeze share one lock. Only finalization removes an entry. |
| Runtime session | `src-tauri/src/lib/terminal/session.rs` | Owns the Job Object, pseudoconsole master, input writer, child killer, exit record, flow gate, and detachable output sink. Its local Running/Closing state is the I/O projection that refuses writes and records the first close reason; transaction lifecycle remains registry-owned. |
| Open route | `session_open.rs`, `session_spawn.rs`, `worker_start.rs`, `shell.rs`, `start_dir.rs` | Admission precedes blocking resolution and spawn. A worker start barrier prevents reader/waiter execution until their handles and the runtime are installed in the admitted transaction. Every failed-open resource is terminated, observed, drained, joined, or dropped before the transaction settles. |
| Windows process creation | `windows_pty.rs`, `windows_process.rs`, `windows_command_line.rs`; shared Job primitive in `crates/im_bundle/src/process_job.rs`, `process_job_termination.rs` | The exclusive Windows adapter creates ConPTY and calls `CreateProcessW` once with both `PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE` and `PROC_THREAD_ATTRIBUTE_JOB_LIST`. The shell is inside the Job when process creation returns; profile code never runs first. Forced cleanup arms kill-on-close, terminates, and observes Job emptiness only within its caller-owned deadline. |
| WSL control boundary | `src-tauri/src/lib/wsl_control.rs` | Shared five-second `wsl.exe` control runner: script over stdin to `bash --noprofile --norc -s`. Native roots resolve the actual default or configured distro once, pin that identity, and prove the exact Linux directory before ConPTY opens. |
| Reader and waiter | `reader_thread.rs`, `waiter_thread.rs` | Exactly two long-lived workers per running transaction. The waiter alone waits the direct child and records its exit. The reader drains to EOF and returns its byte/error receipt. Neither worker removes the transaction or joins itself. |
| Close ladder | `session_close.rs` | Detach output and refuse input, release the gate, close ConPTY on a retained short worker, wait console-first, then arm kill-on-close and terminate/observe the whole Job within a deadline. A failed Job receipt is `StillAlive`, remains registry-owned, and is retried once under one shared app-exit deadline; the supervisor is skipped if it stays unresolved. |
| Output bound | `output_sink.rs`, `flow_gate.rs`, `app/src/lib/terminal/terminal_flow.ts` | Rust charges bytes before publication and pauses at 512 KiB unconsumed, resuming at 128 KiB. The frontend coalesces cumulative consumed watermarks, confirms only successful invokes, and retries the latest watermark. Explicit close detaches before gate release and drains later bytes privately. Tauri 2.9's production channel route enqueues the payload plus an event-loop evaluation request; it does not call or wait for the JavaScript callback on the reader thread. |
| Frontend resources | `app/src/lib/terminal/terminal_registry.ts`, `terminal_session.ts`, `terminal_session_io.ts`, `terminal_renderer.ts` | The module-level registry owns xterm, DOM, renderer, and PTY client outside React. Every retained tab counts toward twelve, including exited and failed tabs. Root identity is part of group identity. |
| App ordering | `src-tauri/src/lib/mod.rs` | Page start increments the generation and requests closure without removing transactions. App exit freezes admission, requests every captured transaction, and waits final receipts before invoking the agent supervisor. If finality cannot be proved, the WSL idle route is not run. |
| Security and platform reads | `src-tauri/src/lib/commands/terminal.rs`, `clipboard.rs`, `windows_build.rs` | Six typed Tauri commands and one channel per session. Clipboard text and the Windows build number are read directly; no plugin, capability, CSP, host-agent, or WSL-agent surface is added. |

## Transaction lifecycle

```text
admit                 install runtime             request end       joined receipt
Opening  ──────────────────▶ Running ─────────────────▶ Closing ───────▶ Reaping ───────▶ Terminal
   │                            │                         ▲                 │
   └─ open failure ─────────────┴─────────────────────────┘                 └─ only here: remove map entry
```

1. The frontend mints a UUID and invokes `terminal_open`. Rust captures the page generation and
   atomically admits the UUID before resolving PowerShell, probing WSL, or creating a process. This is
   the capacity reservation and the lifecycle owner; concurrent opens cannot pass a separate check.
2. A host root must be an existing directory. A native WSL root first resolves one explicit distro and
   validates the exact path inside it. PowerShell still loads JL's profile. Its guarded initial
   `wsl.exe -d <distro> --cd <path>` returns normally to PowerShell after a successful bash exit, but a
   failed initial entry exits PowerShell instead of exposing the profile directory as a false success.
3. On Windows, pipes and ConPTY exist before the shell. Reader and writer handles are prepared, then the
   one extended process-creation call supplies both ConPTY and Job attributes. Any error after that call
   arms kill-on-close, terminates the Job within a bounded observation window, waits the direct child,
   and completes PTY teardown. A Win32 cleanup error is carried in the failed-open error rather than
   silently discarded; an armed Job remains the drop-time safety net.
4. The reader and waiter are created behind `WorkerStart`. The transaction installs the session and both
   join handles before the barrier releases. If navigation or exit claimed the Opening transaction, the
   installed runtime enters Closing immediately and never reports a successful open.
5. For each output chunk, the reader waits for credit, reads at most 16 KiB, charges the cumulative sent
   total, and publishes. Visible pages advance consumed credit from xterm's write callback; hidden pages
   advance it on receipt. Stale and duplicate watermarks are no-ops; a value beyond Rust's sent total is
   an error. A failed invoke leaves the frontend's confirmed total unchanged and is retried.
6. Natural child exit records the child once and closes ConPTY. Final output remains gated while the tab
   owns its sink. Reader EOF and child exit converge on one external reaper.
7. Explicit tab, repo, navigation, or app closure records the first reason, marks input closing, detaches
   the output sink, then releases flow credit. At most one already-started 16 KiB send can finish; every
   subsequent byte is drained from ConPTY without entering Tauri's channel queue.
8. The runtime blocking pool supplies the external reaper, so a terminating reader or waiter never has
   to create an ad-hoc thread and cannot strand its own join handle. The reaper owns the close ladder,
   joins waiter, reader, and PTY-close worker, records the escalation outcome, and publishes the
   natural-exit frame when a live sink remains. It removes the exact map entry only for a final receipt.
   `StillAlive` remains in `Reaping`, retains the armed Job and its capacity slot, and is retried at app
   exit; unresolved finality prevents the supervisor's WSL idle route.
9. Navigation changes the generation and requests every existing transaction without draining the map.
   App exit sets the admission freeze under the same lock, requests all phases with one shared console
   deadline, and waits every receipt before the supervisor may perform its WSL idle decision.

## Capacity model

There are two symmetric bounds over different resident resources:

- The frontend holds at most twelve retained tabs. Starting, running, exited, and failed tabs all retain
  an xterm/DOM/WebGL owner and therefore all count. Restart replaces the PTY inside one existing tab and
  consumes no additional frontend slot.
- Rust holds at most twelve non-terminal transactions. Opening reserves a slot atomically; Closing and
  Reaping keep it until every worker and PTY receipt is joined. A frontend close may free its renderer
  before Rust frees the process slot, so a replacement open can be refused briefly but neither resource
  set can exceed twelve.

A repo id is insufficient identity. If its canonical `RepoRoot` changes, the frontend closes and removes
the old group before creating or restarting a session under the new root.

## Invariants

- **I1 — Continuous ownership.** Admission creates the owner; only a joined terminal receipt releases it.
- **I2 — Console-first close.** ConPTY close is the ordinary signal; Job termination is bounded escalation.
- **I3 — Complete receipt.** Direct child, Job escalation, PTY close, reader EOF, waiter, reader, and close
  worker are observed before final removal.
- **I4 — Exit ordering.** Admission freezes and all phases settle before the supervisor's WSL idle route.
- **I5 — Bounded publication.** A live sink is cumulative-credit-gated; an ending sink is detached before
  private drain, with at most one already-started bounded frame.
- **I6 — Byte fidelity.** PTY bytes cross the raw channel and are decoded exactly once by xterm.
- **I7 — Bounded resources.** Twelve retained frontend tabs and twelve non-terminal Rust transactions;
  two long-lived workers per running transaction and no private recurring cadence.
- **I8 — Stable presentation.** Rail, repo, handset, and layout switches park sessions without disposing
  their terminal, scrollback, DOM, or renderer.
- **I9 — Fixed trust surface.** Tauri IPC only; no terminal agent, plugin, capability, or CSP expansion.
- **I10 — Workflow fidelity.** PowerShell 7 loads JL's profile; native WSL entry uses the validated path
  and pinned distro and never silently substitutes another shell or directory.

## Failure behaviour

| Failure | Result |
| --- | --- |
| PowerShell absent or host directory missing | Open settles with an actionable error; no process is registered. |
| Native WSL distro/path unavailable | The bounded preflight names both distro and path; ConPTY is not created. |
| WSL entry changes after preflight | The interactive PowerShell process exits instead of remaining in its profile directory. |
| Thirteenth frontend tab | The add control is disabled until a retained tab is closed. |
| Thirteenth or duplicate backend admission | Atomic admission returns an error before any probe or spawn. |
| Job, ConPTY, process, worker, or install failure | The complete resource set is terminated/observed and the Opening transaction settles. |
| Output acknowledgement lost or duplicated | The latest cumulative watermark is retried or ignored idempotently; credit is not permanently lost. |
| Explicit close during flood/navigation | Sink detaches, flow releases, PTY drains privately, and capacity remains occupied until its receipt. |
| Child or reader ends first | Its worker requests the one reaper; the other worker is still joined before removal. |
| Job termination or observation exceeds its deadline | The joined worker/PTy attempt reports `StillAlive`; the transaction and armed Job remain owned. App exit retries under one shared deadline and skips the supervisor if finality is still unproved. |
| Registry/transaction finality cannot be proved at app exit | The error is logged and the supervisor's WSL idle teardown is not invoked. |

## Proof and witness

Owner-level Rust tests cover atomic concurrent admission, an app exit racing an Opening transaction,
cumulative ACK semantics, detached publication, reader/waiter/PTY finality on a real Unix PTY, and app
shutdown of a live transaction. The Windows-only test creates a real child through the same ConPTY
adapter and queries Job membership immediately after process creation. The installer build compiles the
native branch; the installed-app route remains the product witness for profile fidelity, TUIs, renderer
parking, close feel, and WSL idle behaviour.

See [the terminal design](../design/terminal_design.md) and
[the installed-app witness route](../commands/verify_terminal.md).
