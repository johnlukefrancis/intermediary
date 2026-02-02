# Workflow Closeout Commands
Updated on: 2026-02-02
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands to keep the repo in a shippable state after code or doc changes.

## Dependency sync

```bash
pnpm install
```

## File headers + ledger (when files are added/removed/moved)

```bash
pnpm run headers:list -- app
pnpm run headers:write -- app
pnpm run headers:list -- agent
pnpm run headers:write -- agent
pnpm run headers:list -- crates
pnpm run headers:write -- crates
pnpm run gen:ledger
```

## TypeScript checks

```bash
pnpm exec tsc --noEmit
pnpm exec eslint
```

## Rust checks

```bash
cargo check
```

This checks the entire workspace. For package-specific checks:
```bash
cargo check -p intermediary
```

## Rust tests (im_bundle)

```bash
cargo test -p im_bundle
```
