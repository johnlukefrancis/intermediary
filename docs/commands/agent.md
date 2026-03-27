# Agent Commands
Updated on: 2026-02-12
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running and developing the WSL backend agent.

## Development

Start the agent in dev mode:

```bash
pnpm run agent:dev
```

Use `3142` when running as the backend under the host-agent model (`hostPort + 1`).
When `INTERMEDIARY_WSL_WS_TOKEN` is unset, the launcher resolves `wslWsToken`
from app-local `ws_auth.json` under the active Windows `%LOCALAPPDATA%` profile
before falling back to `im_dev_wsl_token`. You can pin lookup with
`INTERMEDIARY_WINDOWS_LOCALAPPDATA` or `INTERMEDIARY_WS_AUTH_FILE`.

If this dev backend is launched separately from the app, run the app with
`INTERMEDIARY_WSL_BACKEND_MODE=external` so supervisor stale-port remediation does
not attempt to terminate externally managed port `3142` occupants.
`INTERMEDIARY_WSL_BACKEND_MODE=managed` now enforces installed-backend ownership and
will reject external occupants even when websocket auth succeeds.

## Type Check

Run Rust checks for the agent:

```bash
cargo check -p im_agent
```

## Lint

Lint is included in the root lint command:

```bash
pnpm run lint
```

## Manual Testing

Connect to the running agent with wscat (optional):

```bash
wscat -c "ws://127.0.0.1:3142/?token=<wslWsToken-from-ws_auth.json>"
```

Send a clientHello to configure the agent:

```json
{
  "kind": "request",
  "requestId": "test-1",
  "payload": {
    "type": "clientHello",
    "config": {
      "agentHost": "127.0.0.1",
      "agentPort": 3142,
      "autoStageGlobal": true,
      "repos": [
        {
          "repoId": "example-repo",
          "label": "Example Repo",
          "root": { "kind": "wsl", "path": "/home/<you>/code/example-repo" },
          "autoStage": true,
          "docsGlobs": ["docs/**", "**/*.md", "**/*.mdx"],
          "codeGlobs": ["src/**", "**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
          "ignoreGlobs": ["**/node_modules/**", "**/.git/**", "**/dist/**", "**/build/**", "**/target/**"]
        }
      ]
    },
    "stagingWslRoot": "/mnt/c/Users/<you>/AppData/Local/Intermediary/staging",
    "stagingWinRoot": "C:\\Users\\<you>\\AppData\\Local\\Intermediary\\staging",
    "autoStageOnChange": true
  }
}
```

Staging roots point at the staging root. The agent stages files under `staging/files/<repoId>/...` and bundles under `staging/bundles/<repoId>/<presetId>/...`.

Request staging of a file:

```json
{
  "kind": "request",
  "requestId": "test-2",
  "payload": {
    "type": "stageFile",
    "repoId": "example-repo",
    "path": "docs/readme.md"
  }
}
```
