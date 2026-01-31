// Path: docs/commands/bundle_cli.md
// Description: Build and verify the Rust bundle CLI binary.
# Bundle CLI Commands
Updated on: 2026-01-31
Owners: JL · Agents
Depends on: ADR-008, ADR-012

Commands to build and verify the Rust bundle CLI used by the agent.

## Build (release)

```bash
cargo build -p im_bundle --bin im_bundle_cli --release
```

## Build (dev)

```bash
cargo build -p im_bundle --bin im_bundle_cli
```

## Verify CLI binary path

```bash
ls -la target/release/im_bundle_cli
ls -la target/debug/im_bundle_cli
```
