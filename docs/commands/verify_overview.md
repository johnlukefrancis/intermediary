# Verify Repository Overview
Updated on: 2026-10-12
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

## Crosstalk bubbles

With Crosstalk installed, remain in Overview while agents exchange a real message.
Confirm the sender's repository/group accent, the original purple Codex icon or
orange Claude Code mascot, faint sender tag, recipient repository and bottom-right
timestamp. There are no client-name labels or click hints. Hover beyond fourteen seconds to hold a
card; click its text to expand, click again to fold, and dismiss with the cross.
Inspect spring entrance and shrink/fade departure, plus reduced-motion opacity.

Let a message expire, then have its exact recipient answer within five minutes.
The previous text should softly reappear as a two-line echo above the reply,
preserving both agent icons and repository colours. Expand the echo and verify
the exchange stays open. A reply while the original is visible must not duplicate
it; another session with the same folder name must not inherit its context.
Send from two sessions in the same repository: the faint `#tag` after each
repository header must distinguish them, including the recalled-message header.

Move the main window to either side of the monitor; bubbles occupy the roomier
side. Maximize to put them inside. Minimize and switch to a repository: the surface
hides and clears. Return to Overview: expired/hidden messages do not replay. Check
dark, warm and light themes, three-message bursts and long expanded text. The
companion must never become the foreground window when a message arrives.
Focus a neighbouring Chrome window where a bubble will appear, with Chrome partly
covering Intermediary. The bubble must stay above Chrome through arrival and text
expansion while Chrome retains keyboard focus and Intermediary stays behind it.
Click Chrome: it now covers the bubble. Clicking a non-overlapping window must
leave an exposed bubble visible. A new message comes forward again, including a
same-height replacement in a full three-card stack. Resizing or expiring a card
must not bring a covered stack forward. Expiry, minimization and leaving Overview
must still remove the surface.

The feed reader and placement invariants run on the Windows build surface:

```bash
pwsh -NoProfile -Command 'Set-Location "E:\Code\intermediary"; cargo test -p intermediary crosstalk::'
```

Live delivery is the source witness. Explicitly labelled UI fixtures may supplement
it for bursts and appearance via the installed app's temporary WebView2 inspection
lane. Never append fixture messages to Crosstalk's real event log. Restore window
bounds, theme and navigation, then relaunch without debugging arguments.

## Git lane glow

After an agent's next real turn transition, inspect its configured root in the
Git column. A dirty lane breathes in its repo/group accent, including expanded
file names. Nested launch folders map to the most specific configured root;
siblings and roots sharing a folder name do not inherit the glow.

Use a clean configured root and two sessions: ending one must not show a finished
signal while the other works. When both are idle, a `CLEAN` row glows more strongly
and fades over 24 seconds; it disappears by the next five-second Overview age tick.
It does not increase `N CHANGED`. Repeated or older turn events must not restart
the fade. Leaving and returning to Overview preserves only its remaining time.
Held, disconnected, failed, truncated or mid-operation status cannot prove clean.
Check reduced motion: steady dirty highlight and gradual clean fade.

Keep fixtures in the installed renderer's temporary inspection lane, never in
the canonical Crosstalk event log. Restore the original view and native bounds.

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
