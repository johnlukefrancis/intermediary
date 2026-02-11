# Agent Commands
Updated on: 2026-02-06
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running and developing the WSL backend agent.

## Development

Start the agent in dev mode:

```bash
INTERMEDIARY_AGENT_PORT=3142 INTERMEDIARY_WSL_WS_TOKEN=im_dev_wsl_token cargo run -p im_agent --bin im_agent
```

Use `3142` when running as the backend under the host-agent model (`hostPort + 1`).

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
wscat -c "ws://127.0.0.1:3142/?token=im_dev_wsl_token"
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
          "repoId": "textureportal",
          "label": "TexturePortal",
          "root": { "kind": "wsl", "path": "/home/johnf/code/textureportal" },
          "autoStage": true,
          "docsGlobs": ["docs/**", "**/*.md", "**/*.mdx"],
          "codeGlobs": ["src/**", "**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"],
          "ignoreGlobs": ["**/node_modules/**", "**/.git/**", "**/dist/**", "**/build/**", "**/target/**"]
        }
      ]
    },
    "stagingWslRoot": "/mnt/c/Users/johnf/AppData/Local/Intermediary/staging",
    "stagingWinRoot": "C:\\Users\\johnf\\AppData\\Local\\Intermediary\\staging",
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
    "repoId": "textureportal",
    "path": "docs/readme.md"
  }
}
```
