# WSL Agent Development Commands
Updated on: 2026-01-30
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running the WSL agent during daily development.

## Start the Agent (Watch Mode)

Run from the repo root in WSL:

```bash
pnpm run agent:dev
```

## Log Output

The agent logs to the terminal and to:

`logs/agent_latest.log`

## Health Check

If the UI shows "Agent: Disconnected", confirm the agent is running and listening on:

- `ws://localhost:3141`
