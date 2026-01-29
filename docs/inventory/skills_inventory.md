# TexturePortal Skills Inventory

Skills are workflow rails that agents (Codex/Claude) invoke proactively based on task surface. They are global-only (`~/.codex/skills` for Codex, `~/.claude/skills` for Claude).

## Skills Table

| Skill | USE when… |
|-------|-----------|
| tp-core-rails | ANY code work (app/src, src-tauri, crates, tp-ml, tp-ml-diffusers) |
| tp-architecture-first | Non-trivial changes or bugfixes (ADR-007) |
| tp-typescript-native-rails | Edits to app/src/**/*.ts (ADR-005) |
| tp-rust-runtime-rails | Work in src-tauri/** or crates/** (ADR-008/009) |
| tp-tauri-security-baseline | Tauri security or asset/protocol changes (ADR-010) |
| tp-ml-sidecar-protocol-rails | ML sidecar protocol work (Rust client, Python sidecars) |
| tp-docs-discipline | Documentation work (reports, architecture, roadmap, known_issues, inventories) |
| tp-copy-safe-commands | Any response or doc that needs runnable commands (ADR-012) |
| tp-review-lens | Reviewing diffs or PRs |
| tp-workflow-closeout | Before landing any change (end-of-turn checks) |

## Rules

1. **Agent prompts must include relevant skill names in Constraints.**
   - Example: `Skills: tp-typescript-native-rails, tp-workflow-closeout`
2. **Agents must start every code-touching reply with `Skills: …`** listing skills used.
3. **Invoke proactively** — don't wait to be asked; match skills to task surface.

## See also

- ADR-006 section 3.5 for the skills mandate
- AGENTS.md / CLAUDE.md for the canonical skill table in agent context
