# Verify the Integrated Terminal End-to-End
Updated on: 2026-09-04
Owners: JL · Agents
Depends on: ADR-012, ADR-013

Manual witness of the TERMINAL rail in the **installed** Windows app. The native adapter creates ConPTY
and the shell in one `CreateProcessW` call whose attribute list already contains the Job Object; the
clipboard and build-number reads are Windows-only too. Run this after touching
`src-tauri/src/lib/terminal/`, `src-tauri/src/lib/commands/terminal.rs`, `app/src/lib/terminal/`, or the
rail layout. Build and install first with `docs/commands/build_installer_from_wsl.md`.

Each step names what to do in the app and what proves it. The acceptance list in
`docs/design/terminal_design.md` is the oracle; this file is the route.

## 0. Native process-owner proof

After the WSL source has been synced to the Windows mirror, run the Windows-only test against the same
ConPTY adapter the app uses:

```powershell
Set-Location 'D:\code\intermediary'
cargo test -p intermediary child_belongs_to_the_job_at_create_process_return --lib
```

Expect `child_belongs_to_the_job_at_create_process_return ... ok`. The test creates a real ConPTY child,
queries the Job immediately when process creation returns, terminates and observes the Job empty, then
drains and joins the pseudoconsole.

## 1. Open tabs

In the app: pick a **host-rooted** repo tab, click the TERMINAL cell on the right rail. Expect the rail at
its usual width, `> STARTING PWSH` until the profile has loaded, one tab `PWSH 1` to open on its own, JL's
prompt and PSReadLine colours, and the working directory to be the repo folder. Drag the divider between
the columns: the rail resizes live, keeps the width after a restart, and double-click resets it:

```powershell
Get-Location
$PSVersionTable.PSVersion
```

Click `+` twice. Expect `PWSH 2` and `PWSH 3`, each a fresh shell in the same folder. Close `PWSH 2` with
its `×`; expect the neighbour to become active and the strip to read `PWSH 1`, `PWSH 3`.

## 2. WSL root entry and preflight (`wb-code` shape)

Switch to a **WSL-rooted** repo tab. Expect its first TERMINAL visit to open `PWSH 1`, run the profile,
and land in bash inside the repo. The open must have validated this exact path inside the exact selected
distro before the interactive process appeared. Prove the shell, distro, and directory:

```bash
printf '%s %s %s\n' "$WSL_DISTRO_NAME" "$SHELL" "$(pwd)"
```

Type `exit`; expect the pwsh prompt at the user profile directory. From there the alias must resolve:

```powershell
wb-code
```

If a configured native WSL repo has been moved or its selected distro is unavailable, visit TERMINAL and
expect `> PWSH FAILED TO START` naming both the Linux path and distro. There must be no PowerShell prompt
in the user profile and no interactive terminal registered for that failed open.

## 3. Interactive TUIs

In a host or WSL tab, start each and prove it renders, accepts the arrow keys and Ctrl+C, and reflows when
the window is resized (drag the window edge while the TUI is up):

```powershell
claude
```

```powershell
codex
```

Inside `claude`, type a line and press **Shift+Enter**: expect a newline inside the prompt, not a send.

## 4. Clipboard policy

Select text in the terminal with the mouse, press **Ctrl+Shift+C**, then paste it into the SOURCE commit
box: expect the same text. Copy a line from the commit box, click into the terminal, press **Ctrl+V** and
**Ctrl+Shift+V**: expect it typed at the prompt both times. Right-click with nothing selected: expect a
paste, no native menu. Select text and press **Ctrl+C**: expect a copy, not `^C`; with no selection expect
`^C` to interrupt a running `Start-Sleep 30`.

## 5. Rail, repo, and mode switches keep sessions

Start a long-lived process in a tab so survival is visible:

```powershell
1..600 | ForEach-Object { "tick $_"; Start-Sleep 1 }
```

Switch ZIPS → TERMINAL → SOURCE → TERMINAL, then repo A → B → A, then open and close a file from Auto
Files, then shrink the window under 860 px (handset) and back over 980 px. Expect the ticks to have kept
counting with no gap, every tab's scrollback intact, and the same tab still active per repo.

## 6. Exit, restart, close

Type `exit` in a tab. Expect `> PROCESS EXITED · CODE 0` over the kept scrollback. Click RESTART: a fresh
prompt in the same tab, old scrollback still above it. Type `exit` again, click CLOSE. Close every tab in
the repo: expect `> NO TERMINAL` with `+ NEW`, and no auto-open when you leave and return to TERMINAL.

## 7. Retained-tab bound

Open twelve tabs across any repo groups and type `exit` in each. Keep all twelve exited tabs. Expect every
tab and its scrollback to remain, while every `+` and `+ NEW` control is disabled with the twelve-session
limit in its tooltip. Close one exited tab: expect exactly one new tab to become admissible. A failed-open
tab counts the same way until closed; RESTART reuses its existing slot.

## 8. Flood, interrupt, and close-time detach

```powershell
Get-ChildItem -Recurse C:\Windows\System32 | Out-String
```

Expect the deck to stay responsive (rocker clicks land during the flood) and **Ctrl+C** to stop it within
a second. Once enough scrollback exists, move the pointer over the terminal's right edge. Expect a clearly
visible thumb on an elevated track, stronger hover feedback, and the accent colour while held. Drag the
thumb to the top, middle, and bottom; expect the viewport to follow throughout the drag and mouse-wheel
scrolling to remain unchanged. Repeat the flood and immediately close its tab, then open another tab. Expect close to finish,
the old output not to spill into the new tab or keep accumulating in the page, and the replacement open
to succeed as soon as the joined backend receipt releases the old slot.

## 9. Minimize during output

Start the same flood, minimize the window at once, wait ten seconds, restore. Expect the listing to have
finished (the prompt is back) rather than paused where it was when the window went away.

## 10. Tab close ends the console tree, not GUI children

In a tab, launch a GUI app from the shell, then note the tree:

```powershell
code .
```

From a separate Windows Terminal window, list the app's console-attached tree before closing the tab:

```powershell
Get-CimInstance Win32_Process | Where-Object { $_.Name -in 'pwsh.exe','wsl.exe','conhost.exe','OpenConsole.exe','wslhost.exe' } | Select-Object ProcessId, ParentProcessId, Name, CommandLine | Format-Table -AutoSize
```

Close the tab with `×`, run the listing again. Expect the tab's `pwsh.exe`, its `conhost.exe`, and any
`wsl.exe` it started to be gone, and the VS Code window to still be open. The Task Manager **Details**
tab (sorted by name) is the same check by eye.

## 11. App exit with an in-app `wsl` session (ADR-013 rule 4)

Open a WSL-rooted tab so a `wsl.exe` is live inside the app, leave no other WSL window open anywhere, and
close the app. Then read the supervisor log:

```powershell
Select-String -Path "$env:LOCALAPPDATA\com.johnf.intermediary\logs\run_latest.txt" -Pattern '\[terminal\]|wsl_exit_teardown' | Select-Object -Last 12
```

Expect the terminal scope's shutdown line (`shutdown_all`) with `still_alive=0` and
`receipt_errors=0` **before**
`wsl_exit_teardown` with `outcome=terminated … reason=idle`. An
`outcome=skipped reason=interactive_session_open` here means an app-owned session survived into the idle
probe, which is the defect this ordering exists to close.
Repeat with a `tmux` session started inside the tab and detached first: expect `reason=interactive_session_open`,
because a detached survivor correctly keeps the distro alive.

## 12. Persistence of the choice, not the sessions

Leave TERMINAL active, close and relaunch the app. Expect TERMINAL to be the active rail and one fresh
`PWSH 1` to auto-open; the previous session's scrollback is not restored (by design).
