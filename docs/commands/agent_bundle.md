# Agent Bundle Commands
Updated on: 2026-02-03
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for building the bundled WSL agent runtime that ships with the installer.

## Build the Bundle

1) Build the bundled Rust agent binary and refresh the Tauri resources bundle.

```bash
pnpm run agent:bundle
```

Run this inside WSL/Linux so the bundled binary matches the agent runtime environment.

Tauri packaging validates that `im_agent` is present; if the bundle is missing or version-mismatched the build fails with a pointer back to this doc.

## Output

Artifacts are written to:

`src-tauri/resources/agent_bundle/`

- `im_agent`
- `version.json`
