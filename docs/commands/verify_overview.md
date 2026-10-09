# Verify Repository Overview
Updated on: 2026-10-09
Owners: JL · Agents
Depends on: ADR-005, ADR-010, ADR-012

Build and install using [the Windows installer route](build_installer_from_wsl.md).
The app and both packaged agents must be 0.1.24 or newer because activity now
includes its configured root identity.

## Desktop witness

1. Select Overview beside the repository tabs. Confirm one chronological feed,
   labelled by repository, with no idle repository catalogue or right rail.
2. Edit files in two repositories. Confirm their labels, paths and previews enter
   the feed without visiting tabs. Repositories leave after fifteen quiet minutes.
   Recent history is labelled and never creates an invented preview.
3. Scroll up while work arrives: reading stays put and the N NEW pill returns to
   the live tail. Use a repository chip to focus it. Updates continue without a
   Pause control or a routine Live indicator.
4. Open a repository's Stream. Confirm the exact repository and Files/Stream surface
   opens from its header name, right arrow and keyboard, even in handset layout.
   Return to Overview; observations remain. Check arrows while scrolling and at 360px.
5. Restart the app on Overview. The destination persists; initial recent paths
   are History and do not invent live activity.
6. Inspect narrow and ordinary small windows and all three themes. Scroll the
   feed; check repository labels stay visible, keyboard focus and no overflow.
7. On a backend interruption, retained activity must be distinguished from a
   working watch. Host repos stay available through a WSL-only interruption.
8. Edit a supported image. Confirm its inline thumbnail and full-pane viewer on
   click; closing returns to Overview. Filename clicks open text files, right-click
   uses existing file actions, and explicit drags use the app's drag-out route.
   Images with the same relative path in different roots must never share pixels.
9. Check compact navigation, the repository-filter toolbar and the shared glass
   connecting/quiet/unavailable states. When no filters or issues are present,
   there is no empty toolbar. Inspect file viewing in handset layout.

## Automated corroboration

Run from the ext4 worktree:

```bash
pnpm exec tsc --noEmit
pnpm exec eslint
pnpm run test:ts
cargo test -p im_agent
cargo test -p im_host_agent
pnpm run version:check
```

The Windows product check uses the synced build surface:

```bash
pwsh -NoProfile -Command 'Set-Location "E:\Code\intermediary"; cargo check; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }; cargo test -p intermediary config::types::ui_state::tests'
```

Native inspection can attach to the app's own WebView2 DevTools. A process-local
debugging argument used for a witness must be removed from the final relaunch;
release CSP and Tauri capabilities remain unchanged.
