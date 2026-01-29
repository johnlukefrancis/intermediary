# ADR-012: Copy-safe command delivery
Updated on: 2026-01-22
Owners: JL · Agents
Depends on: ADR-000, ADR-006

## Context
Agent responses that include inline commands are frequently copy-truncated or corrupted by the in-app/CLI console.
This causes broken paths, failed runs, and repeated debugging churn.

We already enforce workflow discipline (ADR-006), but command delivery is still fragile at the response layer.
We need a copy-safe delivery path that is reliable for the user and enforceable for agents.

## Decision
All runnable commands must be delivered via repository files under `docs/commands/`.

Rules:
- **No inline commands in agent responses.** This includes inline snippets and fenced blocks.
- **Commands must live in `docs/commands/**` files.** Use a scoped folder per area (e.g. `docs/commands/workflow/`).
- **Agent responses must link the command file path and describe what it does.**
- **If a response needs a new command, create a new commands doc and add it to `docs/guide.md`.**

## Consequences
- Agent responses will be copy-safe by default.
- Command sequences are discoverable, versioned, and reviewable in-repo.
- Any doc or workflow that requires commands must now point to a `docs/commands/**` file.

## Enforcement
- Violations are P0 workflow failures.
- Add checks or reminders in AGENTS/CLAUDE and skills to keep the rule visible.
- During reviews, reject any response or doc that ships inline commands.
