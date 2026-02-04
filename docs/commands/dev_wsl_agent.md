# WSL Agent Development Commands
Updated on: 2026-02-04
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running the WSL agent during daily development.

## Requirements

The agent runs inside WSL and requires the Rust toolchain available in WSL.

## Start the Agent

Run from the repo root in WSL:

```bash
cargo run -p im_agent --bin im_agent
```

## Log Output

The agent logs to the terminal and to:

`logs/agent_latest.log`

## Health Check

If the UI shows "Agent: Disconnected", confirm the agent is running and listening on:

- `ws://localhost:3141`
