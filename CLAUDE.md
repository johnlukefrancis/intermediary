# CLAUDE.md — Intermediary

*Merges with ~/.claude/CLAUDE.md (nearest wins). Keep this file short; detailed rails live in docs/compliance/*.md.*

## Skills (USE PROACTIVELY)

Skills are global-only (`~/.claude/skills`). Invoke based on task surface—don't wait to be asked.

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

## Scope
- This repo uses **generic skills** (no prefix). Ignore `tr-*` / `tp-*` prefixed skills.
- Docs in `docs/compliance/` are the primary architectural contracts.
- If asked to write documentation, use docs-discipline (reads `docs/environment/docs_workflow.md` first).

## Must follow ADRs
- ADR-000 (Modular File Discipline)
- ADR-005 (TypeScript Native Contracts and Rails)
- ADR-007 (Architecture-First Execution)
- ADR-008 (Rust Runtime Contracts and Error Handling)
- ADR-009 (Rust Concurrency and IO Boundary Rules)
- ADR-010 (Tauri Security Baseline)
- ADR-012 (Copy-safe Command Delivery)

## Copy-safe command delivery (hardline)
- **No inline commands in agent responses.**
- All runnable commands must live in `docs/commands/**`.
- Responses must link the commands file path and describe what it does.

## Modular file discipline - ADR 0 (high signal)
- **snake_case** for files/folders (Rust `mod.rs`/`lib.rs`/`main.rs` are allowed).
- Prefer **tiny, single‑purpose modules**; target ≤250 LOC, hard cap 300 LOC (ADR‑000 is canonical).
- No monolith growth: split when adding new responsibilities.

## TypeScript/Rust contracts - ADR 5
- Do **not** weaken types to satisfy the compiler. Fix code or tighten types.
- Use `any`/`unknown` only at real dynamic seams with a TODO tag.

## Rust runtime rails - ADR 8/9/10
- No panics/unwraps across Tauri command boundaries; return `Result` with context.
- Required invariants use release‑enforced checks (no `debug_assert!` for runtime contracts).
- Handle EOF/disconnect as terminal; no spin loops in background workers.
- Long‑running CPU/IO work must be off the UI thread (`spawn_blocking` or owned thread).
- Cancellation is explicit and checked at stage boundaries.

## Architectural North Star - ADR 7
- No stop gaps, or band aid solutions that are not on a direct path to end state architecture.
- If existing structure prevents a correct solution, rewrite the structure.

## Workflow (Windows + WSL)
- Edit in WSL, run the app in Windows (synced via `scripts/windows/sync_to_windows.sh`).
- Use pnpm for frontend commands.
- If you add or change the location of a document, update `docs/guide.md`.

## File ledger (inventory)
- If adding/moving/deleting files under a scoped root (e.g., `src-tauri`, `app`, `scripts`): `npm run headers:list -- <scope>` then `npm run headers:write -- <scope>`.
- If files were added/removed/moved: `npm run gen:ledger` (no scope argument — always regenerates the full ledger from all roots).
- When creating new files, add the `// Path:` + `// Description:` header manually before running the ledger scripts.

## End of turn contract
- **First line of every reply:** `Skills: …` listing skills used this turn.
- List files touched (+ why).
- List acceptance checks actually run.
- Required: run `pnpm exec tsc --noEmit` and `pnpm exec eslint` at end of turn (when app code exists); report lint errors.
- Required: run `cargo check` at end of turn (when Rust code exists); fix any build errors cleanly under end state assumptions (ADR 7).
- ALWAYS Suggest a commit message when you make changes to any file, so that the user always has a unique commit message to use, for example: `emoji type(scope): summary`.
