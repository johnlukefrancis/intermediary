# ADR-006: Dev Environment and Agent Workflow Discipline

Status: Accepted
Date: 2025-12-05
Owners: JL, ChatGPT (planning agent), Agents
Stage: Environment and process guardrails

Applies to:

- Agent prompts and workflow for intermediary
- New documentation (Design, Implementation, Architecture, Reports)
- Any work that introduces new runtime surfaces or long-lived processes

## 1. Purpose

We already have strong rails for runtime and code (ADR-000..ADR-010, plus docs canon). We still lose time to workflow
failures around those rails: wrong doc placement, missing checks, and prompts that omit the real spec.

This ADR makes the workflow predictable so JL does not have to remember meta rules, and agents stop shipping
"tune later" systems that create debugging debt.

## 2. Behavior contract

When:
- JL requests new or updated docs (Design, Implementation, Architecture, Reports)
- Or any change affects user-visible behavior, subsystems, or scripts
Then:
- Start from the relevant docs index in `docs/` and place docs in the correct subtree.
- Follow `docs/environment/docs_workflow.md` for naming, headers, and required sections.
- Agent prompts must include `im-docs-discipline` in Constraints (most work requires doc updates).

When:
- A change affects long-running runtime behavior (ML sidecars, queues, IO, streaming)
Then:
- Add observability (stage, logs, diagnostics) in the same PR ladder, not "later".

When:
- JL asks for a Codex prompt
Then:
- Output a single branch name line, then a single fenced prompt with **Task, Context, Refs, Deliver, Constraints**.
- Refs must point at the correct repo paths; do not invent paths.

When:
- A task touches runtime or architecture
Then:
- Include the relevant ADRs in Constraints and confirm they were read.

When:
- An agent investigates a bug, failure, or unexpected behavior
Then:
- Request/locate the run log (`logs/run_latest.txt` or the specific run file).
- Cite log excerpts (seq range, error entries, relevant breadcrumbs) in findings.
- Do not rely solely on stderr output or console screenshots.

When:
- An agent begins any turn that touches code (TypeScript, Rust, ML sidecars)
Then:
- Invoke the relevant skills from the skills table (see AGENTS.md / CLAUDE.md).
- First line of the reply must be `Skills: …` listing skills used.

## 3. Decisions and rails

### 3.1 Docs are the primary spec

- Treat repo docs as the source of truth (see `docs/` and `docs/compliance/`).
- If a task references a document by name, read it before planning or editing.

### 3.2 Prompt shape is strict and minimal

Prompts authored by ChatGPT must:
- Use **Task, Context, Refs, Deliver, Constraints**.
- Keep prompts concise and aligned to the repo spec.
- Avoid vague requirements like "do what's reasonable".

### 3.3 Environment and execution discipline

- Edit in WSL, run the app in Windows when required by the repo workflow.
- Use repo scripts for sync and run steps (e.g. `scripts/windows/sync_to_windows.sh`, `scripts/ml/*`).
- Windows dev runs MUST set `INTERMEDIARY_LOG_DIR` to the WSL UNC path (`\\wsl$\<distro>\<wsl_path>\logs`). Do not symlink `logs/` or rely on `/mnt/` visibility.
- Required checks must be run when specified in AGENTS or ADRs.

### 3.4 Types are contracts (ADR-005)

- Do not weaken TS types to silence errors.
- Fix code to match contracts, or tighten types to reflect reality.

### 3.5 Skills are mandatory workflow rails

Skills encode architectural and compliance rails and must be invoked proactively based on task surface.

- Skills are global-only (`~/.codex/skills` for Codex, `~/.claude/skills` for Claude).
- Agents must invoke skills based on the task surface—don't wait to be asked.
- Every agent reply that touches code must start with `Skills: …` listing skills used.
- See AGENTS.md / CLAUDE.md for the canonical skill table.

**im-docs-discipline is near-universal**: most work changes behavior or touches docs, so include it by default. It enforces guide index updates, roadmap/known_issues upkeep, required doc headers, and prevents pasted chat logs as repo docs.

## 4. Invariants

I6.1 Docs placement invariant
- New docs must live under the correct `docs/` subtree.

I6.2 Prompt shape invariant
- Codex prompts use Task, Context, Refs, Deliver, Constraints; Refs must be real paths.

I6.3 Observability invariant
- Long-running operations must ship with a minimal observability surface (stage + logs + diagnostics).

I6.4 Type safety invariant (ADR-005)
- No type weakening to satisfy the compiler.

I6.5 Skills invocation invariant
- Agents must invoke relevant skills proactively; every code-touching reply starts with `Skills: …`.

I6.6 Log-first debugging invariant
- Investigations MUST cite run log entries (seq + target + message). Reports without log evidence are incomplete.

## 5. Enforcement

- New work follows ADR-006 immediately.
- If an existing subsystem is missing required workflow/observability, fix it in the same PR ladder.
