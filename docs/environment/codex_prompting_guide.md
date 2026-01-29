# Codex Prompting Guide

Updated: 2025-01-10

## 1. Core Philosophy

Modern Codex models are autonomous coding agents: they plan, call tools, and compact long trajectories. We only need to give clear work and hard rails.

Key ideas:
- Prompts stay short and surgical
- Don't restate generic CLI rules
- Don't ask for explicit plans or "think step by step" — the model handles this
- Point to docs and code paths instead of re-encoding repo internals

Every prompt is just:
- **Task** — what must happen
- **Context** — which slice, what exists, what's off limits
- **Refs** — which docs to read
- **Deliver** — what to produce
- **Constraints** — non-negotiable rails

---

## 2. Canonical Prompt Shape

```text
Task: <imperative, one or two sentences max>
Context: <essential project context, 1–4 lines>
Refs: { docs/..., src/... }
Deliver: <what Codex should produce or change>
Constraints: <atomic, project-level rails only>
```

### Task
- Single imperative sentence
- Describe outcome, not process
- No meta like "make a plan" or "explain step by step"

### Context
- Keep very short: which slice, what exists, what's off limits
- Avoid long essays or pasting large code blobs

### Refs
- Docs and code paths only
- Prefer one tight set over long lists
- Never paste large code blocks — Codex can open files itself

### Deliver
- What the output should be
- Common patterns:
  - `Implement the feature and summarize edits`
  - `New doc at docs/... following project conventions`
  - `Scout report with findings; no code edits`

### Constraints
- Short, real rules
- Good: "Respect ADR-000 modular file discipline"
- Good: "No band-aids per ADR-007"
- Avoid: "Write beautiful code" or "follow best practices"

---

## 3. Rails to Remember

1. **ADR-000 — Modular files**: See ADR‑000 for the canonical target/cap and snake_case rule
2. **ADR-007 — Architecture-first**: No band-aids, fix at contract level

---

## 4. Example Prompts

### Feature Implementation
```text
Task: Add multi-repo tab support to the handoff console.
Context: Intermediary workflow tool. UI in app/src/, Tauri backend in src-tauri/.
Refs: { docs/compliance/adr_000_modular_file_discipline.md }
Deliver: Implement tab switching and summarize edits.
Constraints: Respect ADR-000 modularity; keep new modules within ADR-000 targets/caps.
```

### Bug Investigation
```text
Task: Investigate why file changes from WSL agent are not appearing in UI.
Context: Intermediary handoff console. IPC between WSL agent and Tauri UI.
Refs: { docs/system_overview.md }
Deliver: Root cause analysis and proposed fix.
Constraints: No band-aids per ADR-007.
```

---

## 5. Anti-Patterns

Avoid:
1. **Over-prompting** — long essays, pasting code, re-describing tools
2. **Preambles** — "Start by outputting a plan, then wait..."
3. **Reasoning micromanagement** — "think step by step", "be verbose"
4. **Vague constraints** — "write beautiful code"

If a line isn't a concrete Task, Ref, or Constraint — delete it.
