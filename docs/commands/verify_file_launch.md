# Verify File Actions and Gallery Launch
Updated on: 2026-10-13
Owners: JL · Agents
Depends on: ADR-005, ADR-008, ADR-010, ADR-012

Build, silently install and reopen through [the Windows installer route](build_installer_from_wsl.md).

## Installed desktop witness

1. In a Windows repository's ZIPS tree, right-click a file and choose Open
   Containing Folder. Explorer should open its parent and select the file.
2. Repeat for a directory. Its parent opens with the directory selected.
3. Repeat from the file viewer and a WSL repository, including a path with
   spaces. Check the exact selected path in Explorer.
4. In the Blender authoring group, mainland's play action opens
   J:\BlenderAuthoring\blorb\mainland\index.html. Islands' action opens
   http://localhost:8788/repairs/ through the established server.
5. Restart the app. Both gallery choices persist. Other project launch targets
   retain their discovered files and intermediary has no launch action.
6. Restore the original repository/view and remove process-local inspection
   arguments before the final relaunch. Keep gallery servers and shared pages open.

## Blorb HTML demo

1. Select any Blorb worktree. The tab has the existing project play action and
   a small separate browser/play icon with no text, occupying a 20px button.
   Its tooltip identifies the HTML demo. Each worktree dropdown row has both too.
2. Click the demo icon. Normal MOMO play opens at
   http://127.0.0.1:8777/Docs/Reference/momo.html, served by the existing lab
   server. The URL must have no harness query arguments.
3. Check that the selected repository and the Unreal launch target did not change.
   Demo feedback must appear on its own button.
4. Restart. The group-wide demo binding survives, and existing authoring gallery
   choices still launch. Unconfigured groups do not gain a demo button.

The server must already be available. Intermediary opens the page and does not
start, stop or replace the server. Updating the saved demo binding handles a
future moved file/URL; automatic browser-history discovery is not implemented.

## Automated corroboration

From the ext4 source tree:

```bash
pnpm exec tsc --noEmit
pnpm exec eslint
pnpm run test:ts
```

From WSL, after syncing the Windows product tree:

```bash
pwsh -NoProfile -Command 'Set-Location "E:\Code\intermediary"; cargo check; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; cargo test -p intermediary launch::; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; cargo test -p intermediary config::types::tests'
```

An unreachable server is reported by the external browser. Candidate discovery
does not prove gallery contents; inspect the actual browser page.
