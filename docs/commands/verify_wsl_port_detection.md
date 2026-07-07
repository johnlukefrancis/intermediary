# Verify WSL Port-Listener Detection End-to-End
Updated on: 2026-07-07
Owners: JL · Agents
Depends on: ADR-012, ADR-013

Manually exercises the real WSL detection code path (`list_wsl_agent_pids_by_port_listener`)
through an actual `wsl.exe`, confirming that scripts fed over stdin survive the Windows→WSL
boundary and that an `im_agent`-signed listener on the reserved port is detected. Run this after
touching WSL command execution or the port-listener detector.

## Start an im_agent-signed listener on a free port

Runs a throwaway listener carrying `INTERMEDIARY_WSL_WS_TOKEN` (the detector's confirmation
signal) on port `3199`:

```bash
INTERMEDIARY_WSL_WS_TOKEN=deadbeef INTERMEDIARY_AGENT_PORT=3199 python3 -m http.server 3199 &
```

## Run the ignored detection test against that port

```bash
INTERMEDIARY_TEST_PORT=3199 cargo test -p intermediary --lib \
  port_listener_detection_finds_live_agent_via_real_wsl -- --ignored --nocapture
```

Expect `detected im_agent pids = [<pid>]` and `test result: ok`. An empty result means the WSL
command boundary is corrupting output again (see ADR-013). Stop the listener with `kill %1` when
done.
