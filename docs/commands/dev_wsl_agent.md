# WSL Agent Development Commands
Updated on: 2026-02-06
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running the WSL agent during daily development.

## Requirements

The agent runs inside WSL and requires the Rust toolchain available in WSL.

## Start the Agent

Run from the repo root in WSL:

```bash
INTERMEDIARY_AGENT_PORT=3142 INTERMEDIARY_WSL_WS_TOKEN=im_dev_wsl_token cargo run -p im_agent --bin im_agent
```

## Log Output

The agent logs to the terminal and to:

`logs/agent_latest.log`

## Health Check

If the host agent shows WSL backend unavailable, confirm the backend is running and listening on:

- `ws://localhost:3142/?token=im_dev_wsl_token`
