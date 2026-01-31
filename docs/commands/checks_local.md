// Path: docs/commands/checks_local.md
// Description: Local typecheck, lint, and Rust check commands for this repo.
# Local Checks
Updated on: 2026-01-31
Owners: JL · Agents
Depends on: ADR-008, ADR-012

Run the standard checks used for local validation.

## Commands

```bash
pnpm exec tsc --noEmit
pnpm exec eslint
cargo check
```

## Expected output
- TypeScript type check passes without errors.
- ESLint exits cleanly.
- Cargo check completes for the entire workspace (all crates).

## Package-specific checks

To check only the Tauri app:
```bash
cargo check -p intermediary
```

To check only the zip library:
```bash
cargo check -p im_zip
```
