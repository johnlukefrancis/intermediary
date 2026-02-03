# Agent Bundle Commands
Updated on: 2026-02-03
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for building the bundled WSL agent runtime and CLI assets that ship with the installer.

## Build the Bundle

1) Build the Rust bundle CLI first (required for `im_bundle_cli`).

```bash
cargo build -p im_bundle --bin im_bundle_cli --release
```

2) Build the bundled agent runtime and copy the CLI into the Tauri resources folder.

```bash
pnpm run agent:bundle
```

## Output

Artifacts are written to:

`src-tauri/resources/agent_bundle/`

- `agent_main.cjs`
- `im_bundle_cli`
- `version.json`
