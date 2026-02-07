# Agent Bundle Commands
Updated on: 2026-02-06
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for preparing bundled host-agent runtime (all platforms) plus the WSL backend runtime (Windows builds).

## Build the Bundle

1) Build the bundled WSL backend binary and refresh the Tauri resources bundle.

```bash
bash ./scripts/build/build_agent_bundle.sh
```

Run this inside WSL/Linux when preparing a Windows build so the bundled WSL binary matches the runtime environment.
This script does not require Node inside WSL.
The script pins `TMPDIR` to the workspace `target/tmp` directory to avoid WSL cross-device temp-file failures during `cargo build --release`.

2) Build the host-agent binary for your host platform:

```powershell
cargo build -p im_host_agent --bin im_host_agent --release
```

3) Run the Tauri build as usual. The pre-build validation copies `target/release/im_host_agent(.exe)` into `src-tauri/resources/agent_bundle/` when needed and verifies bundle/version consistency.

## Output

Artifacts are written to:

`src-tauri/resources/agent_bundle/`

- `im_host_agent` (macOS/Linux) or `im_host_agent.exe` (Windows)
- `im_agent` (Windows builds that need WSL backend)
- `version.json`
