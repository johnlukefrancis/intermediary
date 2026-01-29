# ADR-009: Rust Concurrency and IO Boundary Rules

Status: Accepted
Date: 2026-01-14
Owners: JL · Coding agents
Scope: Rust runtime concurrency + IO (src-tauri/**, crates/**)

---

## Context
The app runs long-lived IO loops (sidecar NDJSON, save queues) and background threads. Concurrency bugs, spin loops, and sloppy EOF handling cause hangs or runaway CPU.

---

## Decision
1) **No spin loops**
- All background loops must block on a channel read, IO read, or explicit sleep with a bounded timeout.

2) **EOF and disconnection are terminal events**
- NDJSON readers and channel consumers must treat EOF / disconnect as shutdown signals and clean up state.

3) **Cancellation is explicit**
- Long-running tasks must support cooperative cancellation via `AtomicBool`, channel, or cancellation token.
- Cancellation checks must happen at stage boundaries (before/after heavy work).

4) **Don’t block the UI runtime**
- CPU-bound or blocking IO work must run in `tauri::async_runtime::spawn_blocking` or a dedicated thread.

5) **Thread lifecycles are bounded**
- Background threads must be tied to a supervisor or owner; no unbounded thread spawning in loops.

---

## Invariants
- I9.1: Background loops must block or sleep; no busy waiting.
- I9.2: EOF/disconnect ends the loop and triggers cleanup.
- I9.3: Cancellation is checked before and after heavy stages.
- I9.4: Blocking work never runs on the UI thread.
- I9.5: Thread creation is bounded and tied to an owner.

---

## Noncompliant examples
- `loop { if flag { ... } }` with no blocking or sleep.
- Ignoring `recv()` disconnect and continuing the loop.
- Running a long IO read directly inside a Tauri command without `spawn_blocking`.
- Spawning a new thread per message without a cap or supervisor.

---

## Consequences
- Predictable shutdown on EOF and cancellation.
- Lower CPU use and fewer deadlocks/hangs.

---

## Enforcement
- Code review must flag busy-wait loops and missing EOF handling.
- New long-running work must include cancellation hooks and be off the UI thread.
