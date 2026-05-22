# Kill Stale Agent Ports From Windows
Updated on: 2026-05-22
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Use this when the app reports that the WSL backend port is occupied by an
external process that rejected the current websocket token.

## Inspect port 3142

Run in Windows PowerShell:

```powershell
Get-NetTCPConnection -LocalPort 3142 -State Listen -ErrorAction SilentlyContinue |
  Select-Object LocalAddress,LocalPort,OwningProcess
```

## Identify the owning process

Replace `<PID>` with the `OwningProcess` value from the inspect step:

```powershell
Get-Process -Id <PID> | Format-List Id,ProcessName,Path
```

## Stop the stale listener

Only run this when the process is the stale Intermediary agent/backend you intend
to replace:

```powershell
Stop-Process -Id <PID> -Force
```

## Verify the port is clear

```powershell
Get-NetTCPConnection -LocalPort 3142 -State Listen -ErrorAction SilentlyContinue |
  Select-Object LocalAddress,LocalPort,OwningProcess
```

If this prints no listener, reopen Intermediary and let auto-start launch a fresh
agent.

## If PowerShell cannot see it

Run in a normal WSL terminal outside the sandbox:

```bash
lsof -nP -iTCP:3142 -sTCP:LISTEN
```

Then stop the shown process only if it is the stale Intermediary agent:

```bash
kill <PID>
```

If it does not exit after a few seconds:

```bash
kill -9 <PID>
```
