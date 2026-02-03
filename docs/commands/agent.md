# Agent Commands
Updated on: 2026-02-03
Owners: JL · Agents
Depends on: ADR-000, ADR-012

Commands for running and developing the WSL agent.

## Development

Start the agent in watch mode (auto-restarts on file changes):

```bash
pnpm run agent:dev
```

## Type Check

Run TypeScript type checking on agent code:

```bash
pnpm run agent:typecheck
```

## Lint

Lint is included in the root lint command:

```bash
pnpm run lint
```

## Manual Testing

Connect to the running agent with wscat:

```bash
wscat -c ws://127.0.0.1:3141
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
      "agentPort": 3141,
      "autoStageGlobal": true,
      "repos": [
        {
          "repoId": "textureportal",
          "label": "TexturePortal",
          "wslPath": "/home/johnf/code/textureportal",
          "tabId": "texture-portal",
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
