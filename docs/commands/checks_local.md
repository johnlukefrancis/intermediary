// Path: docs/commands/checks_local.md
// Description: Local typecheck, lint, and Rust check commands for this repo.
# Local Checks
Updated on: 2026-03-27
Owners: JL · Agents
Depends on: ADR-008, ADR-012

Run the standard checks used for local validation.

## Commands

```bash
pnpm run version:check
pnpm exec tsc --noEmit
pnpm exec eslint
cargo check
```

## Expected output
- Version contract is consistent across package, Tauri, crate, and bundled-agent version files.
- TypeScript type check passes without errors.
- ESLint exits cleanly.
- Cargo check completes for the entire workspace (all crates).

## Package-specific checks

To check only the Tauri app:
```bash
cargo check -p intermediary
```

To check only the bundle CLI crate:
```bash
cargo check -p im_bundle
```

To run bundle CLI crate unit tests:
```bash
cargo test -p im_bundle
```

For building and verifying the bundle CLI binary, see `docs/commands/bundle_cli.md`.
