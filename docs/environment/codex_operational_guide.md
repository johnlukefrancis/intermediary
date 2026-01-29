# Codex CLI Operational Guide

Last verified: 2025-01-10

## Install + Update
- Install: `npm i -g @openai/codex`
- Verify: `codex --version`

## How We Run Codex

1. **Interactive (TUI)**: for multi-step work, editing, shell runs
   - `cd /path/to/repo && codex`

2. **Non-interactive**: for automation, CI-ish tasks
   - `codex exec "task..."` (read-only by default)
   - `codex exec --full-auto "..."` (allows edits)

## Session Controls

Slash commands (in TUI):
- `/status` — active sandbox/approvals + token usage
- `/diff` — git diff including untracked
- `/undo` — undo last turn
- `/compact` — shrink context
- `/skills` — skills browser
- `/mcp` — MCP tools list

## Default Config

Your `~/.codex/config.toml`:
- `approval_policy = "never"` — no prompts
- `sandbox_mode = "workspace-write"` — edit inside workspace, no network

Recommended features:
- `unified_exec = true` — PTY-backed exec
- `ghost_commit = true` — per-turn snapshots for `/undo`
- `skills = true` — skill discovery

## Trust + Guardrails

### Sandbox Combos
- Safe browsing: `codex --sandbox read-only --ask-for-approval on-request`
- Full auto: `codex --full-auto`
- YOLO (avoid): `codex --yolo`

Network in workspace-write stays off unless:
```toml
[sandbox_workspace_write]
network_access = true
```

## Skills

Skills are bundles (`SKILL.md` + scripts). Located in:
- `$CWD/.codex/skills`, `$REPO_ROOT/.codex/skills`
- `~/.codex/skills`, `/etc/codex/skills`

Invoke: `/skills` or type `$` then pick

Built-ins:
- `$skill-creator` — bootstrap new skill
- `$plan` — research + step-by-step plan

## Prompt Format

Use this skeleton:
- **Task**: what to change; success criteria
- **Context**: constraints, environment
- **Refs**: paths to read first
- **Deliver**: files/patches/tests + summary
- **Constraints**: sandbox, modularity rules, etc.
