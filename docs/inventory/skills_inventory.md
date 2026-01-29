# Intermediary Skills Inventory

Skills are workflow rails that agents (Codex/Claude) invoke proactively based on task surface. They are global-only (`~/.codex/skills` for Codex, `~/.claude/skills` for Claude).

This repo uses **generic skills** (no prefix). Other repos may use prefixed skills (e.g., `tp-*` for TexturePortal, `tr-*` for Triangle Rain) — ignore those when working in Intermediary.

## Skills Table

| Skill | USE when… |
|-------|-----------|
| architecture-first | Non-trivial changes or bugfixes (ADR-007) |
| typescript-native-rails | Edits to app/**/*.ts (ADR-005) |
| rust-runtime-rails | Work in src-tauri/** or crates/** (ADR-008/009) |
| tauri-security-baseline | Tauri security or asset/protocol changes (ADR-010) |
| docs-discipline | Documentation work (reports, architecture, roadmap, known_issues, inventories) |
| copy-safe-commands | Any response or doc that needs runnable commands (ADR-012) |
| review-lens | Reviewing diffs or PRs |
| workflow-closeout | Before landing any change (end-of-turn checks) |

## Rules

1. **Agent prompts must include relevant skill names in Constraints.**
   - Example: `Skills: typescript-native-rails, workflow-closeout`
2. **Agents must start every code-touching reply with `Skills: …`** listing skills used.
3. **Invoke proactively** — don't wait to be asked; match skills to task surface.

## Notes

- Generic skills are defined in `~/.codex/skills/` and `~/.claude/skills/`.
- Skills reference CLAUDE.md/AGENTS.md for repo-specific paths and conventions.

## See also

- ADR-006 section 3.5 for the skills mandate
- AGENTS.md / CLAUDE.md for the canonical skill table in agent context
