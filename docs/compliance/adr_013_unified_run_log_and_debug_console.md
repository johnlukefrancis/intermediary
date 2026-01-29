# ADR-013: Unified Run Log and Debug Console
Updated on: 2026-01-23
Owners: JL · Agents
Depends on: ADR-000, ADR-005, ADR-006, ADR-007, ADR-008, ADR-009

## Context

The current debug window uses tabs and panels with fragmented logging: stderr output, runtime pack status lists, ML attempt logs, and scattered `console.error` calls. There is no single timeline. Ordering between frontend and backend events is guesswork. Debugging requires piecing together multiple sources, and investigations frequently stall because agents and humans cannot answer "what happened, in order?"

## Decision

One unified, sequential run log per app run. The debug console renders that same stream.

### D1: One sequential run log per app run
Each app run produces exactly one NDJSON log file, persisted in `logs/`. The file is append-only during the run.

### D2: Backend is the ordering authority
The Rust backend assigns a monotonic `seq` to every log entry (frontend and backend alike). Frontend never assigns ordering.

### D3: Debug console renders the log stream
The debug console shows a snapshot of the current run log plus a live tail. No bespoke debug tabs or panels exist outside this view.

### D4: Frontend logs via Tauri command
Frontend code logs by calling a Tauri command (`obs_log_frontend`). It never writes directly to the log file.

### D5: Error entries MUST include stack traces
TypeScript errors include `Error.stack`; Rust errors include the full error chain (source chain or `anyhow` context). Entries without traces are incomplete.

### D6: Invariant violations log at error level with structured context
Before returning an error or throwing, invariant violations emit a structured log entry at error level with the violated invariant name and relevant state.

### D7: Every user action and long-running stage MUST log start/end/error
User-initiated actions and multi-step backend stages emit at minimum: a start entry, an end entry (with outcome), and an error entry on failure.

### D8: `logs/run_latest.txt` contains the current run log path
On startup, the backend writes the absolute path of the current run's log file to `logs/run_latest.txt`. This is the stable lookup point for agents and tooling.

## Invariants

### I13.1: One NDJSON log file per run
No parallel log files for the same run. All sources write to the single run log.

### I13.2: Backend assigns seq
Frontend never assigns ordering. The Tauri command returns the assigned `seq` for confirmation.

### I13.3: No new debug tabs or panels
All diagnostics are log entries rendered in the unified debug console. New debug surfaces are rejected.

### I13.4: New features and commands MUST emit start/end/error log entries
A feature or command that lacks logging is incomplete and must not ship.

### I13.5: Debugging reports MUST include run log path or excerpt
Investigations without log evidence are incomplete. Cite seq ranges, error entries, and relevant breadcrumbs.

### I13.6: Ad-hoc in-memory debug lists are not the primary observability surface
Status arrays, attempt logs, and other in-memory lists may exist for UI state but must not be the primary debugging artifact. The run log is canonical.

## Noncompliant examples

- Adding a new "ML debug tab" that renders its own data outside the run log.
- Using `eprintln!` for diagnostics without also emitting a structured log entry.
- Investigations that describe behavior without citing log entries or the run log path.
- Frontend code that writes diagnostics to `localStorage` or a separate file instead of using `obs_log_frontend`.
- A new Tauri command that performs work without start/end/error log entries.

## Consequences

- Single debugging artifact per run; agents and humans check logs first.
- Faster bug hunts: "show me the log" replaces "reproduce and watch stderr".
- Debug console becomes a log viewer, not a bespoke dashboard.
- Existing fragmented debug surfaces (stderr panels, status lists, attempt logs) are migrated to log entries over time.

## Enforcement

- Code review rejects new debug panels or tabs.
- New commands without start/end/error logging are incomplete and blocked from merge.
- Agent workflows require log citations in debugging reports (see ADR-006 I6.6).
- Skills (`tp-core-rails`, `tp-workflow-closeout`) check for logging compliance.
