# WSL Agent Development Commands
Updated on: 2026-02-11
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
2. `wslWsToken` from app-local `ws_auth.json` under the active Windows `%LOCALAPPDATA%` profile (override path with `INTERMEDIARY_WS_AUTH_FILE` if needed)
3. Fallback `im_dev_wsl_token` (with warning)

## Log Output

The agent logs to the terminal and to:

`logs/agent_latest.log`

## Health Check

If the host agent shows WSL backend unavailable, confirm the backend is running and listening on:

- `ws://localhost:3142/?token=<wslWsToken-from-ws_auth.json>`
