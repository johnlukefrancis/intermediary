// Path: docs/commands/checks_local.md
// Description: Local typecheck, lint, and Rust check commands for this repo.
# Local Checks
Updated on: 2026-01-30
Owners: JL · Agents
Depends on: ADR-008, ADR-012

Run the standard checks used for local validation.

## Commands

```bash
pnpm exec tsc --noEmit
pnpm exec eslint
cd src-tauri
cargo check
```

## Expected output
- TypeScript type check passes without errors.
- ESLint exits cleanly.
- Cargo check completes for the Tauri crate.
