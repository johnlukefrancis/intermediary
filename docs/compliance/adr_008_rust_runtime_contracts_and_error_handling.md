# ADR-008: Rust Runtime Contracts and Error Handling

Status: Accepted
Date: 2026-01-14
Owners: JL · Coding agents
Scope: Rust runtime (src-tauri/**, crates/**)

---

## Context
TexturePortal relies on Rust for runtime orchestration (Tauri commands, ML sidecars, IO, and core pipeline). Failures must be surfaced as typed errors, not panics, and invariants must be enforced consistently across release builds.

---

## Decision
1) **No panics across Tauri command boundaries**
- Tauri commands must return `Result<_, String>` (or a typed error converted to `String`).
- Command handlers must not `panic!`/`unwrap`/`expect` on runtime inputs.

2) **Errors are explicit and user-safe**
- Runtime errors must include actionable context (what failed, which input), but avoid leaking internal paths or secrets.
- Prefer structured error types internally; convert at the boundary.

3) **Invariants are enforced in release**
- Required invariants must be enforced with explicit error handling or `assert!` + fail-fast at startup.
- `debug_assert!` is not allowed for required runtime invariants.

4) **No “silent recovery”**
- If a runtime path cannot proceed safely, return an error; do not continue with defaults that change behavior.

---

## Invariants
- I8.1: Tauri command handlers must never panic on user or runtime input; they must return `Err(String)`.
- I8.2: Required invariants are enforced in release (no `debug_assert!` for runtime contracts).
- I8.3: Error messages emitted across IPC must be deterministic and actionable.

---

## Noncompliant examples
- `unwrap()` on user-provided path inside a Tauri command.
- `debug_assert!(config.valid())` for a rule that must hold in production.
- `panic!("should never happen")` inside a runtime worker thread.

---

## Consequences
- Errors become visible and recoverable, improving UX and debuggability.
- Runtime failures surface at the correct boundary instead of crashing the app.

---

## Enforcement
- Code review must reject new panics/unwraps in Tauri command paths.
- CI or review should check new invariants are enforced in release paths (no `debug_assert!`).
- When touching a runtime path, migrate any surrounding panics to `Result`-based errors.
