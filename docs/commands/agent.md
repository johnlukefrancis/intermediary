# Agent Commands

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
wscat -c ws://localhost:3141
```

Send a clientHello to configure the agent:

```json
{
  "kind": "request",
  "requestId": "test-1",
  "payload": {
    "type": "clientHello",
    "repos": {
      "texture-portal": "/home/johnf/code/textureportal"
    },
    "stagingWslRoot": "/mnt/c/Users/johnf/AppData/Local/Intermediary/staging/files",
    "stagingWinRoot": "C:\\Users\\johnf\\AppData\\Local\\Intermediary\\staging\\files",
    "autoStageOnChange": true
  }
}
```

Request staging of a file:

```json
{
  "kind": "request",
  "requestId": "test-2",
  "payload": {
    "type": "stageFile",
    "repoId": "texture-portal",
    "path": "docs/readme.md"
  }
}
```
