# Agent Bundle Commands
Updated on: 2026-02-04
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for building the bundled WSL agent runtime that ships with the installer.

## Build the Bundle

1) Build the bundled Rust agent binary and refresh the Tauri resources bundle.

```bash
bash ./scripts/build/build_agent_bundle.sh
```

Run this inside WSL/Linux so the bundled binary matches the agent runtime environment.
This script does not require Node inside WSL.

Tauri packaging validates that `im_agent` is present; if the bundle is missing or version-mismatched the build fails with a pointer back to this doc.

## Output

Artifacts are written to:

`src-tauri/resources/agent_bundle/`

- `im_agent`
- `version.json`
