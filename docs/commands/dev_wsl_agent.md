# WSL Agent Development Commands
Updated on: 2026-02-12
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running the WSL agent during daily development.

## Requirements

The agent runs inside WSL and requires the Rust toolchain available in WSL.

## Start the Agent

Run from the repo root in WSL:

```bash
pnpm run agent:dev
```

`pnpm run agent:dev` uses `scripts/dev/run_wsl_agent_dev.sh`, which resolves
`INTERMEDIARY_WSL_WS_TOKEN` in this order:
1. Explicit `INTERMEDIARY_WSL_WS_TOKEN`
2. Explicit `INTERMEDIARY_WS_AUTH_FILE`
3. `wslWsToken` from the app-local `ws_auth.json` selected by `INTERMEDIARY_WS_AUTH_APP_ID`
4. Existing production, legacy, then dev app-local auth files under the active Windows `%LOCALAPPDATA%` profile (override lookup with `INTERMEDIARY_WINDOWS_LOCALAPPDATA` if needed)
5. Fallback `im_dev_wsl_token` (with warning; this typically means websocket auth will fail until token source is corrected)

When `INTERMEDIARY_WS_AUTH_APP_ID` is set and that app auth file does not exist yet, the launcher creates it before starting the WSL backend. The VS Code Windows dev task sets this to the app-local identity used by the Windows Tauri task so the backend and app share the same WSL token even when the backend task starts first.

If port `3142` is already listening but rejects the selected token, the launcher retires the listener only when `/proc` identifies it as an Intermediary `im_agent` for the same `INTERMEDIARY_AGENT_PORT`; unrelated listeners are left alone and the task fails.

When running alongside the app in Windows dev tasks, set
`INTERMEDIARY_WSL_BACKEND_MODE=external` in the app process so supervisor logic treats
this backend as externally managed.

## Log Output

The agent logs to the terminal and to:

`logs/agent_latest.log`

## Health Check

If the host agent shows WSL backend unavailable, confirm the backend is running and listening on:

- `ws://localhost:3142/?token=<wslWsToken-from-ws_auth.json>`
