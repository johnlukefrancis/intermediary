# Agent Bundle Commands
Updated on: 2026-02-08
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

## macOS Signing/Notarization Note

When shipping macOS builds, treat `im_host_agent` as a bundled helper binary that must be part of the app-signing and notarization workflow.

- Include `src-tauri/resources/agent_bundle/im_host_agent` in signing inputs.
- Ensure notarization covers the final packaged app that contains this helper.
- If helper signing/notarization is skipped, runtime launch can fail with macOS permission errors even when local debug builds work.

## Troubleshooting: Gatekeeper/quarantine

At runtime, Intermediary now attempts to clear `com.apple.quarantine` on the installed host helper after copy + `chmod +x`.
This is best-effort and non-privileged. If the clear attempt fails, launch still proceeds and existing spawn diagnostics remain the source of truth.

If you still hit macOS `PermissionDenied` launch failures, remove quarantine manually from the installed helper:

```bash
xattr -d com.apple.quarantine "$HOME/Library/Application Support/intermediary/agent/im_host_agent"
```

Then relaunch the app and retry agent startup. If the error persists, treat signing/notarization coverage of the bundled helper as the next root cause.

## Output

Artifacts are written to:

`src-tauri/resources/agent_bundle/`

- `im_host_agent` (macOS/Linux) or `im_host_agent.exe` (Windows)
- `im_agent` (Windows builds that need WSL backend)
- `version.json`
